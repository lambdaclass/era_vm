use crate::{
    eravm_error::{EraVmError, OperandError},
    ptr_operator::{ptr_operands_read, ptr_operands_store},
    state::VMState,
    value::FatPointer,
    Opcode,
};

pub fn ptr_shrink(vm: &mut VMState, opcode: &Opcode) -> Result<(), EraVmError> {
    let (pointer, diff, src0) = ptr_operands_read(vm, opcode)?;

    let (new_len, overflow) = pointer.len.overflowing_sub(diff);
    if overflow {
        return Err(OperandError::Overflow(opcode.variant).into());
    }
    let new_pointer = FatPointer {
        len: new_len,
        ..pointer
    };
    ptr_operands_store(vm, opcode, new_pointer, src0)
}
