use crate::error::LexError;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Goal,
    On,
    Emit,
    Store,
    Use,
    If,
    Else,
    Match,
    Fallback,
    OnSuccess,
    OnFailure,
    Where,
    Agent,
    And,
    Or,
    True,
    False,

    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Semicolon,
    Arrow,
    Eq,
    EqEq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    Plus,
    Minus,
    Star,
    Slash,
    Dot,

    String(String),
    Number(f64),
    Ident(String),
    Eof,
}

pub struct Lexer {
    input: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.input.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.input.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.peek();
        self.pos += 1;
        ch
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else if ch == '/' && self.peek_next() == Some('/') {
                while let Some(c) = self.peek() {
                    if c == '\n' {
                        break;
                    }
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    fn read_string(&mut self) -> Result<String, LexError> {
        let start = self.pos;
        self.advance(); // consume opening quote
        let mut s = String::new();

        loop {
            match self.peek() {
                Some('"') => {
                    self.advance();
                    return Ok(s);
                }
                Some('\\') => {
                    self.advance();
                    match self.peek() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('r') => s.push('\r'),
                        Some('\\') => s.push('\\'),
                        Some('"') => s.push('"'),
                        _ => {}
                    }
                    self.advance();
                }
                Some(ch) => {
                    s.push(ch);
                    self.advance();
                }
                None => return Err(LexError::UnterminatedString(start)),
            }
        }
    }

    fn read_number(&mut self) -> Result<f64, LexError> {
        let start = self.pos;
        let mut s = String::new();

        if self.peek() == Some('-') {
            s.push('-');
            self.advance();
        }

        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() || ch == '.' {
                s.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        s.parse().map_err(|_| LexError::InvalidNumber(start))
    }

    fn read_ident(&mut self) -> String {
        let mut s = String::new();
        while let Some(ch) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' || ch == '.' {
                s.push(ch);
                self.advance();
            } else {
                break;
            }
        }
        s
    }

    pub fn next_token(&mut self) -> Result<Token, LexError> {
        self.skip_whitespace();

        let Some(ch) = self.peek() else {
            return Ok(Token::Eof);
        };

        let token = match ch {
            '{' => {
                self.advance();
                Token::LBrace
            }
            '}' => {
                self.advance();
                Token::RBrace
            }
            '(' => {
                self.advance();
                Token::LParen
            }
            ')' => {
                self.advance();
                Token::RParen
            }
            '[' => {
                self.advance();
                Token::LBracket
            }
            ']' => {
                self.advance();
                Token::RBracket
            }
            ',' => {
                self.advance();
                Token::Comma
            }
            ':' => {
                self.advance();
                Token::Colon
            }
            ';' => {
                self.advance();
                Token::Semicolon
            }
            '.' => {
                self.advance();
                Token::Dot
            }
            '+' => {
                self.advance();
                Token::Plus
            }
            '*' => {
                self.advance();
                Token::Star
            }
            '/' => {
                self.advance();
                Token::Slash
            }
            '-' => {
                self.advance();
                if self.peek() == Some('>') {
                    self.advance();
                    Token::Arrow
                } else if self.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    self.pos -= 1;
                    Token::Number(self.read_number()?)
                } else {
                    Token::Minus
                }
            }
            '=' => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Token::EqEq
                } else {
                    Token::Eq
                }
            }
            '!' => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Token::Ne
                } else {
                    return Err(LexError::UnexpectedChar('!', self.pos - 1));
                }
            }
            '<' => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Token::Le
                } else {
                    Token::Lt
                }
            }
            '>' => {
                self.advance();
                if self.peek() == Some('=') {
                    self.advance();
                    Token::Ge
                } else {
                    Token::Gt
                }
            }
            '"' => Token::String(self.read_string()?),
            c if c.is_ascii_digit() => Token::Number(self.read_number()?),
            c if c.is_alphabetic() || c == '_' => {
                let ident = self.read_ident();
                match ident.as_str() {
                    "goal" => Token::Goal,
                    "on" => Token::On,
                    "emit" => Token::Emit,
                    "store" | "store_result" => Token::Store,
                    "use" => Token::Use,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "match" => Token::Match,
                    "fallback" => Token::Fallback,
                    "on_success" => Token::OnSuccess,
                    "on_failure" => Token::OnFailure,
                    "where" => Token::Where,
                    "agent" => Token::Agent,
                    "and" => Token::And,
                    "or" => Token::Or,
                    "true" => Token::True,
                    "false" => Token::False,
                    _ => Token::Ident(ident),
                }
            }
            _ => return Err(LexError::UnexpectedChar(ch, self.pos)),
        };

        Ok(token)
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            if token == Token::Eof {
                tokens.push(token);
                break;
            }
            tokens.push(token);
        }
        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_basic() {
        let mut lexer = Lexer::new("goal \"test\" { emit \"done\"; }");
        let tokens = lexer.tokenize().unwrap();
        assert!(matches!(tokens[0], Token::Goal));
        assert!(matches!(tokens[1], Token::String(ref s) if s == "test"));
        assert!(matches!(tokens[2], Token::LBrace));
    }
}
