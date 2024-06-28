use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::{collections::HashMap, fmt::Debug};
use u256::{H160, U256};

use crate::decommit::address_into_u256;

/// Trait for storage operations inside the VM, this will handle the sload and sstore opcodes.
/// This storage will handle the storage of a contract and the storage of the called contract.
pub trait Storage: Debug {
    /// Store a key-value pair in the storage.
    /// The key is a tuple of the contract address and the key.
    /// The value is the value to be stored.
    fn store(&mut self, key: (H160, U256), value: U256) -> Result<(), StorageError>;
    /// Read a value from the storage.
    /// The key is a tuple of the contract address and the key.
    fn read(&self, key: &(H160, U256)) -> Result<U256, StorageError>;
}

/// This trait is used for the decommit operation.
/// The storage that implements this trait should include a map from contract hash to the contract code.
pub trait GlobalStorage {
    /// Get the contract hash from the address.
    fn read(&self, contract: &H160, key: &U256) -> Option<U256>;
    /// Decommit the contract code from the storage.
    /// This operation should return the contract code from a given contract hash.
    /// The contract hash should be previously stored.
    fn decommit(&mut self, contract_hash: &U256) -> U256;
}

/// Error type for storage operations.
#[derive(Debug)]
pub enum StorageError {
    KeyNotPresent,
    WriteError,
    ReadError,
}

/// In-memory storage implementation.
#[derive(Debug, Clone, Default)]
pub struct InMemory(HashMap<(H160, U256), U256>);

impl Storage for InMemory {
    /// Store a key-value pair in the storage.
    fn store(&mut self, key: (H160, U256), value: U256) -> Result<(), StorageError> {
        self.0.insert(key, value);
        Ok(())
    }

    /// Read a value from the storage.
    fn read(&self, key: &(H160, U256)) -> Result<U256, StorageError> {
        match self.0.get(key) {
            Some(value) => Ok(value.to_owned()),
            None => Err(StorageError::KeyNotPresent),
        }
    }
}

/// RocksDB storage implementation.
#[derive(Debug)]
pub struct RocksDB {
    db: rocksdb::DB,
}

/// Error type for database operations.
#[derive(Debug)]
pub enum DBError {
    OpenFailed(String),
}

pub enum DatabaseKey {
    /// Key that stores (contract_address, key) to value.
    ContractAddressValue(H160, U256),
}

impl DatabaseKey {
    pub fn encode(&self) -> Vec<u8> {
        match self {
            DatabaseKey::ContractAddressValue(address, value) => {
                let mut encoded = Vec::new();
                encoded.extend(address.as_bytes());
                encoded.extend(encode(value));
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
        let db = rocksdb::DB::open(&open_options, path)
            .map_err(|e| DBError::OpenFailed(e.to_string()))?;
        Ok(Self { db })
    }
}

impl Storage for RocksDB {
    /// Store a key-value pair in the storage.
    fn store(&mut self, key: (H160, U256), value: U256) -> Result<(), StorageError> {
        let key = DatabaseKey::ContractAddressValue(key.0, key.1);
        self.db
            .put(key.encode(), encode(&value))
            .map_err(|_| StorageError::WriteError)?;
        Ok(())
    }

    /// Read a value from the storage.
    fn read(&self, key: &(H160, U256)) -> Result<U256, StorageError> {
        let key = DatabaseKey::ContractAddressValue(key.0, key.1);
        let res = self
            .db
            .get(key.encode())
            .map_err(|_| StorageError::ReadError)?
            .ok_or(StorageError::KeyNotPresent)?;

        let mut value = [0u8; 32];
        value.copy_from_slice(&res);
        Ok(U256::from_big_endian(&value))
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

// This storage is used for testing purposes only.
// This global storage with the correct mapping should probably be elsewhere for a real implementation.
// To use it effectively, the contracts to call should be stored in this storage.

pub struct TestGlobalStorage {
    pub address_to_hash: HashMap<U256, U256>,
    pub hash_to_contract: HashMap<U256, U256>,
}

impl TestGlobalStorage {
    /// Open a RocksDB database at the given path.
    pub fn new(contracts: &[(H160, U256)]) -> Result<Self, DBError> {
        let mut address_to_hash = HashMap::new();
        let mut hash_to_contract = HashMap::new();

        for (i, (address, code)) in contracts.iter().enumerate() {
            // We add the index to the hash because tests may leave the code page blank.
            let mut hasher = DefaultHasher::new();
            i.hash(&mut hasher);
            code.hash(&mut hasher);

            let mut code_info_bytes = [0; 32];
            code_info_bytes[24..].copy_from_slice(&hasher.finish().to_be_bytes());
            // code_info_bytes[2..=3].copy_from_slice(&(code.len() as u16).to_be_bytes());
            code_info_bytes[0] = 1;
            let hash = U256::from_big_endian(&code_info_bytes);

            address_to_hash.insert(address_into_u256(*address), hash);
            hash_to_contract.insert(hash, code);
        }
        Ok(Self {
            address_to_hash: HashMap::new(),
            hash_to_contract: HashMap::new(),
        })
    }
}

impl GlobalStorage for TestGlobalStorage {
    /// Get the contract hash from the address.
    fn read(&self, _contract: &H160, key: &U256) -> Option<U256> {
        let res = *self.address_to_hash.get(key).unwrap();
        Some(res)
    }

    /// Decommit the contract code from the storage.
    fn decommit(&mut self, contract_hash: &U256) -> U256 {
        if let Some(program) = self.hash_to_contract.get(contract_hash) {
            *program
        } else {
            panic!("unexpected decommit")
        }
    }
}
