use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Statement {
    Goal(GoalDef),
    On(OnHandler),
    Emit(EmitExpr),
    Store(StoreExpr),
    Use(UseExpr),
    If(IfExpr),
    Match(MatchExpr),
    Block(Vec<Statement>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalDef {
    pub name: String,
    pub body: Vec<Statement>,
    pub on_success: Option<Box<Statement>>,
    pub on_failure: Option<Box<Statement>>,
    pub fallback: Option<Box<Statement>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnHandler {
    pub event_pattern: EventPattern,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitExpr {
    pub signal: String,
    pub payload: Option<Expr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreExpr {
    pub key: Option<String>,
    pub value: Option<Expr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UseExpr {
    pub agent_type: String,
    pub config: Option<Expr>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IfExpr {
    pub condition: Expr,
    pub then_branch: Vec<Statement>,
    pub else_branch: Option<Vec<Statement>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchExpr {
    pub value: Expr,
    pub arms: Vec<MatchArm>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchArm {
    pub pattern: Expr,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Expr {
    String(String),
    Number(f64),
    Bool(bool),
    Ident(String),
    Call {
        func: String,
        args: Vec<Expr>,
    },
    Object(Vec<(String, Expr)>),
    Array(Vec<Expr>),
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPattern {
    pub kind: String,
    pub filters: Vec<Filter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub field: String,
    pub op: BinaryOp,
    pub value: Expr,
}
