use crate::{eravm_error::EraVmError, state::VMState, Opcode};

pub fn revert(vm: &mut VMState, opcode: &Opcode) -> Result<bool, EraVmError> {
    vm.flag_eq = false;
    vm.flag_lt_of = false;
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
            current_frame.gas_left += previous_frame.gas_left;
        } else {
            revert_far_call(vm)?;
        }
        Ok(false)
    } else {
        Ok(true)
    }
}

fn revert_near_call(vm: &mut VMState) -> Result<(), EraVmError> {
    let previous_frame = vm.pop_frame()?;

    let current_frame = vm.current_frame_mut()?;
    current_frame.stack = previous_frame.stack;
    current_frame.heap = previous_frame.heap;
    current_frame.aux_heap = previous_frame.aux_heap;
    current_frame.pc = previous_frame.exception_handler - 1; // To account for the +1 later
    current_frame.gas_left += previous_frame.gas_left;
    Ok(())
}

fn revert_far_call(vm: &mut VMState) -> Result<(), EraVmError> {
    vm.pop_frame()?;
    Ok(())
}

pub fn revert_out_of_gas(vm: &mut VMState) -> Result<(), EraVmError> {
    vm.flag_eq = false;
    vm.flag_lt_of = false;
    vm.flag_gt = false;
    if !vm.current_context()?.near_call_frames.is_empty() {
        revert_near_call(vm)?;
    } else {
        revert_far_call(vm)?;
    };
    Ok(())
}

pub fn handle_error(vm: &mut VMState, err: EraVmError) -> Result<(), EraVmError> {
    vm.flag_eq = false;
    vm.flag_lt_of = false;
    vm.flag_gt = false;
    if !vm.current_context()?.near_call_frames.is_empty() {
        revert_near_call(vm)?;
    } else if vm.running_contexts.len() > 1 {
        revert_far_call(vm)?;
    } else {
        // Main context
        return Err(err);
    };
    Ok(())
}
