use u256::U256;

use crate::{
    address_operands::{address_operands_read, address_operands_store},
    state::VMState,
    value::TaggedValue,
    Opcode,
};

pub fn _ptr_pack(vm: &mut VMState, opcode: &Opcode) {
    let (src0, src1) = address_operands_read(vm, opcode);

    if !src0.is_pointer || src1.is_pointer {
        panic!("Invalid operands for ptr_pack");
    }

    if src1.value & U256::from_str_radix("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF", 16).unwrap()
        != U256::zero()
    {
        panic!("Src1 low 128 bits not 0");
    }

    let res = TaggedValue::new_pointer(((src0.value << 128) >> 128) | src1.value);
    address_operands_store(vm, opcode, res)
}
