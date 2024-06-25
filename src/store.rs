use std::path::PathBuf;
use std::{collections::HashMap, fmt::Debug};
use u256::{H160, U256};

/// Trait for different types of storage.
pub trait Storage: Debug {
    fn store(&mut self, key: (H160, U256), value: U256) -> Result<(), StorageError>;
    fn read(&self, key: &(H160, U256)) -> Result<U256, StorageError>;
}

/// This trait is used for the decommit operation.
/// The storage that implements this trait should include a map from contract hash to the contract code.
pub trait GlobalStorage {
    /// Get the contract hash from the address.
    fn get_contract_hash(&self, key: H160) -> U256;
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
pub struct InMemory {
    /// Map from (contract_address, key) to a stored value.
    contract_address_value: HashMap<(H160, U256), U256>,
    /// Map from contract address to its hash.
    address_to_hash: HashMap<H160, U256>,
    /// Map from contract hash to its code.
    hash_to_contract: HashMap<U256, U256>,
}

impl Storage for InMemory {
    /// Store a key-value pair in the storage.
    fn store(&mut self, key: (H160, U256), value: U256) -> Result<(), StorageError> {
        self.contract_address_value.insert(key, value);
        Ok(())
    }

    /// Read a value from the storage.
    fn read(&self, key: &(H160, U256)) -> Result<U256, StorageError> {
        match self.contract_address_value.get(key) {
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
    OpenFailed,
}

pub enum DatabaseKey {
    /// Key that stores (contract_address, key) to value.
    ContractAddressValue(H160, U256),
    /// Key that stores (contract_address) to its hash.
    AddressToHash(H160),
    /// Key that stores (contract_hash) to its code.
    HashToContract(U256),
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
            DatabaseKey::AddressToHash(address) => address.as_bytes().to_vec(),
            DatabaseKey::HashToContract(hash) => encode(hash),
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

// FIXME: This implementation might be temporary.
// This global storage with the correct mapping should probably be elsewhere.
// For testing purposes, we are using the same storage that the contracts use to store values to store the hash to contract code mapping.

impl GlobalStorage for InMemory {
    fn get_contract_hash(&self, key: H160) -> U256 {
        self.address_to_hash.get(&key).unwrap().to_owned()
    }
    fn decommit(&mut self, contract_hash: &U256) -> U256 {
        self.hash_to_contract.get(contract_hash).unwrap().to_owned()
    }
}

impl GlobalStorage for RocksDB {
    fn get_contract_hash(&self, key: H160) -> U256 {
        let key = DatabaseKey::AddressToHash(key);
        let res = self.db.get(key.encode()).unwrap().unwrap();
        U256::from_big_endian(&res)
    }
    fn decommit(&mut self, contract_hash: &U256) -> U256 {
        let key = DatabaseKey::HashToContract(contract_hash.clone());
        let code = self.db.get(key.encode()).unwrap().unwrap();
        U256::from_big_endian(&code)
    }
}
