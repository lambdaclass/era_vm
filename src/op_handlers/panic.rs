use crate::{eravm_error::EraVmError, state::VMState, store::Storage, value::TaggedValue, Opcode};
use u256::U256;

pub fn panic(
    vm: &mut VMState,
    opcode: &Opcode,
    storage: &mut dyn Storage,
) -> Result<bool, EraVmError> {
    vm.flag_eq = false;
    vm.flag_lt_of = true;
    vm.flag_gt = false;
    storage.rollback(&vm.current_frame()?.storage_before.clone());
    if vm.in_near_call()? {
        let previous_frame = vm.pop_frame()?;
        // Marks if it has .to_label
        if opcode.alters_vm_flags {
            let to_label = opcode.imm0;
            vm.current_frame_mut()?.pc = to_label as u64;
        } else {
            vm.current_frame_mut()?.pc = previous_frame.exception_handler;
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

/// This functions has the behavior of panic but runs under any EraVmError, which would include:
/// - gas runs out when paying for the fixed cost of an instruction
/// - causing side effects in a static context
/// - using privileged instructions while not in a system call
/// - the far call stack overflows
/// - an instruction returns an err.
pub fn inexplicit_panic(vm: &mut VMState, storage: &mut dyn Storage) -> Result<bool, EraVmError> {
    vm.flag_eq = false;
    vm.flag_lt_of = true;
    vm.flag_gt = false;
    storage.rollback(&vm.current_frame()?.storage_before.clone());
    if vm.in_near_call()? {
        let previous_frame = vm.pop_frame()?;
        vm.current_frame_mut()?.pc = previous_frame.exception_handler;
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
