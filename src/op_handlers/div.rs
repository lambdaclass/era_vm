use crate::address_operands::{address_operands_read, address_operands_store};
use crate::{opcode::Opcode, state::VMState};

pub fn _div(vm: &mut VMState, opcode: &Opcode) {
    let (src0, src1) = address_operands_read(vm, &opcode);
    let (quotient, remainder) = src0.div_mod(src1);
    if opcode.alters_vm_flags {
        // If overflow, set the flag.
        // otherwise keep the current value.
        // vm.flag_lt_of |= overflow;
        // Set eq if res == 0
        // vm.flag_eq |= quotient.is_zero();
        // Gt is set if both of lt_of and eq are cleared.
        vm.flag_gt |= !vm.flag_lt_of && !vm.flag_eq;
    }

    address_operands_store(vm, &opcode, (quotient, Some(remainder)));
}
