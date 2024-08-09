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
