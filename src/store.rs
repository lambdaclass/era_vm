use std::path::PathBuf;
use std::{collections::HashMap, fmt::Debug};
use thiserror::Error;
use u256::{H160, U256};
use zkevm_opcode_defs::{
    ethereum_types::Address, system_params::DEPLOYER_SYSTEM_CONTRACT_ADDRESS_LOW,
};

use crate::utils::address_into_u256;

/// Trait for storage operations inside the VM, this will handle the sload and sstore opcodes.
/// This storage will handle the storage of a contract and the storage of the called contract.
pub trait Storage: Debug {
    fn decommit(&self, hash: U256) -> Option<Vec<U256>>;
    fn add_contract(&mut self, hash: U256, code: Vec<U256>) -> Result<(), StorageError>;
    fn storage_read(&self, key: StorageKey) -> Option<U256>;
    fn storage_write(&mut self, key: StorageKey, value: U256) -> Result<(), StorageError>;
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

/// In-memory storage implementation.
#[derive(Debug, Clone, Default)]
pub struct InMemory {
    contract_storage: HashMap<U256, Vec<U256>>,
    state_storage: HashMap<StorageKey, U256>,
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

impl InMemory {
    pub fn new_empty() -> Self {
        let state_storage = HashMap::new();
        let contract_storage = HashMap::new();
        InMemory {
            state_storage,
            contract_storage,
        }
    }

    pub fn new(
        contract_storage: HashMap<U256, Vec<U256>>,
        state_storage: HashMap<StorageKey, U256>,
    ) -> Self {
        Self {
            contract_storage,
            state_storage,
        }
    }

    // fn fake_clone(&self) -> Result<InMemory, StorageError> {
    //     Ok(InMemory(self.0.clone()))
    // }
}

impl Storage for InMemory {
    fn decommit(&self, hash: U256) -> Option<Vec<U256>> {
        self.contract_storage.get(&hash).cloned()
    }

    fn add_contract(&mut self, hash: U256, code: Vec<U256>) -> Result<(), StorageError> {
        self.contract_storage.insert(hash, code);
        Ok(())
    }

    fn storage_read(&self, key: StorageKey) -> Option<U256> {
        self.state_storage.get(&key).copied()
    }

    fn storage_write(&mut self, key: StorageKey, value: U256) -> Result<(), StorageError> {
        self.state_storage.insert(key, value);
        Ok(())
    }
}

/// May be used to load code when the VM first starts up.
/// Doesn't check for any errors.
/// Doesn't cost anything but also doesn't make the code free in future decommits.
pub fn initial_decommit(storage: &mut dyn Storage, address: H160) -> Vec<U256> {
    let deployer_system_contract_address =
        Address::from_low_u64_be(DEPLOYER_SYSTEM_CONTRACT_ADDRESS_LOW as u64);
    let storage_key = StorageKey::new(deployer_system_contract_address, address_into_u256(address));
    let code_info = storage.storage_read(storage_key).unwrap_or_default();

    let mut code_info_bytes = [0; 32];
    code_info.to_big_endian(&mut code_info_bytes);

    code_info_bytes[1] = 0;
    let code_key: U256 = U256::from_big_endian(&code_info_bytes);

    storage.decommit(code_key).unwrap()
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
    fn decommit(&self, hash: U256) -> Option<Vec<U256>> {
        let key = RocksDBKey::HashToByteCode(hash);
        let res = self.db.get(key.encode()).unwrap();
        res.map(|contract_code| contract_code.chunks_exact(32).map(U256::from).collect())
    }

    fn add_contract(&mut self, hash: U256, code: Vec<U256>) -> Result<(), StorageError> {
        let key = RocksDBKey::HashToByteCode(hash);
        self.db
            .put(key.encode(), encode_contract(code))
            .map_err(|_| StorageError::ReadError)
    }

    fn storage_read(&self, key: StorageKey) -> Option<U256> {
        let key = RocksDBKey::ContractAddressValue(key.address, key.key);
        let res = self
            .db
            .get(key.encode())
            .map_err(|_| StorageError::ReadError)
            .unwrap();
        match res {
            Some(result) => {
                let mut value = [0u8; 32];
                value.copy_from_slice(&result);
                Some(U256::from_big_endian(&value))
            }
            None => None,
        }
    }

    fn storage_write(&mut self, key: StorageKey, value: U256) -> Result<(), StorageError> {
        let key = RocksDBKey::ContractAddressValue(key.address, key.key);
        self.db
            .put(key.encode(), encode(&value))
            .map_err(|_| StorageError::WriteError)
    }

    // fn fake_clone(&self) -> Result<InMemory, StorageError> {
    //     let mut new_storage = HashMap::new();
    //     {
    //         let iter = self.db.iterator(rocksdb::IteratorMode::Start);
    //         for result in iter {
    //             let (key, value) = result.map_err(|_| StorageError::ReadError)?;
    //             let mut key_u256 = [0u8; 32];
    //             key_u256.copy_from_slice(&key);
    //             let mut value_u256 = [0u8; 32];
    //             value_u256.copy_from_slice(&value);

    //             let real_key = U256::from_big_endian(&key_u256);
    //             let real_value = U256::from_big_endian(&value_u256);

    //             new_storage.insert(real_key, real_value);
    //         }
    //     }
    //     Ok(InMemory(new_storage))
    // }
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
