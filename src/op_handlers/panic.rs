use crate::{eravm_error::EraVmError, state::VMState, Opcode};

pub fn panic(vm: &mut VMState, opcode: &Opcode) -> Result<bool, EraVmError> {
    vm.flag_eq = false;
    vm.flag_lt_of = true;
    vm.flag_gt = false;
    if vm.in_near_call()? {
        let previous_frame = vm.pop_frame()?;
        if opcode.alters_vm_flags {
            // Marks if it has .to_label
            let to_label = opcode.imm0;
            vm.current_frame_mut()?.pc = (to_label - 1) as u64; // To account for the +1 later
        } else {
            vm.current_frame_mut()?.pc = previous_frame.exception_handler - 1;
            // To account for the +1 later
        }
        Ok(false)
    } else if vm.in_far_call() {
        vm.pop_frame()?;
        Ok(false)
    } else {
        Ok(true)
    }
}
