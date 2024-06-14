use crate::address_operands::{address_operands_read, address_operands_store};
use crate::{opcode::Opcode, state::VMState};

pub fn _div(vm: &mut VMState, opcode: &Opcode) {
    let (src0, src1) = address_operands_read(vm, &opcode);
    let (quotient, remainder) = src0.div_mod(src1);
    address_operands_store(vm, &opcode, (quotient, Some(remainder)));
}
