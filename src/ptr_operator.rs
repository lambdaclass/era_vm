use crate::{
    address_operands::{address_operands_read, address_operands_store},
    eravm_error::{EraVmError, OperandError},
    execution::Execution,
    value::{FatPointer, TaggedValue},
    Opcode,
};

pub fn ptr_operands_read(
    vm: &mut Execution,
    opcode: &Opcode,
) -> Result<(FatPointer, u32, TaggedValue), EraVmError> {
    let (src0, src1) = address_operands_read(vm, opcode)?;

    if !src0.is_pointer || src1.is_pointer {
        return Err(OperandError::InvalidSrcPointer(opcode.variant).into());
    }

    let pointer = FatPointer::decode(src0.value);
    if src1.value > u32::MAX.into() {
        return Err(OperandError::Src1TooLarge(opcode.variant).into());
    }
    let diff = src1.value.low_u32();

    Ok((pointer, diff, src0))
}

pub fn ptr_operands_store(
    vm: &mut Execution,
    opcode: &Opcode,
    new_pointer: FatPointer,
    src0: TaggedValue,
) -> Result<(), EraVmError> {
    let encoded_ptr = new_pointer.encode();
    let res = TaggedValue::new_pointer(((src0.value >> 128) << 128) | encoded_ptr);
    address_operands_store(vm, opcode, res)
}
