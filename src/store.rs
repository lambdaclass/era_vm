use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::{collections::HashMap, fmt::Debug};
use thiserror::Error;
use u256::{H160, U256};
use zkevm_opcode_defs::{
    ethereum_types::Address, system_params::DEPLOYER_SYSTEM_CONTRACT_ADDRESS_LOW,
};

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
    l1_to_l2_logs: Vec<L2ToL1Log>,
}

impl StateStorage {
    pub fn new(initial_storage: Rc<RefCell<dyn InitialStorage>>) -> Self {
        Self {
            storage_changes: HashMap::new(),
            initial_storage,
            l1_to_l2_logs: Vec::new(),
        }
    }

    pub fn storage_read(&self, key: StorageKey) -> Result<Option<U256>, StorageError> {
        let mut value = self.storage_changes.get(&key).copied();
        if value.is_none() {
            value = self.initial_storage.borrow().storage_read(key)?;
        }
        Ok(value)
    }

    pub fn storage_write(&mut self, key: StorageKey, value: U256) -> Result<(), StorageError> {
        self.storage_changes.insert(key, value);
        Ok(())
    }

    pub fn record_l2_to_l1_log(&mut self, msg: L2ToL1Log) -> Result<(), StorageError> {
        self.l1_to_l2_logs.push(msg);
        Ok(())
    }

    pub fn create_snapshot(&self) -> SnapShot {
        SnapShot {
            storage_changes: self.storage_changes.clone(),
        }
    }

    pub fn rollback(&mut self, snapshot: &SnapShot ) {
        let keys = snapshot.storage_changes.keys();
        for key in keys {
            let value = snapshot.storage_changes.get(key);
            if let Some(value) = value {
                self.storage_write(*key, *value).unwrap();
            }
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
pub fn initial_decommit(initial_storage: &dyn InitialStorage, contract_storage: &dyn ContractStorage, address: H160) -> Vec<U256> {
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

    contract_storage.decommit(code_key).unwrap().unwrap()
}



/// Trait for storage operations inside the VM, this will handle the sload and sstore opcodes.
/// This storage will handle the storage of a contract and the storage of the called contract.
pub trait Storage: Debug {
    fn decommit(&self, hash: U256) -> Result<Option<Vec<U256>>, StorageError>;
    fn add_contract(&mut self, hash: U256, code: Vec<U256>) -> Result<(), StorageError>;
    fn storage_read(&self, key: StorageKey) -> Result<Option<U256>, StorageError>;
    fn storage_write(&mut self, key: StorageKey, value: U256) -> Result<(), StorageError>;
    fn record_l2_to_l1_log(&mut self, msg: L2ToL1Log) -> Result<(), StorageError>;
    fn storage_drop(&mut self, key: StorageKey) -> Result<(), StorageError>;
    fn get_state_storage(&self) -> &HashMap<StorageKey, U256>;
    fn get_all_keys(&self) -> Vec<StorageKey>;
    fn fake_clone(&self) -> InMemory;
    fn rollback(&mut self, previous: &dyn Storage) {
        let keys = previous.get_all_keys();
        for key in keys {
            let value = previous.storage_read(key).unwrap();
            if let Some(value) = value {
                self.storage_write(key, value).unwrap();
            }
        }
        let current_keys = self.get_all_keys();
        for key in current_keys {
            let res = previous.storage_read(key);
            if let Ok(None) = res {
                self.storage_drop(key).unwrap();
            };
        }
    }
}



/// In-memory storage implementation.
#[derive(Debug, Clone, Default)]
pub struct InMemory {
    contract_storage: HashMap<U256, Vec<U256>>,
    pub state_storage: HashMap<StorageKey, U256>,
    l1_to_l2_logs: Vec<L2ToL1Log>,
}



impl InMemory {
    pub fn new_empty() -> Self {
        let state_storage = HashMap::new();
        let contract_storage = HashMap::new();
        let l1_to_l2_logs = Vec::new();
        InMemory {
            state_storage,
            contract_storage,
            l1_to_l2_logs,
        }
    }

    pub fn new(
        contract_storage: HashMap<U256, Vec<U256>>,
        state_storage: HashMap<StorageKey, U256>,
    ) -> Self {
        Self {
            contract_storage,
            state_storage,
            l1_to_l2_logs: Vec::new(),
        }
    }

    pub fn record_l2_to_l1_log(&mut self, msg: L2ToL1Log) -> Result<(), StorageError> {
        self.l1_to_l2_logs.push(msg);
        Ok(())
    }
}

impl Storage for InMemory {
    fn decommit(&self, hash: U256) -> Result<Option<Vec<U256>>, StorageError> {
        Ok(self.contract_storage.get(&hash).cloned())
    }

    fn add_contract(&mut self, hash: U256, code: Vec<U256>) -> Result<(), StorageError> {
        self.contract_storage.insert(hash, code);
        Ok(())
    }

    fn storage_read(&self, key: StorageKey) -> Result<Option<U256>, StorageError> {
        Ok(self.state_storage.get(&key).copied())
    }

    fn storage_write(&mut self, key: StorageKey, value: U256) -> Result<(), StorageError> {
        self.state_storage.insert(key, value);
        Ok(())
    }
    fn record_l2_to_l1_log(&mut self, msg: L2ToL1Log) -> Result<(), StorageError> {
        self.l1_to_l2_logs.push(msg);
        Ok(())
    }


    fn storage_drop(&mut self, key: StorageKey) -> Result<(), StorageError> {
        self.state_storage.remove(&key);
        Ok(())
    }

    fn get_state_storage(&self) -> &HashMap<StorageKey, U256> {
        &self.state_storage
    }

    fn get_all_keys(&self) -> Vec<StorageKey> {
        self.state_storage.keys().copied().collect()
    }
    fn fake_clone(&self) -> InMemory {
        self.clone()
    }
}



/// RocksDB storage implementation.
#[derive(Debug)]
pub struct RocksDB {
    db: rocksdb::DB,
}

/// Error type for database operations.
#[derive(Error, Debug)]
pub enum DBError {
    #[error("Error opening database")]
    OpenFailed,
}

pub enum RocksDBKey {
    /// Key that stores (contract_address, key) to value.
    ContractAddressValue(H160, U256),
    /// Key that maps a contract hash to its code
    HashToByteCode(U256),
    /// Key that stores a L2ToL1Log
    L2ToL1Log(L2ToL1Log),
}

impl RocksDBKey {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            RocksDBKey::ContractAddressValue(address, value) => {
                let mut encoded = Vec::new();
                encoded.extend(address.as_bytes());
                encoded.extend(encode(value));
                encoded
            }
            RocksDBKey::HashToByteCode(hash) => {
                let mut buff: [u8; 32] = [0; 32];
                hash.to_big_endian(&mut buff);
                buff.to_vec()
            }
            RocksDBKey::L2ToL1Log(log) => {
                let mut encoded = Vec::new();
                encoded.extend(encode(&log.key));
                encoded.extend(encode(&log.value));
                encoded.extend_from_slice(&[log.is_service as u8]);
                encoded.extend(log.address.as_bytes());
                encoded.push(log.shard_id);
                encoded.extend_from_slice(&log.tx_number.to_be_bytes());
                encoded
            }
        }
    }
}

impl RocksDB {
    /// Open a RocksDB database at the given path.
    pub fn open(path: PathBuf) -> Result<Self, DBError> {
        let mut open_options = rocksdb::Options::default();
        open_options.create_if_missing(true);
        let db = rocksdb::DB::open(&open_options, path).map_err(|_| DBError::OpenFailed)?;
        Ok(Self { db })
    }
}

impl Storage for RocksDB {
    fn decommit(&self, hash: U256) -> Result<Option<Vec<U256>>, StorageError> {
        let key = RocksDBKey::HashToByteCode(hash);
        let res = self
            .db
            .get(key.encode())
            .map_err(|_| StorageError::KeyNotPresent)?;
        Ok(res.map(|contract_code| contract_code.chunks_exact(32).map(U256::from).collect()))
    }

