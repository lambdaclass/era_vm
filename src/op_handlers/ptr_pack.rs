use u256::U256;

use crate::{
    address_operands::{address_operands_read, address_operands_store},
    eravm_error::EraVmError,
    state::VMState,
    value::TaggedValue,
    Opcode,
};

pub fn ptr_pack(vm: &mut VMState, opcode: &Opcode) -> Result<(), EraVmError> {
    let (src0, src1) = address_operands_read(vm, opcode)?;

    if !src0.is_pointer || src1.is_pointer {
        return Err(EraVmError::OperandError(
            "Invalid operands for ptr_pack".to_string(),
        ));
    }

    if src1.value & U256::from(u128::MAX) != U256::zero() {
        return Err(EraVmError::OperandError(
            "Src1 low 128 bits not 0".to_string(),
        ));
    }

    let res = TaggedValue::new_pointer(((src0.value << 128) >> 128) | src1.value);
    address_operands_store(vm, opcode, res)
}
