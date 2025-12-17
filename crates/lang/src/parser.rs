use crate::ast::*;
use crate::error::ParseError;
use crate::lexer::{Lexer, Token};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(input: &str) -> Self {
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap_or_else(|_| vec![Token::Eof]);
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&Token::Eof)
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens.get(self.pos).cloned().unwrap_or(Token::Eof);
        self.pos += 1;
        token
    }

    fn expect(&mut self, expected: Token) -> Result<(), ParseError> {
        let token = self.advance();
        if std::mem::discriminant(&token) == std::mem::discriminant(&expected) {
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                expected: format!("{:?}", expected),
                found: format!("{:?}", token),
            })
        }
    }

    fn expect_string(&mut self) -> Result<String, ParseError> {
        match self.advance() {
            Token::String(s) => Ok(s),
            t => Err(ParseError::UnexpectedToken {
                expected: "String".to_string(),
                found: format!("{:?}", t),
            }),
        }
    }

    fn expect_ident(&mut self) -> Result<String, ParseError> {
        match self.advance() {
            Token::Ident(s) => Ok(s),
            t => Err(ParseError::UnexpectedToken {
                expected: "Identifier".to_string(),
                found: format!("{:?}", t),
            }),
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Statement>, ParseError> {
        let mut statements = Vec::new();
        while !matches!(self.peek(), Token::Eof) {
            statements.push(self.parse_statement()?);
        }
        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match self.peek() {
            Token::Goal => self.parse_goal().map(Statement::Goal),
            Token::On => self.parse_on_handler().map(Statement::On),
            Token::Emit => self.parse_emit().map(Statement::Emit),
            Token::Store => self.parse_store().map(Statement::Store),
            Token::Use => self.parse_use().map(Statement::Use),
            Token::If => self.parse_if().map(Statement::If),
            Token::Match => self.parse_match().map(Statement::Match),
            Token::LBrace => self.parse_block().map(Statement::Block),
            Token::Ident(name) => {
                let name = name.clone();
                self.advance();
                if matches!(self.peek(), Token::LParen) {
                    self.parse_call_statement(name)
                } else {
                    Err(ParseError::InvalidExpression)
                }
            }
            _ => Err(ParseError::InvalidExpression),
        }
    }

    pub fn parse_goal(&mut self) -> Result<GoalDef, ParseError> {
        self.expect(Token::Goal)?;
        let name = self.expect_string()?;
        self.expect(Token::LBrace)?;

        let mut body = Vec::new();
        let mut on_success = None;
        let mut on_failure = None;
        let mut fallback = None;

        while !matches!(self.peek(), Token::RBrace | Token::Eof) {
            match self.peek() {
                Token::OnSuccess => {
                    self.advance();
                    on_success = Some(Box::new(self.parse_statement()?));
                }
                Token::OnFailure => {
                    self.advance();
                    on_failure = Some(Box::new(self.parse_statement()?));
                }
                Token::Fallback => {
                    self.advance();
                    fallback = Some(Box::new(self.parse_statement()?));
                }
                _ => {
                    body.push(self.parse_statement()?);
                }
            }
        }

        self.expect(Token::RBrace)?;

        Ok(GoalDef {
            name,
            body,
            on_success,
            on_failure,
            fallback,
        })
    }

    fn parse_on_handler(&mut self) -> Result<OnHandler, ParseError> {
        self.expect(Token::On)?;
        let kind = self.expect_string()?;

        let mut filters = Vec::new();
        if matches!(self.peek(), Token::Where) {
            self.advance();
            filters = self.parse_filters()?;
        }

        self.expect(Token::LBrace)?;
        let body = self.parse_statements_until_rbrace()?;
        self.expect(Token::RBrace)?;

        Ok(OnHandler {
            event_pattern: EventPattern { kind, filters },
            body,
        })
    }

    fn parse_filters(&mut self) -> Result<Vec<Filter>, ParseError> {
        let mut filters = Vec::new();
        loop {
            let field = self.expect_ident()?;
            let op = self.parse_binary_op()?;
            let value = self.parse_expr()?;
            filters.push(Filter { field, op, value });

            if !matches!(self.peek(), Token::And) {
                break;
            }
            self.advance();
        }
        Ok(filters)
    }

    fn parse_binary_op(&mut self) -> Result<BinaryOp, ParseError> {
        let op = match self.advance() {
            Token::EqEq => BinaryOp::Eq,
            Token::Ne => BinaryOp::Ne,
            Token::Lt => BinaryOp::Lt,
            Token::Gt => BinaryOp::Gt,
            Token::Le => BinaryOp::Le,
            Token::Ge => BinaryOp::Ge,
            Token::Plus => BinaryOp::Add,
            Token::Minus => BinaryOp::Sub,
            Token::Star => BinaryOp::Mul,
            Token::Slash => BinaryOp::Div,
            t => {
                return Err(ParseError::UnexpectedToken {
                    expected: "binary operator".to_string(),
                    found: format!("{:?}", t),
                })
            }
        };
        Ok(op)
    }

    fn parse_emit(&mut self) -> Result<EmitExpr, ParseError> {
        self.expect(Token::Emit)?;
        let signal = self.expect_string()?;

        let payload = if matches!(self.peek(), Token::Comma) {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };

        if matches!(self.peek(), Token::Semicolon) {
            self.advance();
        }

        Ok(EmitExpr { signal, payload })
    }

    fn parse_store(&mut self) -> Result<StoreExpr, ParseError> {
        self.expect(Token::Store)?;

        if matches!(self.peek(), Token::Semicolon) {
            self.advance();
            return Ok(StoreExpr {
                key: None,
                value: None,
            });
        }

        let key = if matches!(self.peek(), Token::String(_)) {
            Some(self.expect_string()?)
        } else {
            None
        };

        let value = if matches!(self.peek(), Token::Comma) {
            self.advance();
            Some(self.parse_expr()?)
        } else {
            None
        };

        if matches!(self.peek(), Token::Semicolon) {
            self.advance();
        }

        Ok(StoreExpr { key, value })
    }

    fn parse_use(&mut self) -> Result<UseExpr, ParseError> {
        self.expect(Token::Use)?;

        if matches!(self.peek(), Token::Agent) {
            self.advance();
        }

        let agent_type = self.expect_string()?;

        let config = if matches!(self.peek(), Token::Comma | Token::LBrace) {
            if matches!(self.peek(), Token::Comma) {
                self.advance();
            }
            Some(self.parse_expr()?)
        } else {
            None
        };

        if matches!(self.peek(), Token::Semicolon) {
            self.advance();
        }

        Ok(UseExpr { agent_type, config })
    }

    fn parse_if(&mut self) -> Result<IfExpr, ParseError> {
        self.expect(Token::If)?;
        let condition = self.parse_expr()?;
        self.expect(Token::LBrace)?;
        let then_branch = self.parse_statements_until_rbrace()?;
        self.expect(Token::RBrace)?;

        let else_branch = if matches!(self.peek(), Token::Else) {
            self.advance();
            self.expect(Token::LBrace)?;
            let stmts = self.parse_statements_until_rbrace()?;
            self.expect(Token::RBrace)?;
            Some(stmts)
        } else {
            None
        };

        Ok(IfExpr {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn parse_match(&mut self) -> Result<MatchExpr, ParseError> {
        self.expect(Token::Match)?;
        let value = self.parse_expr()?;
        self.expect(Token::LBrace)?;

        let mut arms = Vec::new();
        while !matches!(self.peek(), Token::RBrace | Token::Eof) {
            let pattern = self.parse_expr()?;
            self.expect(Token::Arrow)?;
            self.expect(Token::LBrace)?;
            let body = self.parse_statements_until_rbrace()?;
            self.expect(Token::RBrace)?;
            arms.push(MatchArm { pattern, body });
        }

        self.expect(Token::RBrace)?;
        Ok(MatchExpr { value, arms })
    }

    fn parse_block(&mut self) -> Result<Vec<Statement>, ParseError> {
        self.expect(Token::LBrace)?;
        let stmts = self.parse_statements_until_rbrace()?;
        self.expect(Token::RBrace)?;
        Ok(stmts)
    }

    fn parse_statements_until_rbrace(&mut self) -> Result<Vec<Statement>, ParseError> {
        let mut statements = Vec::new();
        while !matches!(self.peek(), Token::RBrace | Token::Eof) {
            statements.push(self.parse_statement()?);
        }
        Ok(statements)
    }

    fn parse_call_statement(&mut self, func: String) -> Result<Statement, ParseError> {
        self.expect(Token::LParen)?;
        let mut args = Vec::new();
        while !matches!(self.peek(), Token::RParen | Token::Eof) {
            args.push(self.parse_expr()?);
            if matches!(self.peek(), Token::Comma) {
                self.advance();
            }
        }
        self.expect(Token::RParen)?;
        if matches!(self.peek(), Token::Semicolon) {
            self.advance();
        }

        Ok(Statement::Emit(EmitExpr {
            signal: func,
            payload: if args.is_empty() {
                None
            } else {
                Some(Expr::Array(args))
            },
        }))
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_or_expr()
    }

    fn parse_or_expr(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and_expr()?;
        while matches!(self.peek(), Token::Or) {
            self.advance();
            let right = self.parse_and_expr()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Or,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_and_expr(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_comparison_expr()?;
        while matches!(self.peek(), Token::And) {
            self.advance();
            let right = self.parse_comparison_expr()?;
            left = Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::And,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_comparison_expr(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_additive_expr()?;
        while matches!(
            self.peek(),
            Token::EqEq | Token::Ne | Token::Lt | Token::Gt | Token::Le | Token::Ge
        ) {
            let op = match self.advance() {
                Token::EqEq => BinaryOp::Eq,
                Token::Ne => BinaryOp::Ne,
                Token::Lt => BinaryOp::Lt,
                Token::Gt => BinaryOp::Gt,
                Token::Le => BinaryOp::Le,
                Token::Ge => BinaryOp::Ge,
                _ => unreachable!(),
            };
            let right = self.parse_additive_expr()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_additive_expr(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_multiplicative_expr()?;
        while matches!(self.peek(), Token::Plus | Token::Minus) {
            let op = match self.advance() {
                Token::Plus => BinaryOp::Add,
                Token::Minus => BinaryOp::Sub,
                _ => unreachable!(),
            };
            let right = self.parse_multiplicative_expr()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_multiplicative_expr(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_primary()?;
        while matches!(self.peek(), Token::Star | Token::Slash) {
            let op = match self.advance() {
                Token::Star => BinaryOp::Mul,
                Token::Slash => BinaryOp::Div,
                _ => unreachable!(),
            };
            let right = self.parse_primary()?;
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        match self.peek().clone() {
            Token::String(s) => {
                self.advance();
                Ok(Expr::String(s))
            }
            Token::Number(n) => {
                self.advance();
                Ok(Expr::Number(n))
            }
            Token::True => {
                self.advance();
                Ok(Expr::Bool(true))
            }
            Token::False => {
                self.advance();
                Ok(Expr::Bool(false))
            }
            Token::Ident(name) => {
                self.advance();
                if matches!(self.peek(), Token::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    while !matches!(self.peek(), Token::RParen | Token::Eof) {
                        args.push(self.parse_expr()?);
                        if matches!(self.peek(), Token::Comma) {
                            self.advance();
                        }
                    }
                    self.expect(Token::RParen)?;
                    Ok(Expr::Call { func: name, args })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            Token::LBrace => {
                self.advance();
                let mut pairs = Vec::new();
                while !matches!(self.peek(), Token::RBrace | Token::Eof) {
                    let key = match self.advance() {
                        Token::Ident(s) | Token::String(s) => s,
                        t => {
                            return Err(ParseError::UnexpectedToken {
                                expected: "key".to_string(),
                                found: format!("{:?}", t),
                            })
                        }
                    };
                    self.expect(Token::Colon)?;
                    let value = self.parse_expr()?;
                    pairs.push((key, value));
                    if matches!(self.peek(), Token::Comma) {
                        self.advance();
                    }
                }
                self.expect(Token::RBrace)?;
                Ok(Expr::Object(pairs))
            }
            Token::LBracket => {
                self.advance();
                let mut items = Vec::new();
                while !matches!(self.peek(), Token::RBracket | Token::Eof) {
                    items.push(self.parse_expr()?);
                    if matches!(self.peek(), Token::Comma) {
                        self.advance();
                    }
                }
                self.expect(Token::RBracket)?;
                Ok(Expr::Array(items))
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            t => Err(ParseError::UnexpectedToken {
                expected: "expression".to_string(),
                found: format!("{:?}", t),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_goal() {
        let input = r#"
            goal "implement HTTP server" {
                use agent "Compiler";
                on_success { emit "task_complete"; }
            }
        "#;
        let mut parser = Parser::new(input);
        let stmts = parser.parse().unwrap();
        assert_eq!(stmts.len(), 1);
        if let Statement::Goal(goal) = &stmts[0] {
            assert_eq!(goal.name, "implement HTTP server");
            assert!(goal.on_success.is_some());
        } else {
            panic!("Expected Goal statement");
        }
    }

    #[test]
    fn test_parse_on_handler() {
        let input = r#"
            on "sensor.mic.v1" where volume > 0.8 {
                emit "loud_sound_detected";
            }
        "#;
        let mut parser = Parser::new(input);
        let stmts = parser.parse().unwrap();
        assert_eq!(stmts.len(), 1);
        if let Statement::On(handler) = &stmts[0] {
            assert_eq!(handler.event_pattern.kind, "sensor.mic.v1");
            assert_eq!(handler.event_pattern.filters.len(), 1);
        } else {
            panic!("Expected On statement");
        }
    }
}
