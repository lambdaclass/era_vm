use u256::U256;

use crate::{
    eravm_error::{EraVmError, HeapError},
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
            vm.current_frame_mut()?.stack = previous_frame.stack;
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
            revert_far_call(vm, opcode)?;
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
    current_frame.heap_id = previous_frame.heap_id;
    current_frame.aux_heap_id = previous_frame.aux_heap_id;
    current_frame.pc = previous_frame.exception_handler - 1; // To account for the +1 later
    current_frame.gas_left += previous_frame.gas_left;
    Ok(())
}

fn revert_far_call(vm: &mut VMState, opcode: &Opcode) -> Result<(), EraVmError> {
    for i in 2..(vm.registers.len() + 1) {
        vm.set_register(i as u8, TaggedValue::new_raw_integer(U256::zero()));
    }
    let register = vm.get_register(opcode.src0_index);
    let result = get_forward_memory_pointer(register.value, vm, register.is_pointer)?
        .ok_or(HeapError::ReadOutOfBounds)?;
    vm.set_register(1, TaggedValue::new_pointer(FatPointer::encode(&result))); // TODO: Check what else is needed
    vm.flag_lt_of = true;
    let previous_frame = vm.pop_frame()?;
    vm.current_frame_mut()?.pc = previous_frame.exception_handler;
    Ok(())
}

pub fn revert_out_of_gas(vm: &mut VMState, opcode: &Opcode) -> Result<(), EraVmError> {
    vm.flag_eq = false;
    vm.flag_lt_of = false;
    vm.flag_gt = false;
    if !vm.current_context()?.near_call_frames.is_empty() {
        // Near call
        let previous_frame = vm.pop_frame()?;
        vm.current_frame_mut()?.pc = previous_frame.exception_handler - 1; // To account for the +1 later
        vm.current_frame_mut()?.gas_left += previous_frame.gas_left;
    } else {
        revert_far_call(vm, opcode)?;
    };
    Ok(())
}

pub fn handle_error(vm: &mut VMState, err: EraVmError, opcode: &Opcode) -> Result<(), EraVmError> {
    vm.flag_eq = false;
    vm.flag_lt_of = false;
    vm.flag_gt = false;
    dbg!(&err);
    if !vm.current_context()?.near_call_frames.is_empty() {
        dbg!("Reverting near call");
        revert_near_call(vm)?;
    } else if vm.running_contexts.len() > 1 {
        dbg!("Reverting far call");
        revert_far_call(vm, opcode)?;
    } else {
        dbg!("Err");
        // Main context
        return Err(err);
    };
    Ok(())
}
