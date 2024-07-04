use zkevm_opcode_defs::FarCallOpcode;

use crate::{eravm_error::EraVmError, state::VMState};

pub fn far_call(vm: &mut VMState, opcode: &FarCallOpcode) -> Result<(), EraVmError> {
    match opcode {
        FarCallOpcode::Normal => {
            let program_code = vm.current_frame()?.code_page.clone();
            let stipend = vm.current_frame()?.gas_left;
            vm.push_far_call_frame(program_code, stipend.0 / 32);
            Ok(())
        }
        _ => todo!(),
    }
}
