use u256::U256;

use crate::address_operands::{address_operands_read, address_operands_store};
use crate::{opcode::Opcode, state::VMState};

pub fn _mul(vm: &mut VMState, opcode: &Opcode) {
    let (src0, src1) = address_operands_read(vm, &opcode);

    let max = U256::max_value();
    let low_mask = U256::from(max.low_u128());
    let high_mask = !low_mask & max;

    let (res, overflow) = src0.overflowing_mul(src1);
    if opcode.alters_vm_flags {
        // If overflow, set the flag.
        // otherwise keep the current value.
        vm.flag_lt_of |= overflow;
        // Set eq if res == 0
        vm.flag_eq |= res.is_zero();
        // Gt is set if both of lt_of and eq are cleared.
        vm.flag_gt |= !vm.flag_lt_of && !vm.flag_eq;
    }

    let low_bits = res & low_mask;
    let high_bits = res & high_mask;
    address_operands_store(vm, &opcode, (low_bits, Some(high_bits)));
}
