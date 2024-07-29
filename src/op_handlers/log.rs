use std::str::FromStr;

use u256::{H160, U256};

use crate::{
    eravm_error::EraVmError,
    state::VMState,
    store::{Storage, StorageKey},
    value::TaggedValue,
    Opcode,
};

pub fn storage_write(
    vm: &mut VMState,
    opcode: &Opcode,
    storage: &mut dyn Storage,
) -> Result<(), EraVmError> {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_frame()?.contract_address;
    let key = StorageKey::new(address, key_for_contract_storage);
    let value = vm.get_register(opcode.src1_index).value;
    dbg!("storage_write");
    dbg!(key);
    dbg!(value);
    if key.key == 1.into()
        && key.address == H160::from_str("0xe594ae1d7205e8e92fb22c59d040c31e1fcd139d").unwrap()
    {
        dbg!("storage_write");
        dbg!(key_for_contract_storage);
        dbg!(address);
        dbg!(key);
        dbg!(value);
    }
    storage.storage_write(key, value)?;
    Ok(())
}

pub fn storage_read(
    vm: &mut VMState,
    opcode: &Opcode,
    storage: &mut dyn Storage,
) -> Result<(), EraVmError> {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_frame()?.contract_address;
    let key = StorageKey::new(address, key_for_contract_storage);
    let value = if key.key == 1.into()
        && key.address == H160::from_str("0xe594ae1d7205e8e92fb22c59d040c31e1fcd139d").unwrap()
    {
        dbg!("hardcoding...");
        storage.storage_read(key)?.unwrap_or(U256::zero())
    } else {
        storage.storage_read(key)?.unwrap_or(U256::zero())
    };
    if key.key == 1.into()
        && key.address == H160::from_str("0xe594ae1d7205e8e92fb22c59d040c31e1fcd139d").unwrap()
    {
        dbg!("storage_read");
        dbg!(key_for_contract_storage);
        dbg!(address);
        dbg!(key);
        if storage.storage_read(key).unwrap().is_none() {
            dbg!("storage_read is none");
        }
        dbg!(value);
    }
    vm.set_register(opcode.dst0_index, TaggedValue::new_raw_integer(value));
    Ok(())
}

pub fn transient_storage_write(vm: &mut VMState, opcode: &Opcode) -> Result<(), EraVmError> {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_frame()?.contract_address;
    let key = StorageKey::new(address, key_for_contract_storage);
    let value = vm.get_register(opcode.src1_index).value;
    vm.current_frame_mut()?
        .transient_storage
        .storage_write(key, value)?;
    Ok(())
}

pub fn transient_storage_read(vm: &mut VMState, opcode: &Opcode) -> Result<(), EraVmError> {
    let key_for_contract_storage = vm.get_register(opcode.src0_index).value;
    let address = vm.current_frame()?.contract_address;
    let key = StorageKey::new(address, key_for_contract_storage);
    let value = vm
        .current_frame()?
        .transient_storage
        .storage_read(key)?
        .unwrap_or(U256::zero());
    vm.set_register(opcode.dst0_index, TaggedValue::new_raw_integer(value));
    Ok(())
}
