use thiserror::Error;

/// Errors in the MindLang language processing pipeline.
///
/// MindLang is the agent-centric DSL for defining behaviors, goals, and reflexes.
/// These errors cover lexical analysis, parsing, compilation, and VM execution.

/// Lexical analysis errors.
#[derive(Debug, Error)]
pub enum LexError {
    /// Encountered an unexpected character during tokenization
    #[error("Unexpected character '{0}' at position {1}")]
    UnexpectedChar(char, usize),

    /// String literal was not properly terminated
    #[error("Unterminated string at position {0}")]
    UnterminatedString(usize),

    /// Number literal has invalid format
    #[error("Invalid number at position {0}")]
    InvalidNumber(usize),
}

/// Convenience Result type for lexer operations
pub type LexResult<T> = std::result::Result<T, LexError>;

/// Parsing errors.
#[derive(Debug, Error)]
pub enum ParseError {
    /// Token stream does not match expected grammar
    #[error("Unexpected token: expected {expected}, found {found}")]
    UnexpectedToken { expected: String, found: String },

    /// Reached end of input unexpectedly
    #[error("Unexpected end of input")]
    UnexpectedEof,

    /// Expression syntax is invalid
    #[error("Invalid expression")]
    InvalidExpression,

    /// Lexical analysis failed
    #[error("Lexer error: {0}")]
    LexError(#[from] LexError),
}

/// Convenience Result type for parser operations
pub type ParseResult<T> = std::result::Result<T, ParseError>;

/// Virtual machine execution errors.
#[derive(Debug, Error)]
pub enum VMError {
    /// Referenced variable has not been defined
    #[error("Undefined variable: {0}")]
    UndefinedVariable(String),

    /// Type mismatch in operation
    #[error("Type error: {0}")]
    TypeError(String),

    /// Goal execution failed to complete
    #[error("Goal execution failed: {0}")]
    GoalFailed(String),

    /// General runtime error during VM execution
    #[error("Runtime error: {0}")]
    RuntimeError(String),
}

/// Convenience Result type for VM operations
pub type VMResult<T> = std::result::Result<T, VMError>;

/// Compilation errors when generating Rust code from MindLang.
#[derive(Debug, Error)]
pub enum CompileError {
    /// Code generation failed
    #[error("Compilation error: {0}")]
    CompilationFailed(String),

    /// Language construct is not yet supported by compiler
    #[error("Unsupported construct: {0}")]
    UnsupportedConstruct(String),
}

/// Convenience Result type for compiler operations
pub type CompileResult<T> = std::result::Result<T, CompileError>;
