use u256::{U256, U512};

use crate::address_operands::{address_operands_read, address_operands_store};
use crate::{opcode::Opcode, state::VMState};

pub fn _mul(vm: &mut VMState, opcode: &Opcode) {
    let (src0, src1) = address_operands_read(vm, &opcode);
    let src0 = U512::from(src0);
    let src1 = U512::from(src1);
    let res = src0 * src1;

    let u256_mask = U512::from(U256::MAX);
    let low_bits = res & u256_mask;
    let high_bits = res >> 256 & u256_mask;

    if opcode.alters_vm_flags {
        // Lt overflow, is set if
        // src0 * src1 >= 2^256
        let overflow = res >= U512::from(U256::MAX);
        vm.flag_lt_of |= overflow;
        // Eq is set if res_low == 0
        vm.flag_eq |= low_bits.is_zero();
        // Gt is set if both of lt_of and eq are cleared.
        vm.flag_gt |= !vm.flag_lt_of && !vm.flag_eq;
    }

    address_operands_store(
        vm,
        &opcode,
        (
            low_bits.try_into().unwrap(),
            Some(high_bits.try_into().unwrap()),
        ),
    );
}
