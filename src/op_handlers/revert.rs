use crate::{
    eravm_error::EraVmError,
    state::VMState,
    value::{FatPointer, TaggedValue},
    Opcode,
};
use super::far_call::{get_forward_memory_pointer, perform_return};
pub fn revert(vm: &mut VMState, opcode: &Opcode) -> Result<bool, EraVmError> {
    vm.flag_eq = false;
    vm.flag_lt_of = false;
    vm.flag_gt = false;
    if vm.in_near_call()? {
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
        Ok(false)
    } else if vm.in_far_call() {
        let register = vm.get_register(opcode.src0_index);
        let result = get_forward_memory_pointer(register.value, vm, register.is_pointer)?;
        vm.clear_registers();
        vm.set_register(1, TaggedValue::new_pointer(FatPointer::encode(&result)));
        let previous_frame = vm.pop_frame()?;
        vm.current_frame_mut()?.pc = previous_frame.exception_handler;
        vm.current_frame_mut()?.gas_left += previous_frame.gas_left;
        Ok(false)
    } else {
        perform_return(vm, opcode)?;
        Ok(true)
    }
}
