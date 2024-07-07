use crate::address_operands::{address_operands_read, address_operands_store};
use crate::value::TaggedValue;
use crate::{opcode::Opcode, state::VMState};

pub fn sub(vm: &mut VMState, opcode: &Opcode) {
    let (src0_t, src1_t) = address_operands_read(vm, opcode);
    let (src0, src1) = (src0_t.value, src1_t.value);
    // res = (src0 - src1) mod (2**256);
    let (res, overflow) = src0.overflowing_sub(src1);
    if opcode.alters_vm_flags {
        // Overflow <-> src0 < src1
        vm.flag_lt_of |= overflow;
        // Set eq if res == 0
        vm.flag_eq |= res.is_zero();
        // Gt is set if both of lt_of and eq are cleared.
        vm.flag_gt |= !vm.flag_lt_of && !vm.flag_eq;
    }
    address_operands_store(vm, opcode, TaggedValue::new_raw_integer(res));
}
