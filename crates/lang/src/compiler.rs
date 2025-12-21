use crate::ast::*;
use crate::error::CompileError;

pub struct Compiler;

impl Compiler {
    pub fn compile_to_rust(ast: &[Statement]) -> Result<String, CompileError> {
        let mut output = String::new();
        output.push_str("// Auto-generated from MindLang\n");
        output.push_str("// This is a stub implementation\n\n");

        for stmt in ast {
            Self::compile_statement(&mut output, stmt, 0)?;
        }

        Ok(output)
    }

    fn compile_statement(
        output: &mut String,
        stmt: &Statement,
        indent: usize,
    ) -> Result<(), CompileError> {
        let prefix = "    ".repeat(indent);

        match stmt {
            Statement::Goal(goal) => {
                output.push_str(&format!("{}// Goal: {}\n", prefix, goal.name));
                output.push_str(&format!(
                    "{}pub async fn goal_{}() {{\n",
                    prefix,
                    Self::sanitize_name(&goal.name)
                ));
                for s in &goal.body {
                    Self::compile_statement(output, s, indent + 1)?;
                }
                output.push_str(&format!("{}}}\n\n", prefix));
            }
            Statement::On(handler) => {
                output.push_str(&format!(
                    "{}// Handler for: {}\n",
                    prefix, handler.event_pattern.kind
                ));
                output.push_str(&format!(
                    "{}pub async fn on_{}(event: Event) {{\n",
                    prefix,
                    Self::sanitize_name(&handler.event_pattern.kind)
                ));
                for s in &handler.body {
                    Self::compile_statement(output, s, indent + 1)?;
                }
                output.push_str(&format!("{}}}\n\n", prefix));
            }
            Statement::Emit(emit) => {
                output.push_str(&format!(
                    "{}emit_signal(\"{}\", payload).await;\n",
                    prefix, emit.signal
                ));
            }
            Statement::Store(store) => {
                let key = store.key.as_deref().unwrap_or("result");
                output.push_str(&format!("{}store(\"{}\", value).await;\n", prefix, key));
            }
            Statement::Use(use_expr) => {
                output.push_str(&format!(
                    "{}let agent = Agent::new::<{}>();\n",
                    prefix, use_expr.agent_type
                ));
            }
            Statement::If(_) => {
                output.push_str(&format!("{}// TODO: if expression\n", prefix));
            }
            Statement::Match(_) => {
                output.push_str(&format!("{}// TODO: match expression\n", prefix));
            }
            Statement::Block(stmts) => {
                output.push_str(&format!("{}{{\n", prefix));
                for s in stmts {
                    Self::compile_statement(output, s, indent + 1)?;
                }
                output.push_str(&format!("{}}}\n", prefix));
            }
        }

        Ok(())
    }

    fn sanitize_name(name: &str) -> String {
        name.chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect::<String>()
            .to_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    #[test]
    fn test_compile_goal() {
        let input = r#"goal "test" { emit "done"; }"#;
        let mut parser = Parser::new(input);
        let stmts = parser.parse().unwrap();
        let rust_code = Compiler::compile_to_rust(&stmts).unwrap();
        assert!(rust_code.contains("goal_test"));
        assert!(rust_code.contains("emit_signal"));
    }
}
