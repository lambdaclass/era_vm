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
}
