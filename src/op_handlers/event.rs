use u256::H160;
use zkevm_opcode_defs::ADDRESS_EVENT_WRITER;

use crate::{
    eravm_error::EraVmError,
    execution::Execution,
    state::{Event, VMState},
    Opcode,
};

pub fn event(vm: &mut Execution, opcode: &Opcode, state: &mut VMState) -> Result<(), EraVmError> {
    if vm.current_context()?.contract_address == H160::from_low_u64_be(ADDRESS_EVENT_WRITER as u64)
    {
        let key = vm.get_register(opcode.src0_index).value;
        let value = vm.get_register(opcode.src1_index).value;
        let event = Event {
            key,
            value,
            is_first: opcode.flag0_set,
            shard_id: 1,
            tx_number: vm.tx_number as u16,
        };

        state.record_event(event);
    }
    Ok(())
}
