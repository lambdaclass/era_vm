use crate::{
    eravm_error::{EraVmError, OperandError},
    execution::Execution,
    ptr_operator::{ptr_operands_read, ptr_operands_store},
    Opcode,
};

pub fn ptr_shrink(vm: &mut Execution, opcode: &Opcode) -> Result<(), EraVmError> {
    let (mut pointer, diff, src0) = ptr_operands_read(vm, opcode)?;

    let (new_len, overflow) = pointer.len.overflowing_sub(diff);
    if overflow {
        return Err(OperandError::Overflow(opcode.variant).into());
    }

    pointer.len = new_len;

    ptr_operands_store(vm, opcode, pointer, src0)
}
