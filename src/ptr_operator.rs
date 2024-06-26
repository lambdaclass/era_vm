use zkevm_opcode_defs::MAX_OFFSET_FOR_ADD_SUB;

use crate::{
    address_operands::{address_operands_read, address_operands_store},
    state::VMState,
    value::{FatPointer, TaggedValue},
    Opcode,
};

pub fn ptr_operands_read(vm: &mut VMState, opcode: &Opcode) -> (FatPointer, u32, TaggedValue) {
    let (src0, src1) = address_operands_read(vm, opcode);

    if !src0.is_pointer || src1.is_pointer {
        panic!("Invalid operands for {:?}", opcode.variant);
    }

    let pointer = FatPointer::decode(src0.value);
    if src1.value > MAX_OFFSET_FOR_ADD_SUB {
        panic!("Src1 too large for {:?}", opcode.variant);
    }
    let diff = src1.value.low_u32();

    (pointer, diff, src0)
}

pub fn ptr_operands_store(
    vm: &mut VMState,
    opcode: &Opcode,
    new_pointer: FatPointer,
    src0: TaggedValue,
) {
    let encoded_ptr = new_pointer.encode();
    let res = TaggedValue::new_pointer(((src0.value >> 128) << 128) | encoded_ptr);
    address_operands_store(vm, opcode, res)
}
