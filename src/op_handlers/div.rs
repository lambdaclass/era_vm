use crate::address_operands::address_operands_read;
use crate::{opcode::Opcode, state::VMState};

pub fn _div(vm: &mut VMState, opcode: Opcode) {
    let (src0, src1) = address_operands_read(vm, &opcode);
    vm.set_register(opcode.dst0_index, src0 / src1);
    vm.set_register(opcode.dst1_index, src0 % src1);
}
