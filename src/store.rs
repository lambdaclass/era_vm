use std::path::PathBuf;
use std::{collections::HashMap, fmt::Debug};
use u256::{H160, U256};

/// Trait for storage operations inside the VM, this will handle the sload and sstore opcodes.
/// This storage will handle the storage of a contract and the storage of the called contract.
pub trait Storage: Debug {
    fn decommit(&self, hash: U256) -> Option<Vec<U256>>;
    fn add_contract(&mut self, hash: U256, code: Vec<U256>) -> Result<(), StorageError>;
    fn storage_read(&self, key: (H160, U256)) -> Option<U256>;
    fn storage_write(&mut self, key: (H160, U256), value: U256) -> Result<(), StorageError>;
}

/// Error type for storage operations.
#[derive(Debug)]
pub enum StorageError {
    WriteError,
    ReadError,
}

/// In-memory storage implementation.
#[derive(Debug, Clone, Default)]
pub struct InMemory {
    contract_storage: HashMap<U256, Vec<U256>>,
    state_storage: HashMap<(H160, U256), U256>,
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
}

impl Storage for InMemory {
    fn decommit(&self, hash: U256) -> Option<Vec<U256>> {
        self.contract_storage.get(&hash).cloned()
    }

    fn add_contract(&mut self, hash: U256, code: Vec<U256>) -> Result<(), StorageError> {
        self.contract_storage.insert(hash, code);
        Ok(())
    }

    fn storage_read(&self, key: (H160, U256)) -> Option<U256> {
        self.state_storage.get(&key).copied()
    }

    fn storage_write(&mut self, key: (H160, U256), value: U256) -> Result<(), StorageError> {
        self.state_storage.insert(key, value);
        Ok(())
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
        let db = rocksdb::DB::open(&open_options, path)
            .map_err(|e| DBError::OpenFailed(e.to_string()))?;
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

    fn storage_read(&self, key: (H160, U256)) -> Option<U256> {
        let key = RocksDBKey::ContractAddressValue(key.0, key.1);
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

    fn storage_write(&mut self, key: (H160, U256), value: U256) -> Result<(), StorageError> {
        let key = RocksDBKey::ContractAddressValue(key.0, key.1);
        self.db
            .put(key.encode(), encode(&value))
            .map_err(|_| StorageError::WriteError)
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
