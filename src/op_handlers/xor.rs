use crate::address_operands::{address_operands_read, address_operands_store};
use crate::eravm_error::EraVmError;
use crate::value::TaggedValue;
use crate::{opcode::Opcode, execution::Execution};

pub fn xor(vm: &mut Execution, opcode: &Opcode) -> Result<(), EraVmError> {
    let (src0, src1) = address_operands_read(vm, opcode)?;

    let res = src0.value ^ src1.value;
    if opcode.flag0_set {
        // Always cleared
        vm.flag_lt_of = false;
        // Set eq if res == 0
        vm.flag_eq = res.is_zero();
        // Always cleared
        vm.flag_gt = false;
    }
    address_operands_store(vm, opcode, TaggedValue::new_raw_integer(res))
}
