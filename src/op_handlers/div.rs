use crate::address_operands::{address_operands_div_mul, address_operands_read};
use crate::{opcode::Opcode, state::VMState};

pub fn _div(vm: &mut VMState, opcode: &Opcode) {
    let (src0, src1) = address_operands_read(vm, opcode);
    let (quotient, remainder) = src0.div_mod(src1);
    if opcode.alters_vm_flags {
        // Lt overflow is cleared
        vm.flag_lt_of = false;
        // Eq is set if quotient is not zero
        vm.flag_eq = !quotient.is_zero();
        // Gt is set if the remainder is not zero
        vm.flag_gt = !remainder.is_zero();
    }

    address_operands_div_mul(vm, opcode, (quotient, remainder));
}