    fn add_contract(&mut self, hash: U256, code: Vec<U256>) -> Result<(), StorageError> {
        let key = RocksDBKey::HashToByteCode(hash);
        self.db
            .put(key.encode(), encode_contract(code))
            .map_err(|_| StorageError::ReadError)
    }

    fn storage_read(&self, key: StorageKey) -> Result<Option<U256>, StorageError> {
        let key = RocksDBKey::ContractAddressValue(key.address, key.key);
        let res = self
            .db
            .get(key.encode())
            .map_err(|_| StorageError::ReadError)?;
        match res {
            Some(result) => {
                let mut value = [0u8; 32];
                value.copy_from_slice(&result);
                Ok(Some(U256::from_big_endian(&value)))
            }
            None => Ok(None),
        }
    }

    fn storage_write(&mut self, key: StorageKey, value: U256) -> Result<(), StorageError> {
        let key = RocksDBKey::ContractAddressValue(key.address, key.key);
        self.db
            .put(key.encode(), encode(&value))
            .map_err(|_| StorageError::WriteError)
    }

    fn record_l2_to_l1_log(&mut self, msg: L2ToL1Log) -> Result<(), StorageError> {
        let key = RocksDBKey::L2ToL1Log(msg);
        self.db
            .put(key.encode(), vec![0])
            .map_err(|_| StorageError::WriteError)
    }
    fn storage_drop(&mut self, key: StorageKey) -> Result<(), StorageError> {
        let key = RocksDBKey::ContractAddressValue(key.address, key.key);
        self.db
            .delete(key.encode())
            .map_err(|_| StorageError::WriteError)
    }

    fn get_state_storage(&self) -> &HashMap<StorageKey, U256> {
        unimplemented!()
    }

    fn get_all_keys(&self) -> Vec<StorageKey> {
        let mut iter = self.db.raw_iterator();
        iter.seek_to_first();
        let mut keys = Vec::new();
        while iter.valid() {
            let key = iter.key().unwrap();
            let address = H160::from_slice(&key[..20]);
            let value = U256::from_big_endian(&key[20..]);
            keys.push(StorageKey::new(address, value));
            iter.next();
        }
        keys
    }
    fn fake_clone(&self) -> InMemory {
        unimplemented!()
    }
}

/// Encode a U256 into a byte vector to store and read from RocksDB.
pub fn encode(value: &U256) -> Vec<u8> {
    let mut encoded = Vec::new();
    for key in value.0.iter().rev() {
        let new_key = key.to_be_bytes().to_vec();
        encoded.extend(new_key);
    }
    encoded
}

pub fn encode_contract(contract: Vec<U256>) -> Vec<u8> {
    let mut encoded: Vec<u8> = vec![];
    for word in contract {
        let encoded_word = encode(&word);
        encoded.extend_from_slice(&encoded_word);
    }

    encoded
}

