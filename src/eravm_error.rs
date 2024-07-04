use thiserror::Error;

use crate::store::{DBError, StorageError};

#[derive(Error, Debug)]
pub enum EraVmError {
    #[error("Database Error: {0}")]
    DBError(#[from] DBError),
    #[error("Storage Error: {0}")]
    StorageError(#[from] StorageError),
    #[error("IO Error")]
    IOError(#[from] std::io::Error),
    #[error("Incorrect Bytecode Format")]
    IncorrectBytecodeFormat,
    #[error("Context Error: {0}")]
    ContextError(String),
    #[error("Operand Error: {0}")]
    OperandError(String),
    #[error("Stack Error: {0}")]
    StackError(String),
}
