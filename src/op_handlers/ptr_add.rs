use crate::{address_operands::{self, address_operands_read}, state::VMState, Opcode};

pub fn _ptr_add(vm: &mut VMState, opcode: &Opcode) {
    let (src0,src1) = address_operands_read(vm, opcode);
}
