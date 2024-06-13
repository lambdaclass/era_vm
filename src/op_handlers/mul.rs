use crate::address_operands::{address_operands_read, address_operands_store};
use crate::{opcode::Opcode, state::VMState};

fn _mul_reg_only(vm: &mut VMState, opcode: Opcode) {
    // src0 * src1 -> dst0
    let src0 = vm.get_register(opcode.src0_index);
    let src1 = vm.get_register(opcode.src1_index);
    vm.set_register(opcode.dst0_index, src0 * src1);
}

fn _mul_imm16_only(vm: &mut VMState, opcode: Opcode) {
    // imm0 * src0 -> dst0
    let src1 = vm.get_register(opcode.src1_index);
    vm.set_register(opcode.dst0_index, src1 * opcode.imm0);
}

pub fn _mul(vm: &mut VMState, opcode: Opcode) {
    let (src0, src1) = address_operands_read(vm, &opcode);
    let res = src0 * src1;
    address_operands_store(vm, &opcode, res);
}
