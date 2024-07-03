use u256::U256;
use zkevm_opcode_defs::ethereum_types::Address;

use crate::{state::VMState, Opcode};

pub fn _far_call_normal(vm: &mut VMState, opcode: &Opcode) {
    let program_code = vm.current_frame().code_page.clone();
    let stipend = vm.current_frame().gas_left;

    // TODO: Get values below from the opcode
    let address_mask: U256 = U256::MAX >> (256 - 160);
    let address = vm.get_register(opcode.src1_index).value & address_mask;

    let address = Address::default();
    let caller = Address::default();
    vm.push_far_call_frame(program_code, stipend.0 / 32, address, caller);
}
