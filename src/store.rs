use std::{collections::HashMap, fmt::Debug};
use thiserror::Error;
use u256::{H160, U256};
use zkevm_opcode_defs::{
    ethereum_types::Address, system_params::DEPLOYER_SYSTEM_CONTRACT_ADDRESS_LOW,
};

use crate::eravm_error::EraVmError;
use crate::utils::address_into_u256;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub struct StorageKey {
    pub address: H160,
    pub key: U256,
}

impl StorageKey {
    pub fn new(address: H160, key: U256) -> Self {
        Self { address, key }
    }
}

pub trait Storage {
    fn decommit(&mut self, hash: U256) -> Option<Vec<U256>>;

    fn storage_read(&self, key: StorageKey) -> Option<U256>;

    fn cost_of_writing_storage(&mut self, key: StorageKey, value: U256) -> u32;

    fn is_free_storage_slot(&self, key: &StorageKey) -> bool;
}

#[derive(Debug, Clone)]
pub struct InitialStorageMemory {
    pub contracts: HashMap<StorageKey, U256>,
    pub storage: HashMap<StorageKey, U256>,
}

// The initial storage acts as a read-only storage with initial values
// Any changes to the storage are stored in the state storage
// This specific implementation is just a simple way of doing it, so that the compiler tester can use it for testing
impl Storage for InitialStorageMemory {
    fn storage_read(&self, key: StorageKey) -> Option<U256> {
        Ok(self.initial_storage.get(&key).copied())
    }

    fn decommit(&mut self, hash: U256) -> Option<Vec<U256>> {}

    fn cost_of_writing_storage(&mut self, key: StorageKey, value: U256) -> u32 {}

    fn is_free_storage_slot(&self, key: &StorageKey) -> bool {}
}

/// Error type for storage operations.
#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Key not present in storage")]
    KeyNotPresent,
    #[error("Error writing to storage")]
    WriteError,
    #[error("Error reading from storage")]
    ReadError,
}

/// May be used to load code when the VM first starts up.
/// Doesn't check for any errors.
/// Doesn't cost anything but also doesn't make the code free in future decommits.
pub fn initial_decommit(
    storage: &dyn Storage,
    address: H160,
    evm_interpreter_code_hash: [u8; 32],
) -> Result<Vec<U256>, EraVmError> {
    let deployer_system_contract_address =
        Address::from_low_u64_be(DEPLOYER_SYSTEM_CONTRACT_ADDRESS_LOW as u64);
    let storage_key = StorageKey::new(deployer_system_contract_address, address_into_u256(address));
    let code_info = storage
        .storage_read(storage_key)
        .unwrap()
        .unwrap_or_default();

    let mut code_info_bytes = [0; 32];
    code_info.to_big_endian(&mut code_info_bytes);

    if code_info_bytes[0] == 2 {
        code_info_bytes = evm_interpreter_code_hash;
    }

    code_info_bytes[1] = 0;
    let code_key: U256 = U256::from_big_endian(&code_info_bytes);

    let code = storage.decommit(code_key)?;
    match code {
        Some(code) => Ok(code),
        None => Err(EraVmError::StorageError(StorageError::KeyNotPresent)),
    }
}
