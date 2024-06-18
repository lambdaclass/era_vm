use crate::address_operands::{address_operands_read, address_operands_store};
use crate::{opcode::Opcode, state::VMState};

fn _add_reg_only(vm: &mut VMState, opcode: &Opcode) {
    // src0 + src1 -> dst0
    let src0 = vm.get_register(opcode.src0_index);
    let src1 = vm.get_register(opcode.src1_index);
    vm.set_register(opcode.dst0_index, src0 + src1);
}

fn _add_imm16_only(vm: &mut VMState, opcode: &Opcode) {
    // imm0 + src0 -> dst0
    let src1 = vm.get_register(opcode.src1_index);
    vm.set_register(opcode.dst0_index, src1 + opcode.imm0);
}

pub fn _add(vm: &mut VMState, opcode: &Opcode) {
    println!("add");
    let (src0, src1) = address_operands_read(vm, opcode);
    // res = (src0 + src1) mod (2**256);
    let (res, overflow) = src0.overflowing_add(src1);
    if opcode.alters_vm_flags {
        // If overflow, set the flag.
        // otherwise keep the current value.
        vm.flag_lt_of |= overflow;
        // Set eq if res == 0
        vm.flag_eq |= res.is_zero();
        // Gt is set if both of lt_of and eq are cleared.
        vm.flag_gt |= !vm.flag_lt_of && !vm.flag_eq;
    }
    address_operands_store(vm, opcode, res);
}
