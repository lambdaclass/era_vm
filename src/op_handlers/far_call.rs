use u256::U256;
use zkevm_opcode_defs::ethereum_types::Address;

use crate::{state::VMState, Opcode};

fn u256_to_address(integer: &U256) -> Address {
    let mut buffer = [0u8; 32];
    integer.to_big_endian(&mut buffer);

    Address::from_slice(&buffer[12..32])
}

pub fn far_call(vm: &mut VMState, opcode: &Opcode) {
    let program_code = vm.current_frame().code_page.clone();
    let stipend = vm.current_frame().gas_left;

    let address_mask: U256 = U256::MAX >> (256 - 160);
    let address = u256_to_address(&((vm.get_register(opcode.src1_index).value) & address_mask));
    let caller = vm.current_context().contract_address;
    vm.push_far_call_frame(program_code, stipend.0 / 32, address, caller);
}
