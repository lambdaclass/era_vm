use crate::state::VMState;

pub fn _panic(vm: &mut VMState) -> bool {
    vm.flag_eq = false;
    vm.flag_lt_of = true;
    vm.flag_gt = false;
    if vm.running_contexts.len() > 1 || !vm.current_context().near_call_frames.is_empty() {
        if !vm.current_context().near_call_frames.is_empty() {
            // Near call
            let previous_frame = vm.pop_frame();
            vm.current_frame_mut().stack = previous_frame.stack;
            vm.current_frame_mut().heap = previous_frame.heap;
            //  vm.current_frame_mut().aux_hep = previous_frame.aux_heap;
            vm.current_frame_mut().pc = previous_frame.exception_handler - 1; // To account for the +1 later
        } else {
            // Far call
            vm.pop_frame();
        }
        false
    } else {
        true
    }
}
