use u256::U256;

use crate::address_operands::{address_operands_read, address_operands_store};
use crate::eravm_error::EraVmError;
use crate::value::TaggedValue;
use crate::{opcode::Opcode, state::VMState};

pub fn jump(vm: &mut VMState, opcode: &Opcode) -> Result<(), EraVmError> {
    let (src0, _) = address_operands_read(vm, opcode)?;

    let next_pc = src0.value.low_u64();
    vm.current_frame_mut()?.pc = next_pc;
    address_operands_store(vm, opcode, TaggedValue::new_raw_integer(next_pc))
}
