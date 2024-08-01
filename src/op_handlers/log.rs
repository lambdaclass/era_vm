use u256::U256;

use crate::{
    eravm_error::EraVmError,
    state::VMState,
    store::{L2ToL1Log, Storage, StorageKey},
    value::TaggedValue,
    Opcode,
};

pub fn storage_write(
    vm: &mut VMState,
    opcode: &Opcode,
    storage: &mut dyn Storage,
) -> Result<(), EraVmError> {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_context()?.contract_address;
    let key = StorageKey::new(address, key_for_contract_storage);
    let value = vm.get_register(opcode.src1_index).value;
    storage.storage_write(key, value)?;
    Ok(())
}

pub fn storage_read(
    vm: &mut VMState,
    opcode: &Opcode,
    storage: &mut dyn Storage,
) -> Result<(), EraVmError> {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_context()?.contract_address;
    let key = StorageKey::new(address, key_for_contract_storage);
    let value = storage.storage_read(key)?.unwrap_or(U256::zero());
    vm.set_register(opcode.dst0_index, TaggedValue::new_raw_integer(value));
    Ok(())
}

pub fn transient_storage_write(vm: &mut VMState, opcode: &Opcode) -> Result<(), EraVmError> {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_context()?.contract_address;
    let key = StorageKey::new(address, key_for_contract_storage);
    let value = vm.get_register(opcode.src1_index).value;
    vm.current_frame_mut()?
        .transient_storage
        .storage_write(key, value)?;
    Ok(())
}

pub fn transient_storage_read(vm: &mut VMState, opcode: &Opcode) -> Result<(), EraVmError> {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_context()?.contract_address;
    let key = StorageKey::new(address, key_for_contract_storage);
    let value = vm
        .current_frame()?
        .transient_storage
        .storage_read(key)?
        .unwrap_or(U256::zero());
    vm.set_register(opcode.dst0_index, TaggedValue::new_raw_integer(value));
    Ok(())
}

pub fn add_l2_to_l1_message(
    vm_state: &mut VMState,
    opcode: &Opcode,
    storage: &mut dyn Storage,
) -> Result<(), EraVmError> {
    let key = vm_state.get_register(opcode.src0_index).value;
    let value = vm_state.get_register(opcode.src1_index).value;
    let is_service = opcode.imm1 == 1;
    storage.record_l2_to_l1_log(L2ToL1Log {
        key,
        value,
        is_service,
        address: vm_state.current_context()?.contract_address,
        shard_id: 0,
        tx_number: vm_state.tx_number as u16,
    })?;
    Ok(())
}
