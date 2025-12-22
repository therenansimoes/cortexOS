use std::collections::HashMap;
use std::pin::Pin;
use std::future::Future;

use crate::ast::*;
use crate::error::VMError;

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Object(HashMap<String, Value>),
    Array(Vec<Value>),
}

impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}

impl From<&Expr> for Value {
    fn from(expr: &Expr) -> Self {
        match expr {
            Expr::String(s) => Value::String(s.clone()),
            Expr::Number(n) => Value::Number(*n),
            Expr::Bool(b) => Value::Bool(*b),
            Expr::Array(arr) => Value::Array(arr.iter().map(Value::from).collect()),
            Expr::Object(pairs) => {
                Value::Object(pairs.iter().map(|(k, v)| (k.clone(), Value::from(v))).collect())
            }
            _ => Value::Null,
        }
    }
}

pub struct VMContext {
    pub variables: HashMap<String, Value>,
    pub signals: Vec<(String, Option<Value>)>,
    pub stored: HashMap<String, Value>,
    pub reward: f64,
}

impl Default for VMContext {
    fn default() -> Self {
        Self::new()
    }
}

impl VMContext {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            signals: Vec::new(),
            stored: HashMap::new(),
            reward: 0.0,
        }
    }
}

pub struct VM {
    _stack: Vec<Value>,
    context: VMContext,
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

impl VM {
    pub fn new() -> Self {
        Self {
            _stack: Vec::new(),
            context: VMContext::new(),
        }
    }

    pub fn with_context(context: VMContext) -> Self {
        Self {
            _stack: Vec::new(),
            context,
        }
    }

    pub fn context(&self) -> &VMContext {
        &self.context
    }

    pub fn context_mut(&mut self) -> &mut VMContext {
        &mut self.context
    }

    pub fn execute<'a>(
        &'a mut self,
        statements: &'a [Statement],
    ) -> Pin<Box<dyn Future<Output = Result<Value, VMError>> + Send + 'a>>
    where
        Self: Send,
    {
        Box::pin(async move {
            let mut last_value = Value::Null;
            for stmt in statements {
                last_value = self.execute_statement(stmt).await?;
            }
            Ok(last_value)
        })
    }

