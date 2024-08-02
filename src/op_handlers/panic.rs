use crate::{eravm_error::EraVmError, state::VMState, value::TaggedValue, Opcode};
use u256::U256;
use zkevm_opcode_defs::Opcode as Variant;

pub fn panic(vm: &mut VMState, opcode: &Opcode) -> Result<bool, EraVmError> {
    vm.flag_eq = false;
    vm.flag_lt_of = true;
    vm.flag_gt = false;

    if vm.in_near_call()? {
        let previous_frame = vm.pop_frame()?;
        // Marks if it has .to_label
        if opcode.alters_vm_flags {
            let to_label = opcode.imm0;
            vm.current_frame_mut()?.pc = (to_label - 1) as u64; // To account for the +1 later
        } else {
            vm.current_frame_mut()?.pc = previous_frame.exception_handler - 1; // To account for the +1 later
        }

        vm.current_frame_mut()?.gas_left += previous_frame.gas_left;
        Ok(false)
    } else if vm.in_far_call() {
        vm.clear_registers();
        vm.register_context_u128 = 0;
        let previous_frame = vm.pop_frame()?;
        vm.set_register(1, TaggedValue::new_pointer(U256::zero()));
        vm.current_frame_mut()?.pc = previous_frame.exception_handler;
        vm.current_frame_mut()?.gas_left += previous_frame.gas_left;
        Ok(false)
    } else {
        Ok(true)
    }
}

/// Call this when:
/// - gas runs out when paying for the fixed cost of an instruction
/// - causing side effects in a static context
/// - using privileged instructions while not in a system call
/// - the far call stack overflows
pub fn handle_error(vm: &mut VMState) -> Result<bool, EraVmError> {
    vm.flag_eq = false;
    vm.flag_lt_of = true;
    vm.flag_gt = false;

    if vm.in_near_call()? {
        let previous_frame = vm.pop_frame()?;
        vm.current_frame_mut()?.pc = previous_frame.exception_handler - 1; // To account for the +1 later
        vm.current_frame_mut()?.gas_left += previous_frame.gas_left;
        Ok(false)
    } else if vm.in_far_call() {
        vm.clear_registers();
        vm.register_context_u128 = 0;
        let previous_frame = vm.pop_frame()?;
        vm.set_register(1, TaggedValue::new_pointer(U256::zero()));
        vm.current_frame_mut()?.pc = previous_frame.exception_handler - 1; // To account for the +1 later
        vm.current_frame_mut()?.gas_left += previous_frame.gas_left;
        Ok(false)
    } else {
        Ok(true)
    }
}
