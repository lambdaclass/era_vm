use crate::address_operands::{address_operands_read, address_operands_store};
use crate::value::TaggedValue;
use crate::{opcode::Opcode, state::VMState};

pub fn and(vm: &mut VMState, opcode: &Opcode) {
    let (src0, src1) = address_operands_read(vm, opcode);
    let res = src0.value & src1.value;
    if opcode.alters_vm_flags {
        // Always cleared
        vm.flag_lt_of = false;
        // Set eq if res == 0
        vm.flag_eq = res.is_zero();
        // Always cleared
        vm.flag_gt = false;
    }
    address_operands_store(vm, opcode, TaggedValue::new_raw_integer(res));
}
