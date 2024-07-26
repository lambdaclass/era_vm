use u256::U256;

use crate::{
    eravm_error::EraVmError,
    state::VMState,
    value::{FatPointer, TaggedValue},
    Opcode,
};

use super::far_call::get_forward_memory_pointer;

pub fn revert(vm: &mut VMState, opcode: &Opcode) -> Result<bool, EraVmError> {
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
                vm.current_frame_mut()?.pc = (to_label - 1) as u64;
            // To account for the +1 later
            } else {
                vm.current_frame_mut()?.pc = previous_frame.exception_handler - 1;
                // To account for the +1 later
            }
            vm.current_frame_mut()?.gas_left += previous_frame.gas_left;
        } else {
            let register = vm.get_register(opcode.src0_index);
            let result = get_forward_memory_pointer(register.value, vm, register.is_pointer)?;
            vm.clear_registers();
            vm.set_register(1, TaggedValue::new_pointer(FatPointer::encode(&result)));
            vm.flag_lt_of = true;
            let previous_frame = vm.pop_frame()?;
            vm.current_frame_mut()?.pc = previous_frame.exception_handler;
        }
        Ok(false)
    } else {
        let register = vm.get_register(opcode.src0_index);
        let result = get_forward_memory_pointer(register.value, vm, register.is_pointer)?;
        vm.set_register(
            opcode.src0_index,
            TaggedValue::new_pointer(FatPointer::encode(&result)),
        );
        Ok(true)
    }
}

fn revert_near_call(vm: &mut VMState) -> Result<(), EraVmError> {
    let previous_frame = vm.pop_frame()?;

    let current_frame = vm.current_frame_mut()?;
    current_frame.heap_id = previous_frame.heap_id;
    current_frame.aux_heap_id = previous_frame.aux_heap_id;
    current_frame.pc = previous_frame.exception_handler - 1; // To account for the +1 later
    current_frame.gas_left += previous_frame.gas_left;
    Ok(())
}

fn revert_far_call(vm: &mut VMState) -> Result<(), EraVmError> {
    vm.clear_registers();
    vm.set_register(1, TaggedValue::new_pointer(U256::zero()));
    vm.flag_lt_of = true;
    vm.register_context_u128 = 0_u128;
    let previous_frame = vm.pop_frame()?;
    vm.current_frame_mut()?.pc = previous_frame.exception_handler;
    vm.current_context_mut()?.context_u128 = 0_u128;
    Ok(())
}

pub fn revert_out_of_gas(vm: &mut VMState) -> Result<(), EraVmError> {
    vm.flag_eq = false;
    vm.flag_lt_of = false;
    vm.flag_gt = false;
    if !vm.current_context()?.near_call_frames.is_empty() {
        // Near call
        let previous_frame = vm.pop_frame()?;
        vm.current_frame_mut()?.pc = previous_frame.exception_handler - 1; // To account for the +1 later
        vm.current_frame_mut()?.gas_left += previous_frame.gas_left;
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
