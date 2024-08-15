use u256::{U256, U512};

use crate::address_operands::{address_operands_div_mul, address_operands_read};
use crate::eravm_error::EraVmError;
use crate::value::TaggedValue;
use crate::{opcode::Opcode, execution::Execution};

pub fn mul(vm: &mut Execution, opcode: &Opcode) -> Result<(), EraVmError> {
    let (src0, src1) = address_operands_read(vm, opcode)?;
    let (src0, src1) = (src0.value, src1.value);
    let src0 = U512::from(src0);
    let src1 = U512::from(src1);
    let res = src0 * src1;

    let u256_mask = U512::from(U256::MAX);
    let low_bits = res & u256_mask;
    let high_bits = res >> 256 & u256_mask;

    if opcode.flag0_set {
        // Lt overflow, is set if
        // src0 * src1 >= 2^256
        let overflow = res >= U512::from(U256::MAX);
        vm.flag_lt_of = overflow;
        // Eq is set if res_low == 0
        vm.flag_eq = low_bits.is_zero();
        // Gt is set if both of lt_of and eq are cleared.
        vm.flag_gt = !vm.flag_lt_of && !vm.flag_eq;
    }

    address_operands_div_mul(
        vm,
        opcode,
        (
            TaggedValue::new_raw_integer(low_bits.try_into().unwrap()),
            TaggedValue::new_raw_integer(high_bits.try_into().unwrap()),
        ), // safe to unwrap, as we have applied the mask
    )
}