    fn execute_statement<'a>(
        &'a mut self,
        stmt: &'a Statement,
    ) -> Pin<Box<dyn Future<Output = Result<Value, VMError>> + Send + 'a>>
    where
        Self: Send,
    {
        Box::pin(async move {
            match stmt {
                Statement::Goal(goal) => self.execute_goal(goal).await,
                Statement::On(handler) => self.execute_on_handler(handler).await,
                Statement::Emit(emit) => self.execute_emit(emit).await,
                Statement::Store(store) => self.execute_store(store).await,
                Statement::Use(use_expr) => self.execute_use(use_expr).await,
                Statement::If(if_expr) => self.execute_if(if_expr).await,
                Statement::Match(match_expr) => self.execute_match(match_expr).await,
                Statement::Block(stmts) => self.execute(stmts).await,
            }
        })
    }

    pub fn execute_goal<'a>(
        &'a mut self,
        goal: &'a GoalDef,
    ) -> Pin<Box<dyn Future<Output = Result<Value, VMError>> + Send + 'a>>
    where
        Self: Send,
    {
        Box::pin(async move {
            tracing::info!("Executing goal: {}", goal.name);

            let result = self.execute(&goal.body).await;

            match result {
                Ok(value) => {
                    if let Some(on_success) = &goal.on_success {
                        self.execute_statement(on_success).await?;
                    }
                    Ok(value)
                }
                Err(e) => {
                    if let Some(on_failure) = &goal.on_failure {
                        self.execute_statement(on_failure).await?;
                    }
                    if let Some(fallback) = &goal.fallback {
                        return self.execute_statement(fallback).await;
                    }
                    Err(e)
                }
            }
        })
    }

    async fn execute_on_handler(&mut self, handler: &OnHandler) -> Result<Value, VMError> {
        tracing::debug!(
            "Registered handler for event: {}",
            handler.event_pattern.kind
        );
        Ok(Value::Null)
    }

    async fn execute_emit(&mut self, emit: &EmitExpr) -> Result<Value, VMError> {
        let payload = emit.payload.as_ref().map(Value::from);
        tracing::debug!("Emitting signal: {}", emit.signal);
        self.context.signals.push((emit.signal.clone(), payload));
        Ok(Value::Null)
    }

    async fn execute_store(&mut self, store: &StoreExpr) -> Result<Value, VMError> {
        let key = store.key.clone().unwrap_or_else(|| "result".to_string());
        let value = store
            .value
            .as_ref()
            .map(Value::from)
            .unwrap_or(Value::Null);
        self.context.stored.insert(key, value);
        Ok(Value::Null)
    }

    async fn execute_use(&mut self, use_expr: &UseExpr) -> Result<Value, VMError> {
        tracing::debug!("Using agent: {}", use_expr.agent_type);
        Ok(Value::Null)
    }

    async fn execute_if(&mut self, if_expr: &IfExpr) -> Result<Value, VMError> {
        let condition = self.eval_expr(&if_expr.condition)?;
        let is_truthy = match condition {
            Value::Bool(b) => b,
            Value::Null => false,
            Value::Number(n) => n != 0.0,
            Value::String(s) => !s.is_empty(),
            _ => true,
        };

        if is_truthy {
            self.execute(&if_expr.then_branch).await
        } else if let Some(else_branch) = &if_expr.else_branch {
            self.execute(else_branch).await
        } else {
            Ok(Value::Null)
        }
    }

    async fn execute_match(&mut self, match_expr: &MatchExpr) -> Result<Value, VMError> {
        let value = self.eval_expr(&match_expr.value)?;

        for arm in &match_expr.arms {
            let pattern = self.eval_expr(&arm.pattern)?;
            if self.values_equal(&value, &pattern) {
                return self.execute(&arm.body).await;
            }
        }

        Ok(Value::Null)
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value, VMError> {
        match expr {
            Expr::String(s) => Ok(Value::String(s.clone())),
            Expr::Number(n) => Ok(Value::Number(*n)),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Ident(name) => self
                .context
                .variables
                .get(name)
                .cloned()
                .ok_or_else(|| VMError::UndefinedVariable(name.clone())),
            Expr::Call { func, args } => self.eval_call(func, args),
            Expr::Object(pairs) => {
                let mut map = HashMap::new();
                for (k, v) in pairs {
                    map.insert(k.clone(), self.eval_expr(v)?);
                }
                Ok(Value::Object(map))
            }
            Expr::Array(items) => {
                let values: Result<Vec<_>, _> = items.iter().map(|e| self.eval_expr(e)).collect();
                Ok(Value::Array(values?))
            }
            Expr::Binary { left, op, right } => {
                let l = self.eval_expr(left)?;
                let r = self.eval_expr(right)?;
                self.eval_binary_op(&l, op, &r)
            }
        }
    }

    fn eval_call(&mut self, func: &str, args: &[Expr]) -> Result<Value, VMError> {
        match func {
            "adjust_reward" => {
                if let Some(Expr::Number(n)) = args.first() {
                    self.context.reward += n;
                }
                Ok(Value::Null)
            }
            "request_help" => {
                if let Some(Expr::String(target)) = args.first() {
                    tracing::info!("Requesting help from: {}", target);
                }
                Ok(Value::Null)
            }
            _ => {
                tracing::warn!("Unknown function: {}", func);
                Ok(Value::Null)
            }
        }
    }

    fn eval_binary_op(&self, left: &Value, op: &BinaryOp, right: &Value) -> Result<Value, VMError> {
        match (left, op, right) {
            (Value::Number(l), BinaryOp::Add, Value::Number(r)) => Ok(Value::Number(l + r)),
            (Value::Number(l), BinaryOp::Sub, Value::Number(r)) => Ok(Value::Number(l - r)),
            (Value::Number(l), BinaryOp::Mul, Value::Number(r)) => Ok(Value::Number(l * r)),
            (Value::Number(l), BinaryOp::Div, Value::Number(r)) => {
                if *r == 0.0 {
                    Err(VMError::RuntimeError("Division by zero".to_string()))
                } else {
                    Ok(Value::Number(l / r))
                }
            }
            (Value::Number(l), BinaryOp::Lt, Value::Number(r)) => Ok(Value::Bool(l < r)),
            (Value::Number(l), BinaryOp::Gt, Value::Number(r)) => Ok(Value::Bool(l > r)),
            (Value::Number(l), BinaryOp::Le, Value::Number(r)) => Ok(Value::Bool(l <= r)),
            (Value::Number(l), BinaryOp::Ge, Value::Number(r)) => Ok(Value::Bool(l >= r)),
            (l, BinaryOp::Eq, r) => Ok(Value::Bool(self.values_equal(l, r))),
            (l, BinaryOp::Ne, r) => Ok(Value::Bool(!self.values_equal(l, r))),
            (Value::Bool(l), BinaryOp::And, Value::Bool(r)) => Ok(Value::Bool(*l && *r)),
            (Value::Bool(l), BinaryOp::Or, Value::Bool(r)) => Ok(Value::Bool(*l || *r)),
            (Value::String(l), BinaryOp::Add, Value::String(r)) => {
                Ok(Value::String(format!("{}{}", l, r)))
            }
            _ => Err(VMError::TypeError(format!(
                "Cannot apply {:?} to {:?} and {:?}",
                op, left, right
            ))),
        }
    }

    fn values_equal(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => (a - b).abs() < f64::EPSILON,
            (Value::String(a), Value::String(b)) => a == b,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    #[tokio::test]
    async fn test_vm_emit() {
        let input = r#"emit "test_signal";"#;
        let mut parser = Parser::new(input);
        let stmts = parser.parse().unwrap();

        let mut vm = VM::new();
        vm.execute(&stmts).await.unwrap();

        assert_eq!(vm.context.signals.len(), 1);
        assert_eq!(vm.context.signals[0].0, "test_signal");
    }
}
