use std::path::PathBuf;
use std::{collections::HashMap, fmt::Debug};
use thiserror::Error;
use u256::U256;

/// Trait for different types of storage.
pub trait Storage: Debug {
    fn store(&mut self, key: U256, value: U256) -> Result<(), StorageError>;
    fn read(&self, key: &U256) -> Result<U256, StorageError>;
    fn fake_clone(&self) -> InMemory;
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
pub struct InMemory(pub HashMap<U256, U256>);
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

    fn fake_clone(&self) -> InMemory {
        InMemory(self.0.clone())
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
        self.db
            .put(encode(&key), encode(&value))
            .map_err(|_| StorageError::WriteError)?;
        Ok(())
    }

    /// Read a value from the storage.
    fn read(&self, key: &U256) -> Result<U256, StorageError> {
        let res = self
            .db
            .get(encode(key))
            .map_err(|_| StorageError::ReadError)?
            .ok_or(StorageError::KeyNotPresent)?;

        let mut value = [0u8; 32];
        value.copy_from_slice(&res);
        Ok(U256::from_big_endian(&value))
    }

    fn fake_clone(&self) -> InMemory {
        let mut new_storage = HashMap::new();
        {
            let iter = self.db.iterator(rocksdb::IteratorMode::Start);
            for result in iter {
                let (key, value) = result.unwrap();
                let mut key_u256 = [0u8; 32];
                key_u256.copy_from_slice(&key);
                let mut value_u256 = [0u8; 32];
                value_u256.copy_from_slice(&value);

                let real_key = U256::from_big_endian(&key_u256);
                let real_value = U256::from_big_endian(&value_u256);

                new_storage.insert(real_key, real_value);
            }
        }
        InMemory(new_storage)
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
