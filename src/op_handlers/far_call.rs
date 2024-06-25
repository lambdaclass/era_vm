use zkevm_opcode_defs::FarCallOpcode;

use crate::state::VMState;

pub fn far_call(vm: &mut VMState, opcode: &FarCallOpcode) {
    match opcode {
        FarCallOpcode::Normal => {
            let program_code = vm.current_context().code_page.clone();
            let stipend = vm.current_context().gas_left;
            vm.push_frame(program_code, stipend.0 / 32);
        }
        _ => todo!(),
    }
}
