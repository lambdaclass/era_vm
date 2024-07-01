use zkevm_opcode_defs::{ethereum_types::Address, FarCallOpcode};

use crate::state::VMState;

pub fn far_call(vm: &mut VMState, opcode: &FarCallOpcode) {
    match opcode {
        FarCallOpcode::Normal => {
            let program_code = vm.current_frame().code_page.clone();
            let stipend = vm.current_frame().gas_left;
            // TODO: Get values below from the opcode
            let address = Address::default();
            let caller = Address::default();
            vm.push_far_call_frame(program_code, stipend.0 / 32, address, caller);
        }
        _ => todo!(),
    }
}
