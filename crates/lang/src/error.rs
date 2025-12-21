use thiserror::Error;

#[derive(Debug, Error)]
pub enum LexError {
    #[error("Unexpected character '{0}' at position {1}")]
    UnexpectedChar(char, usize),
    #[error("Unterminated string at position {0}")]
    UnterminatedString(usize),
    #[error("Invalid number at position {0}")]
    InvalidNumber(usize),
}

pub type LexResult<T> = std::result::Result<T, LexError>;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Unexpected token: expected {expected}, found {found}")]
    UnexpectedToken { expected: String, found: String },
    #[error("Unexpected end of input")]
    UnexpectedEof,
    #[error("Invalid expression")]
    InvalidExpression,
    #[error("Lexer error: {0}")]
    LexError(#[from] LexError),
}

pub type ParseResult<T> = std::result::Result<T, ParseError>;

#[derive(Debug, Error)]
pub enum VMError {
    #[error("Undefined variable: {0}")]
    UndefinedVariable(String),
    #[error("Type error: {0}")]
    TypeError(String),
    #[error("Goal execution failed: {0}")]
    GoalFailed(String),
    #[error("Runtime error: {0}")]
    RuntimeError(String),
}

pub type VMResult<T> = std::result::Result<T, VMError>;

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("Compilation error: {0}")]
    CompilationFailed(String),
    #[error("Unsupported construct: {0}")]
    UnsupportedConstruct(String),
}

pub type CompileResult<T> = std::result::Result<T, CompileError>;
