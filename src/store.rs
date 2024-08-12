use std::cell::RefCell;
use std::rc::Rc;
use std::{collections::HashMap, fmt::Debug};
use thiserror::Error;
use u256::{H160, U256};
use zkevm_opcode_defs::{
    ethereum_types::Address, system_params::DEPLOYER_SYSTEM_CONTRACT_ADDRESS_LOW,
};

use crate::eravm_error::EraVmError;
use crate::utils::address_into_u256;

#[derive(Debug, Clone)]
pub struct L2ToL1Log {
    pub key: U256,
    pub value: U256,
    pub is_service: bool,
    pub address: H160,
    pub shard_id: u8,
    pub tx_number: u16,
}

pub trait InitialStorage: Debug {
    fn storage_read(&self, key: StorageKey) -> Result<Option<U256>, StorageError>;
}

#[derive(Debug, Clone)]
pub struct InitialStorageMemory {
    pub initial_storage: HashMap<StorageKey, U256>,
}

// The initial storage acts as a read-only storage with initial values
// Any changes to the storage are stored in the state storage
// This specific implementation is just a simple way of doing it, so that the compiler tester can use it for testing
impl InitialStorage for InitialStorageMemory {
    fn storage_read(&self, key: StorageKey) -> Result<Option<U256>, StorageError> {
        Ok(self.initial_storage.get(&key).copied())
    }
}

pub trait ContractStorage: Debug {
    fn decommit(&self, hash: U256) -> Result<Option<Vec<U256>>, StorageError>;
}
#[derive(Debug)]
pub struct ContractStorageMemory {
    pub contract_storage: HashMap<U256, Vec<U256>>,
}

impl ContractStorage for ContractStorageMemory {
    fn decommit(&self, hash: U256) -> Result<Option<Vec<U256>>, StorageError> {
        Ok(self.contract_storage.get(&hash).cloned())
    }
}

#[derive(Debug)]
pub struct StateStorage {
    pub storage_changes: HashMap<StorageKey, U256>,
    pub initial_storage: Rc<RefCell<dyn InitialStorage>>,
    l2_to_l1_logs: Vec<L2ToL1Log>,
}

impl Default for StateStorage {
    fn default() -> Self {
        Self {
            storage_changes: HashMap::new(),
            initial_storage: Rc::new(RefCell::new(InitialStorageMemory {
                initial_storage: HashMap::new(),
            })),
            l2_to_l1_logs: Vec::new(),
        }
    }
}

impl StateStorage {
    pub fn new(initial_storage: Rc<RefCell<dyn InitialStorage>>) -> Self {
        Self {
            storage_changes: HashMap::new(),
            initial_storage,
            l2_to_l1_logs: Vec::new(),
        }
    }

    pub fn storage_read(&self, key: StorageKey) -> Result<Option<U256>, StorageError> {
        match self.storage_changes.get(&key) {
            None => self.initial_storage.borrow().storage_read(key),
            value => Ok(value.copied()),
        }
    }

    pub fn storage_write(&mut self, key: StorageKey, value: U256) -> Result<(), StorageError> {
        self.storage_changes.insert(key, value);
        Ok(())
    }

    pub fn record_l2_to_l1_log(&mut self, msg: L2ToL1Log) -> Result<(), StorageError> {
        self.l2_to_l1_logs.push(msg);
        Ok(())
    }

    pub fn create_snapshot(&self) -> SnapShot {
        SnapShot {
            storage_changes: self.storage_changes.clone(),
        }
    }

    pub fn rollback(&mut self, snapshot: &SnapShot) {
        let keys = snapshot.storage_changes.keys();
        for key in keys {
            snapshot
                .storage_changes
                .get(key)
                .map(|value| self.storage_write(*key, *value));
        }
        let current_keys = self.storage_changes.keys();
        let mut keys_to_remove = Vec::new();
        for key in current_keys {
            let res = snapshot.storage_changes.get(key);
            if res.is_none() {
                keys_to_remove.push(*key);
            };
        }
        for key in keys_to_remove {
            self.storage_changes.remove(&key);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SnapShot {
    pub storage_changes: HashMap<StorageKey, U256>,
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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct StorageKey {
    pub address: H160,
    pub key: U256,
}

impl StorageKey {
    pub fn new(address: H160, key: U256) -> Self {
        Self { address, key }
    }
}

/// May be used to load code when the VM first starts up.
/// Doesn't check for any errors.
/// Doesn't cost anything but also doesn't make the code free in future decommits.
pub fn initial_decommit(
    initial_storage: &dyn InitialStorage,
    contract_storage: &dyn ContractStorage,
    address: H160,
) -> Result<Vec<U256>, EraVmError> {
    let deployer_system_contract_address =
        Address::from_low_u64_be(DEPLOYER_SYSTEM_CONTRACT_ADDRESS_LOW as u64);
    let storage_key = StorageKey::new(deployer_system_contract_address, address_into_u256(address));
    let code_info = initial_storage
        .storage_read(storage_key)
        .unwrap()
        .unwrap_or_default();

    let mut code_info_bytes = [0; 32];
    code_info.to_big_endian(&mut code_info_bytes);

    code_info_bytes[1] = 0;
    let code_key: U256 = U256::from_big_endian(&code_info_bytes);

    let code = contract_storage.decommit(code_key)?;
    match code {
        Some(code) => Ok(code),
        None => Err(EraVmError::StorageError(StorageError::KeyNotPresent)),
    }
}
