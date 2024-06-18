use u256::U256;
use zkevm_opcode_defs::MAX_OFFSET_TO_DEREF_LOW_U32;

use crate::address_operands::address_operands_read;
use crate::{opcode::Opcode, state::VMState};

pub fn _heap_read(vm: &mut VMState, opcode: &Opcode) {
    let (src0, src1) = address_operands_read(vm, opcode);
    if src0.is_pointer {
        panic!("Invalid operands for heap_write");
    }

    if src0.value > U256::from(MAX_OFFSET_TO_DEREF_LOW_U32) {
        panic!("Address too large for heap_write");
    }
    let addr = src0.value.low_u32();

    vm.current_frame.heap.expand_memory(addr + 32); // TODO: Handle ergs cost

    let value = vm.current_frame.heap.read(addr);
    vm.set_register(opcode.dst0_index, value);
}
