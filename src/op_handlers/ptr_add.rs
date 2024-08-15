use crate::{
    eravm_error::{EraVmError, OperandError},
    execution::Execution,
    ptr_operator::{ptr_operands_read, ptr_operands_store},
    value::FatPointer,
    Opcode,
};

pub fn ptr_add(vm: &mut Execution, opcode: &Opcode) -> Result<(), EraVmError> {
    let (pointer, diff, src0) = ptr_operands_read(vm, opcode)?;

    let (new_offset, overflow) = pointer.offset.overflowing_add(diff);
    if overflow {
        return Err(OperandError::Overflow(opcode.variant).into());
    }
    let new_pointer = FatPointer {
        offset: new_offset,
        ..pointer
    };
    ptr_operands_store(vm, opcode, new_pointer, src0)
}
