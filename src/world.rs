use std::{cell::RefCell, collections::HashMap, rc::Rc};

use u256::U256;

use crate::store::{ContractStorage, InitialStorage, L2ToL1Log, StorageError, StorageKey};

#[derive(Debug)]
pub struct World {
    pub initial_storage: Rc<RefCell<dyn InitialStorage>>,
    pub contracts_storage: Rc<RefCell<dyn ContractStorage>>,
    pub storage_changes: RollbackableHashMap<StorageKey, U256>,
    pub transient_storage: RollbackableHashMap<StorageKey, U256>,
    pub l2_to_l1_logs: RollbackableVec<L2ToL1Log>,
}

impl World {
    pub fn new(
        initial_storage: Rc<RefCell<dyn InitialStorage>>,
        contracts_storage: Rc<RefCell<dyn ContractStorage>>,
    ) -> Self {
        Self {
            initial_storage,
            contracts_storage,
            l2_to_l1_logs: RollbackableVec::<L2ToL1Log>::default(),
            storage_changes: RollbackableHashMap::<StorageKey, U256>::default(),
            transient_storage: RollbackableHashMap::<StorageKey, U256>::default(),
        }
    }

    pub fn storage_read(&self, key: StorageKey) -> Result<Option<U256>, StorageError> {
        match self.storage_changes.map.get(&key) {
            None => self.initial_storage.borrow().storage_read(key),
            value => Ok(value.copied()),
        }
    }

    pub fn storage_write(&mut self, key: StorageKey, value: U256) -> Result<(), StorageError> {
        self.storage_changes.map.insert(key, value);
        Ok(())
    }
}

#[derive(Clone, Default, PartialEq, Debug)]
// a copy of rollbackable fields
pub struct WorldSnapshot {
    // this casts allows us to get the Snapshot type from the Rollbackable trait
    pub storage_changes: <RollbackableHashMap<StorageKey, U256> as Rollbackable>::Snapshot,
    pub transient_storage: <RollbackableHashMap<StorageKey, U256> as Rollbackable>::Snapshot,
    pub l2_to_l1_logs: <RollbackableVec<L2ToL1Log> as Rollbackable>::Snapshot,
}

pub trait Rollbackable {
    type Snapshot;
    fn rollback(&mut self, snapshot: Self::Snapshot);
    fn snapshot(&self) -> Self::Snapshot;
}

impl Rollbackable for World {
    type Snapshot = WorldSnapshot;

    fn rollback(&mut self, snapshot: Self::Snapshot) {
        self.storage_changes.rollback(snapshot.storage_changes);
        self.transient_storage.rollback(snapshot.transient_storage);
        self.l2_to_l1_logs.rollback(snapshot.l2_to_l1_logs);
    }

    fn snapshot(&self) -> Self::Snapshot {
        Self::Snapshot {
            l2_to_l1_logs: self.l2_to_l1_logs.snapshot(),
            storage_changes: self.storage_changes.snapshot(),
            transient_storage: self.transient_storage.snapshot(),
        }
    }
}

#[derive(Debug, Default)]
struct RollbackableHashMap<K: Clone, V: Clone> {
    map: HashMap<K, V>,
}

impl<K: Clone, V: Clone> Rollbackable for RollbackableHashMap<K, V> {
    type Snapshot = HashMap<K, V>;
    fn rollback(&mut self, snapshot: Self::Snapshot) {
        self.map = snapshot;
    }

    fn snapshot(&self) -> Self::Snapshot {
        self.map.clone()
    }
}

#[derive(Debug, Default)]
struct RollbackableVec<T: Clone> {
    entries: Vec<T>,
}

impl<T: Clone> Rollbackable for RollbackableVec<T> {
    type Snapshot = Vec<T>;

    fn rollback(&mut self, snapshot: Self::Snapshot) {
        self.entries = snapshot;
    }
    fn snapshot(&self) -> Self::Snapshot {
        self.entries.clone()
    }
}

#[derive(Debug, Default)]
struct RollbackablePrimitive<T: Copy> {
    value: T,
}

impl<T: Copy> Rollbackable for RollbackablePrimitive<T> {
    type Snapshot = T;
    fn rollback(&mut self, snapshot: Self::Snapshot) {
        self.value = snapshot;
    }

    fn snapshot(&self) -> Self::Snapshot {
        self.value
    }
}
