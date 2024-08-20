use u256::U256;

use crate::{
    address_operands::{address_operands_read, address_operands_store},
    eravm_error::{EraVmError, OperandError},
    execution::Execution,
    value::TaggedValue,
    Opcode,
};

pub fn ptr_pack(vm: &mut Execution, opcode: &Opcode) -> Result<(), EraVmError> {
    let (src0, src1) = address_operands_read(vm, opcode)?;

    if !src0.is_pointer || src1.is_pointer {
        return Err(OperandError::InvalidSrcPointer(opcode.variant).into());
    }

    // Check if lower 128 bytes are zero
    if (src1.value.0[0] | src1.value.0[1]) != 0 {
        return Err(OperandError::Src1LowNotZero(opcode.variant).into());
    }

    let mut value = U256::zero();

    value.0[3] = src1.value.0[3];
    value.0[2] = src1.value.0[2];
    value.0[1] = src0.value.0[1];
    value.0[0] = src0.value.0[0];

    let res = TaggedValue::new_pointer(value);

    address_operands_store(vm, opcode, res)
}
