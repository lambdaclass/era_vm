use u256::U256;

use crate::address_operands::{address_operands_read, address_operands_store};
use crate::value::TaggedValue;
use crate::{opcode::Opcode, state::VMState};

pub fn _jump(vm: &mut VMState, opcode: &Opcode) {
    let (src0, _) = address_operands_read(vm, opcode);

    let next_pc = (src0.value & U256::from(u64::MAX)) - 1; // we subtract 1 because the pc will be incremented after this function
    vm.current_frame_mut().pc = next_pc.as_u64();
    address_operands_store(vm, opcode, TaggedValue::new_raw_integer(next_pc));
}
