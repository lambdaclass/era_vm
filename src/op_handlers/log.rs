use crate::{
    eravm_error::EraVmError,
    execution::Execution,
    state::{L2ToL1Log, VMState},
    store::StorageKey,
    value::TaggedValue,
    Opcode,
};

pub fn storage_write(
    vm: &mut Execution,
    opcode: &Opcode,
    state: &mut VMState,
) -> Result<(), EraVmError> {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_context()?.contract_address;
    let key = StorageKey::new(address, key_for_contract_storage);
    let value = vm.get_register(opcode.src1_index).value;
    let refund = state.storage_write(key, value);
    vm.increase_gas(refund)?;
    Ok(())
}

pub fn storage_read(
    vm: &mut Execution,
    opcode: &Opcode,
    state: &mut VMState,
) -> Result<(), EraVmError> {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_context()?.contract_address;
    let key = StorageKey::new(address, key_for_contract_storage);
    let (value, refund) = state.storage_read(key);
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
