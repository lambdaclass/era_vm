use thiserror::Error;
use zkevm_opcode_defs::Opcode;

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
    ContextError(#[from] ContextError),
    #[error("Operand Error in {0}")]
    OperandError(#[from] OperandError),
    #[error("Stack Error: {0}")]
    StackError(#[from] StackError),
    #[error("Heap Error: {0}")]
    HeapError(#[from] HeapError),
    #[error("Non Valid Forwarded Memory")]
    NonValidForwardedMemory,
    #[error("Non Valid Program Counter")]
    NonValidProgramCounter,
    #[error("Opcode error: {0}")]
    OpcodeError(#[from] OpcodeError),
    #[error("Out of gas")]
    OutOfGas,
    #[error("VM not in kernel mode")]
    VmNotInKernelMode,
    #[error("Opcode is not static")]
    OpcodeIsNotStatic,
    #[error("Invalid Calldata Access")]
    InvalidCalldataAccess,
    #[error("Decommit failed")]
    DecommitFailed,
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

#[derive(Error, Debug)]
pub enum HeapError {
    #[error("Trying to store outside of heap bounds")]
    StoreOutOfBounds,
    #[error("Trying to read outside of heap bounds")]
    ReadOutOfBounds,
    #[error("Trying to read at invalid address")]
    InvalidAddress,
}

#[derive(Error, Debug)]
pub enum OpcodeError {
    #[error("Invalid OpCode")]
    InvalidOpCode,
    #[error("Unimplemented")]
    UnimplementedOpcode,
    #[error("Invalid Opcode predicate")]
    InvalidPredicate,
}
