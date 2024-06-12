use crate::address_operands::{address_operands_read, address_operands_store};
use crate::{opcode::Opcode, state::VMState};

pub fn _sub(vm: &mut VMState, opcode: Opcode) {
    let (src0, src1) = address_operands_read(vm, &opcode);
    let res = src0 - src1;
    address_operands_store(vm, &opcode, res);
}
