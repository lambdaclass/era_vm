use crate::{
    eravm_error::EraVmError,
    op_handlers::far_call::get_forward_memory_pointer,
    state::VMState,
    value::{FatPointer, TaggedValue},
    Opcode,
};

use super::far_call::perform_return;

pub fn ok(vm: &mut VMState, opcode: &Opcode) -> Result<bool, EraVmError> {
    vm.flag_eq = false;
    vm.flag_lt_of = false;
    vm.flag_gt = false;
    if vm.running_contexts.len() > 1 || !vm.current_context()?.near_call_frames.is_empty() {
        if !vm.current_context()?.near_call_frames.is_empty() {
            // Near call
            let previous_frame = vm.pop_frame()?;
            if opcode.alters_vm_flags {
                // Marks if it has .to_label
                let to_label = opcode.imm0;
                vm.current_frame_mut()?.pc = (to_label - 1) as u64; // To account for the +1 later
            } else {
                vm.current_frame_mut()?.pc -= 1; // To account for the +1 later
            }
            vm.current_frame_mut()?.gas_left += previous_frame.gas_left;
        } else {
            // Far call
            perform_return(vm, opcode)?;
            vm.register_context_u128 = 0_u128;
            vm.pop_frame()?;
        }
        Ok(false)
    } else {
        perform_return(vm, opcode)?;
        Ok(true)
    }
}
