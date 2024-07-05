use crate::{
    eravm_error::EraVmError,
    ptr_operator::{ptr_operands_read, ptr_operands_store},
    state::VMState,
    value::FatPointer,
    Opcode,
};

pub fn _ptr_sub(vm: &mut VMState, opcode: &Opcode) -> Result<(), EraVmError> {
    let (pointer, diff, src0) = ptr_operands_read(vm, opcode)?;
    let (new_offset, overflow) = pointer.offset.overflowing_sub(diff);
    if overflow {
        return Err(EraVmError::OperandError(
            "Offset overflow in ptr_sub".to_string(),
        ));
    }
    let new_pointer = FatPointer {
        offset: new_offset,
        ..pointer
    };
    ptr_operands_store(vm, opcode, new_pointer, src0)
}
