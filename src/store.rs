use std::path::PathBuf;
use std::{collections::HashMap, fmt::Debug};
use u256::U256;

/// Trait for different types of storage.
pub trait Storage: Debug {
    fn store(&mut self, key: U256, value: U256) -> Result<(), StorageError>;
    fn read(&self, key: &U256) -> Result<U256, StorageError>;
}

/// Error type for storage operations.
#[derive(Debug)]
pub enum StorageError {
    KeyNotPresent,
    WriteError,
    ReadError,
}

/// In-memory storage implementation.
#[derive(Debug, Clone)]
pub struct InMemory(HashMap<U256, U256>);

impl InMemory {
    /// Create a new in-memory storage.
    pub fn new() -> Self {
        Self(HashMap::new())
    }
}

impl Storage for InMemory {
    /// Store a key-value pair in the storage.
    fn store(&mut self, key: U256, value: U256) -> Result<(), StorageError> {
        self.0.insert(key, value);
        Ok(())
    }

    /// Read a value from the storage.
    fn read(&self, key: &U256) -> Result<U256, StorageError> {
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
    OpenFailed,
}

pub enum DatabaseKey {
    RegisterContent(U256),
}

impl DatabaseKey {
    /// Encode the key into a byte vector to store in the database.
    pub fn encode(&self) -> Vec<u8> {
        match self {
            DatabaseKey::RegisterContent(register) => {
                let mut encoded = Vec::new();
                for key in register.0.iter().rev() {
                    let new_key = key.to_be_bytes().to_vec();
                    encoded.extend(new_key);
                }
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
    /// Store a key-value pair in the storage.
    fn store(&mut self, key: U256, value: U256) -> Result<(), StorageError> {
        let key = DatabaseKey::RegisterContent(key);
        let mut encoded_value = Vec::new();
        for value in value.0.iter().rev() {
            let new_value = value.to_be_bytes().to_vec();
            encoded_value.extend(new_value);
        }
        self.db
            .put(key.encode(), encoded_value)
            .map_err(|_| StorageError::WriteError)?;
        Ok(())
    }

    /// Read a value from the storage.
    fn read(&self, key: &U256) -> Result<U256, StorageError> {
        let key = DatabaseKey::RegisterContent(*key);
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
