use std::cell::RefCell;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::{collections::HashMap, fmt::Debug};
use u256::{H160, U256};

/// Trait for storage operations inside the VM, this will handle the sload and sstore opcodes.
/// This storage will handle the storage of a contract and the storage of the called contract.
pub trait Storage: Debug {
    /// Store a key-value pair in the storage.
    /// The key is a tuple of the contract address and the key.
    /// The value is the value to be stored.
    fn store(&self, key: (H160, U256), value: U256) -> Result<(), StorageError>;
    /// Read a value from the storage.
    /// The key is a tuple of the contract address and the key.
    fn read(&self, key: &(H160, U256)) -> Result<U256, StorageError>;
    /// Get the contract hash from the address.
    fn get_contract_hash(&self, contract_address: &H160) -> Result<U256, StorageError>;
    fn get_contract_code(&self, contract_hash: &U256) -> Result<Vec<U256>, StorageError>;
    /// Given a contract hash, retrieve the byte code for it.
    /// This operation should return the contract code from a given contract hash.
    /// The contract hash should be previously stored.
    fn decommit(&self, contract_hash: &U256) -> Result<Vec<U256>, StorageError> {
        self.get_contract_code(contract_hash)
    }
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
    hash_to_code: RefCell<HashMap<U256, Vec<U256>>>,
    address_to_hash: RefCell<HashMap<H160, U256>>,
    contract_storage: RefCell<HashMap<H160, HashMap<U256, U256>>>,
}
impl InMemory {
    pub fn new_empty() -> Self {
        let hash_to_code = RefCell::from(HashMap::new());
        let address_to_hash = RefCell::from(HashMap::new());
        let contract_storage = RefCell::from(HashMap::new());
        InMemory {
            hash_to_code,
            address_to_hash,
            contract_storage,
        }
    }
}

impl Storage for InMemory {
    /// Store a key-value pair in the storage.
    fn store(&self, key: (H160, U256), value: U256) -> Result<(), StorageError> {
        let (contract_address, storage_key) = key;
        if let Some(contract_storage) = self
            .contract_storage
            .borrow_mut()
            .get_mut(&contract_address)
        {
            contract_storage.insert(storage_key, value);
            Ok(())
        } else {
            // TODO: Check the contract actually exists.
            let mut new_contract_storage = HashMap::new();
            new_contract_storage.insert(storage_key, value);
            self.contract_storage
                .borrow_mut()
                .insert(contract_address, new_contract_storage);
            Ok(())
        }
    }
    /// Read a value from the storage.
    fn read(&self, key: &(H160, U256)) -> Result<U256, StorageError> {
        let (contract_address, storage_key) = key;
        let contract_storage = self.contract_storage.borrow();
        let read_value = contract_storage
            .get(contract_address)
            .map(|contract_storage| contract_storage.get(storage_key))
            .flatten();

        match read_value {
            Some(&value) => Ok(value),
            None => Err(StorageError::KeyNotPresent),
        }
    }
    fn get_contract_code(&self, contract_hash: &U256) -> Result<Vec<U256>, StorageError> {
        match self.hash_to_code.borrow().get(contract_hash) {
            Some(code) => Ok(code.clone()),
            None => Err(StorageError::KeyNotPresent),
        }
    }
    fn get_contract_hash(&self, contract_address: &H160) -> Result<U256, StorageError> {
        match self.address_to_hash.borrow().get(contract_address) {
            Some(&hash) => Ok(hash),
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

pub enum RocksDBKey {
    /// Key that stores (contract_address, key) to value.
    ContractAddressValue(H160, U256),
    /// Key that maps an Address to a Hash
    AddressToHash(H160),
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
            RocksDBKey::AddressToHash(address) => address.to_fixed_bytes().to_vec(),
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
    /// Store a key-value pair in the storage.
    fn store(&self, key: (H160, U256), value: U256) -> Result<(), StorageError> {
        let key = RocksDBKey::ContractAddressValue(key.0, key.1);
        self.db
            .put(key.encode(), encode(&value))
            .map_err(|_| StorageError::WriteError)?;
        Ok(())
    }

    /// Read a value from the storage.
    fn read(&self, key: &(H160, U256)) -> Result<U256, StorageError> {
        let key = RocksDBKey::ContractAddressValue(key.0, key.1);
        let res = self
            .db
            .get(key.encode())
            .map_err(|_| StorageError::ReadError)?
            .ok_or(StorageError::KeyNotPresent)?;

        let mut value = [0u8; 32];
        value.copy_from_slice(&res);
        Ok(U256::from_big_endian(&value))
    }
    fn get_contract_hash(&self, contract_address: &H160) -> Result<U256, StorageError> {
        let key = RocksDBKey::AddressToHash(*contract_address);
        let res = self.db.get(key.encode());
        match res {
            Ok(Some(contract_hash)) => Ok(U256::from(&contract_hash[..])),
            Ok(None) => Err(StorageError::KeyNotPresent),
            Err(_) => Err(StorageError::ReadError),
        }
    }
    fn get_contract_code(&self, contract_hash: &U256) -> Result<Vec<U256>, StorageError> {
        let key = RocksDBKey::HashToByteCode(*contract_hash);
        let res = self.db.get(key.encode());
        match res {
            Ok(Some(contract_code)) => Ok(contract_code
                .chunks_exact(32)
                .map(|bytes| U256::from(bytes))
                .collect()),
            Ok(None) => Err(StorageError::KeyNotPresent),
            Err(_) => Err(StorageError::ReadError),
        }
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
