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

    if src1.value & U256::from(u128::MAX) != U256::zero() {
        return Err(OperandError::Src1LowNotZero(opcode.variant).into());
    }

    let res = TaggedValue::new_pointer(((src0.value << 128) >> 128) | src1.value);
    address_operands_store(vm, opcode, res)
}
