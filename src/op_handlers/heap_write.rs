use u256::U256;
use zkevm_opcode_defs::MAX_OFFSET_TO_DEREF_LOW_U32;

use crate::address_operands::address_operands_read;
use crate::value::TaggedValue;
use crate::{opcode::Opcode, state::VMState};

pub fn heap_write(vm: &mut VMState, opcode: &Opcode) {
    let (src0, src1) = address_operands_read(vm, opcode);
    if src0.is_pointer {
        panic!("Invalid operands for heap_write");
    }

    if src0.value > U256::from(MAX_OFFSET_TO_DEREF_LOW_U32) {
        panic!("Address too large for heap_write");
    }
    let addr = src0.value.low_u32();

    let gas_cost = vm
        .heaps
        .get_mut(vm.current_frame().heap_id)
        .unwrap()
        .expand_memory(addr + 32);
    vm.current_frame_mut().gas_left -= gas_cost;

    vm.heaps
        .get_mut(vm.current_frame().heap_id)
        .unwrap()
        .store(addr, src1.value);

    if opcode.alters_vm_flags {
        // This flag is set if .inc is present
        vm.set_register(
            opcode.dst0_index,
            TaggedValue::new_raw_integer(U256::from(addr + 32)),
        );
    }
}
