use crate::{
    eravm_error::EraVmError,
    execution::Execution,
    state::{L2ToL1Log, VMState},
    statistics::{
        VmStatistics, STORAGE_READ_STORAGE_APPLICATION_CYCLES,
        STORAGE_WRITE_STORAGE_APPLICATION_CYCLES,
    },
    store::{Storage, StorageKey},
    value::TaggedValue,
    Opcode,
};

pub fn storage_write(
    vm: &mut Execution,
    opcode: &Opcode,
    state: &mut VMState,
    statistics: &mut VmStatistics,
    storage: &mut dyn Storage,
) -> Result<(), EraVmError> {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_context()?.contract_address;
    let key = StorageKey::new(address, key_for_contract_storage);
    if !state.written_storage_slots().contains(&key) {
        statistics.storage_application_cycles += STORAGE_WRITE_STORAGE_APPLICATION_CYCLES;
    }
    let value = vm.get_register(opcode.src1_index).value;
    let refund = state.storage_write(key, value, storage);
    vm.increase_gas(refund)?;
    Ok(())
}

pub fn storage_read(
    vm: &mut Execution,
    opcode: &Opcode,
    state: &mut VMState,
    statistics: &mut VmStatistics,
    storage: &mut dyn Storage,
) -> Result<(), EraVmError> {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_context()?.contract_address;
    let key = StorageKey::new(address, key_for_contract_storage);
    // we need to check if it wasn't written as well
    // because when writing to storage, we need to read the slot as well
    if !state.read_storage_slots().contains(&key) && !state.written_storage_slots().contains(&key) {
        statistics.storage_application_cycles += STORAGE_READ_STORAGE_APPLICATION_CYCLES;
    }
    let (value, refund) = state.storage_read(key, storage);
    vm.increase_gas(refund)?;
    vm.set_register(opcode.dst0_index, TaggedValue::new_raw_integer(value));
    Ok(())
}

pub fn transient_storage_write(
    vm: &mut Execution,
    opcode: &Opcode,
    state: &mut VMState,
) -> Result<(), EraVmError> {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_context()?.contract_address;
    let key = StorageKey::new(address, key_for_contract_storage);
    let value = vm.get_register(opcode.src1_index).value;
    state.transient_storage_write(key, value);
    Ok(())
}

pub fn transient_storage_read(
    vm: &mut Execution,
    opcode: &Opcode,
    state: &mut VMState,
) -> Result<(), EraVmError> {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_context()?.contract_address;
    let key = StorageKey::new(address, key_for_contract_storage);
    let value = state.transient_storage_read(key);
    vm.set_register(opcode.dst0_index, TaggedValue::new_raw_integer(value));
    Ok(())
}

pub fn add_l2_to_l1_message(
    vm_state: &mut Execution,
    opcode: &Opcode,
    state: &mut VMState,
) -> Result<(), EraVmError> {
    let key = vm_state.get_register(opcode.src0_index).value;
    let value = vm_state.get_register(opcode.src1_index).value;
    let is_service = opcode.imm0 == 1;
    state.record_l2_to_l1_log(L2ToL1Log {
        key,
        value,
        is_service,
        address: vm_state.current_context()?.contract_address,
        shard_id: 0,
        tx_number: vm_state.tx_number as u16,
    });
    Ok(())
}
