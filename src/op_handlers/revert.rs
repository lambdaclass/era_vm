use crate::{state::VMState, Opcode};

pub fn revert(vm: &mut VMState, opcode: &Opcode) -> bool {
    vm.flag_eq = false;
    vm.flag_lt_of = false;
    vm.flag_gt = false;
    if vm.running_contexts.len() > 1 || !vm.current_context().near_call_frames.is_empty() {
        if !vm.current_context().near_call_frames.is_empty() {
            // Near call
            let previous_frame = vm.pop_frame();
            vm.current_frame_mut().stack = previous_frame.stack;
            vm.current_frame_mut().heap = previous_frame.heap;
            vm.current_frame_mut().aux_heap = previous_frame.aux_heap;
            if opcode.alters_vm_flags {
                // Marks if it has .to_label
                let to_label = opcode.imm0;
                vm.current_frame_mut().pc = (to_label - 1) as u64; // To account for the +1 later
            } else {
                vm.current_frame_mut().pc = previous_frame.exception_handler - 1;
                // To account for the +1 later
            }
            vm.current_frame_mut().gas_left += previous_frame.gas_left;
        } else {
            // Far call
            vm.pop_frame();
        }
        false
    } else {
        true
    }
}

pub fn revert_out_of_gas(vm: &mut VMState) {
    vm.flag_eq = false;
    vm.flag_lt_of = false;
    vm.flag_gt = false;
    if !vm.current_context().near_call_frames.is_empty() {
        // Near call
        let previous_frame = vm.pop_frame();
        vm.current_frame_mut().stack = previous_frame.stack;
        vm.current_frame_mut().heap = previous_frame.heap;
        //  vm.current_frame_mut().aux_hep = previous_frame.aux_heap;
        vm.current_frame_mut().pc = previous_frame.exception_handler - 1; // To account for the +1 later
        vm.current_frame_mut().gas_left += previous_frame.gas_left;
    } else {
        // Far call
        vm.pop_frame();
    }
}
