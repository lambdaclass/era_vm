use crate::{eravm_error::EraVmError, state::VMState, Opcode};

pub fn panic(vm: &mut VMState, opcode: &Opcode) -> Result<bool, EraVmError> {
    vm.flag_eq = false;
    vm.flag_lt_of = true;
    vm.flag_gt = false;
    if vm.running_contexts.len() > 1 || !vm.current_context()?.near_call_frames.is_empty() {
        if !vm.current_context()?.near_call_frames.is_empty() {
            let previous_frame = vm.pop_frame()?;
            let current_frame = vm.current_frame_mut()?;
            // Near call
            current_frame.stack = previous_frame.stack;
            current_frame.heap = previous_frame.heap;
            current_frame.aux_heap = previous_frame.aux_heap;
            if opcode.alters_vm_flags {
                // Marks if it has .to_label
                let to_label = opcode.imm0;
                current_frame.pc = (to_label - 1) as u64; // To account for the +1 later
            } else {
                current_frame.pc = previous_frame.exception_handler - 1;
                // To account for the +1 later
            }
        } else {
            // Far call
            vm.pop_frame()?;
        }
        Ok(false)
    } else {
        Ok(true)
    }
}
