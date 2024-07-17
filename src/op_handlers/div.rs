
use u256::U256;

use crate::address_operands::{address_operands_div_mul, address_operands_read};
use crate::value::TaggedValue;
use crate::{opcode::Opcode, state::VMState};

pub fn div(vm: &mut VMState, opcode: &Opcode) {
    let (src0_t, src1_t) = address_operands_read(vm, opcode);
    let (src0, src1) = (src0_t.value, src1_t.value);
    let (quotient, remainder) = src0.div_mod(src1);
    if opcode.alters_vm_flags {
        // Lt overflow is cleared
        vm.flag_lt_of = false;
        // Eq is set if quotient is zero
        vm.flag_eq = quotient.is_zero();
        // Gt is set if the remainder is zero
        vm.flag_gt = remainder.is_zero();
    }

    address_operands_div_mul(
        vm,
        opcode,
        (
            TaggedValue::new_raw_integer(quotient),
            TaggedValue::new_raw_integer(remainder),
        ),
    );
}
