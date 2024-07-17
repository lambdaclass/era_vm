use thiserror::Error;
use zkevm_opcode_defs::Opcode;

use crate::{
    store::{DBError, StorageError},
    // Opcode,
};

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
    ContextError(#[from] ContextError),
    #[error("Operand Error in {0}")]
    OperandError(#[from] OperandError),
    #[error("Stack Error: {0}")]
    StackError(#[from] StackError),
}

#[derive(Error, Debug)]
pub enum OperandError {
    #[error("{0:?}: Dest cannot be imm16 only")]
    InvalidDestImm16Only(Opcode),
    #[error("{0:?}: Dest cannot be code page")]
    InvalidDestCodePage(Opcode),
    #[error("{0:?}: Src cannot be a pointer")]
    InvalidSrcPointer(Opcode),
    #[error("{0:?}: Src must be a pointer")]
    InvalidSrcNotPointer(Opcode),
    #[error("{0:?}: Src address is too large")]
    InvalidSrcAddress(Opcode),
    #[error("{0:?}: Src1 low 128 bits are not 0")]
    Src1LowNotZero(Opcode),
    #[error("{0:?}: Src1 too large")]
    Src1TooLarge(Opcode),
    #[error("{0:?}: Overflow")]
    Overflow(Opcode),
}

#[derive(Error, Debug)]
pub enum ContextError {
    #[error("VM has no running contract")]
    NoContract,
}

#[derive(Error, Debug)]
pub enum StackError {
    #[error("Underflow")]
    Underflow,
    #[error("Trying to store outside of stack bounds")]
    StoreOutOfBounds,
    #[error("Trying to read outside of stack bounds")]
    ReadOutOfBounds,
}
