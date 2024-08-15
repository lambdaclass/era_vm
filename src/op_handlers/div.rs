use u256::U256;

use crate::address_operands::{address_operands_div_mul, address_operands_read};
use crate::eravm_error::EraVmError;
use crate::value::TaggedValue;
use crate::{opcode::Opcode, execution::Execution};

pub fn div(vm: &mut Execution, opcode: &Opcode) -> Result<(), EraVmError> {
    let (src0_t, src1_t) = address_operands_read(vm, opcode)?;
    let (src0, src1) = (src0_t.value, src1_t.value);
    let mut quotient = U256::zero();
    let mut remainder = U256::zero();
    if src1.is_zero() {
        if opcode.flag0_set {
            // Lt overflow is set
            vm.flag_lt_of = true;
            // Eq is set if resultlow is 0, in this case its always 0
            vm.flag_eq = true;
            // Gt is set if LT_OF and EQ are cleared, they are not
            vm.flag_gt = false;
        }
    } else {
        (quotient, remainder) = src0.div_mod(src1);
        if opcode.flag0_set {
            // Lt overflow is cleared
            vm.flag_lt_of = false;
            // Eq is set if quotient is zero
            vm.flag_eq = quotient.is_zero();
            // Gt is set if the remainder is zero
            vm.flag_gt = remainder.is_zero();
        }
    }

    address_operands_div_mul(
        vm,
        opcode,
        (
            TaggedValue::new_raw_integer(quotient),
            TaggedValue::new_raw_integer(remainder),
        ),
    )
}
