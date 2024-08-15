use u256::U256;

use crate::{
    eravm_error::EraVmError,
    state::VMState,
    store::StorageKey,
    value::TaggedValue,
    world::{L2ToL1Log, World},
    Opcode,
};

pub fn storage_write(
    vm: &mut VMState,
    opcode: &Opcode,
    world: &mut World,
) -> Result<(), EraVmError> {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_context()?.contract_address;
    let key = StorageKey::new(address, key_for_contract_storage);
    let value = vm.get_register(opcode.src1_index).value;
    world.storage_write(key, value);
    Ok(())
}

pub fn storage_read(vm: &mut VMState, opcode: &Opcode, world: &World) -> Result<(), EraVmError> {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_context()?.contract_address;
    let key = StorageKey::new(address, key_for_contract_storage);
    let value = world.storage_read(key)?.unwrap_or(U256::zero());
    vm.set_register(opcode.dst0_index, TaggedValue::new_raw_integer(value));
    Ok(())
}

pub fn transient_storage_write(
    vm: &mut VMState,
    opcode: &Opcode,
    world: &mut World,
) -> Result<(), EraVmError> {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_context()?.contract_address;
    let key = StorageKey::new(address, key_for_contract_storage);
    let value = vm.get_register(opcode.src1_index).value;
    world.transient_storage_write(key, value);
    Ok(())
}

pub fn transient_storage_read(
    vm: &mut VMState,
    opcode: &Opcode,
    world: &World,
) -> Result<(), EraVmError> {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_context()?.contract_address;
    let key = StorageKey::new(address, key_for_contract_storage);
    let value = world.transient_storage_read(key)?.unwrap_or(U256::zero());
    vm.set_register(opcode.dst0_index, TaggedValue::new_raw_integer(value));
    Ok(())
}

pub fn add_l2_to_l1_message(
    vm_state: &mut VMState,
    opcode: &Opcode,
    world: &mut World,
) -> Result<(), EraVmError> {
    let key = vm_state.get_register(opcode.src0_index).value;
    let value = vm_state.get_register(opcode.src1_index).value;
    let is_service = opcode.imm0 == 1;
    world.record_l2_to_l1_log(L2ToL1Log {
        key,
        value,
        is_service,
        address: vm_state.current_context()?.contract_address,
        shard_id: 0,
        tx_number: vm_state.tx_number as u16,
    });
    Ok(())
}
