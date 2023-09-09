use crate::SymbolId;

// TODO: Replace lazy errors with more structured errs
#[derive(thiserror::Error, Debug, PartialEq, Clone)]
pub enum Error {
    #[error("Failed to lex - {0}")]
    FailedToLex(String),

    #[error("Failed to parse - {0}")]
    FailedToParse(String),

    #[error("Empty expression")]
    EmptyExpression,

    #[error("Unexpected operator - {0}")]
    UnexpectedOperator(String),

    #[error("Undefined symbol - {0}")]
    UndefinedSymbol(SymbolId),

    #[error("Evaluation error - Invalid arguments to function call")]
    InvalidArgumentsToFunctionCall,

    #[error("Evaluation error - Invalid operation to call {0}")]
    InvalidOperation(crate::Value),
}
