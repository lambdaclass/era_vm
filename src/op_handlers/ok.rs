use crate::{
    op_handlers::far_call::get_forward_memory_pointer,
    state::VMState,
    value::{FatPointer, TaggedValue},
    Opcode,
};

fn far_call_ret_routine(vm: &mut VMState, opcode: &Opcode) {}
pub fn ok(vm: &mut VMState, opcode: &Opcode) -> bool {
    vm.flag_eq = false;
    vm.flag_lt_of = false;
    vm.flag_gt = false;
    let mut should_stop = false;
    if vm.running_contexts.len() > 1 || !vm.current_context().near_call_frames.is_empty() {
        let is_near_call_frame = !vm.current_context().near_call_frames.is_empty();
        vm.pop_frame();
        // In near frame context, so do a near call ret.
        if is_near_call_frame {
            // Near call
            let previous_frame = vm.pop_frame();
            vm.current_frame_mut().stack = previous_frame.stack;
            if opcode.alters_vm_flags {
                // Marks if it has .to_label
                let to_label = opcode.imm0;
                vm.current_frame_mut().pc = (to_label - 1) as u64; // To account for the +1 later
            } else {
                vm.current_frame_mut().pc -= 1; // To account for the +1 later
            }
            vm.current_frame_mut().gas_left += previous_frame.gas_left;
        } else {
            // Not in a call frame, but ret was called,
            // so do a far call_ret.
            let register = vm.get_register(opcode.src0_index);
            let result = get_forward_memory_pointer(register.value, vm, register.is_pointer);
            // Store result on first register, like vm2 does.
            vm.set_register(
                opcode.src0_index,
                TaggedValue::new_pointer(FatPointer::encode(&result.unwrap())),
            );
            vm.current_context_mut().context_u128 = 0_u128;
            vm.clear_registers();
        }
    } else {
        // In the frame context, so do a near call ret.
        let register = vm.get_register(opcode.src0_index);
        let result = get_forward_memory_pointer(register.value, vm, register.is_pointer);
        vm.set_register(
            opcode.src0_index,
            TaggedValue::new_pointer(FatPointer::encode(&result.unwrap())),
        );
        should_stop = true;
    }
    dbg!(should_stop);
    should_stop
}
