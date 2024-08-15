use crate::{
    rollbacks::{
        Rollbackable, RollbackableHashMap, RollbackableHashSet, RollbackablePrimitive,
        RollbackableVec,
    },
    store::{ContractStorage, InitialStorage, StorageError, StorageKey},
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use u256::{H160, U256};

#[derive(Debug, PartialEq, Clone, Default)]
pub struct L2ToL1Log {
    pub key: U256,
    pub value: U256,
    pub is_service: bool,
    pub address: H160,
    pub shard_id: u8,
    pub tx_number: u16,
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct Event {
    pub key: U256,
    pub value: U256,
    pub is_first: bool,
    pub shard_id: u8,
    pub tx_number: u16,
}

#[derive(Debug)]
pub struct VMState {
    pub initial_storage: Rc<RefCell<dyn InitialStorage>>,
    pub contracts_storage: Rc<RefCell<dyn ContractStorage>>,
    storage_changes: RollbackableHashMap<StorageKey, U256>,
    transient_storage: RollbackableHashMap<StorageKey, U256>,
    l2_to_l1_logs: RollbackableVec<L2ToL1Log>,
    events: RollbackableVec<Event>,
    pubdata: RollbackablePrimitive<i32>,
    pubdata_costs: RollbackableVec<i32>,
    storage_refunds: RollbackableVec<u32>,

    // this fields don't get rollbacked on reverts(but the bootloader might)
    // that is why we add them as rollbackable as well
    read_storage_slots: RollbackableHashSet<StorageKey>,
    written_storage_slots: RollbackableHashSet<StorageKey>,
}

impl VMState {
    pub fn new(
        initial_storage: Rc<RefCell<dyn InitialStorage>>,
        contracts_storage: Rc<RefCell<dyn ContractStorage>>,
    ) -> Self {
        Self {
            initial_storage,
            contracts_storage,
            storage_changes: RollbackableHashMap::<StorageKey, U256>::default(),
            transient_storage: RollbackableHashMap::<StorageKey, U256>::default(),
            l2_to_l1_logs: RollbackableVec::<L2ToL1Log>::default(),
            events: RollbackableVec::<Event>::default(),
            pubdata: RollbackablePrimitive::<i32>::default(),
            pubdata_costs: RollbackableVec::<i32>::default(),
            storage_refunds: RollbackableVec::<u32>::default(),
            read_storage_slots: RollbackableHashSet::<StorageKey>::default(),
            written_storage_slots: RollbackableHashSet::<StorageKey>::default(),
        }
    }

    pub fn storage_changes(&self) -> &HashMap<StorageKey, U256> {
        &self.storage_changes.map
    }

    pub fn transient_storage(&self) -> &HashMap<StorageKey, U256> {
        &self.transient_storage.map
    }

    pub fn l2_to_l1_logs(&self) -> &Vec<L2ToL1Log> {
        &self.l2_to_l1_logs.entries
    }

    pub fn events(&self) -> &Vec<Event> {
        &self.events.entries
    }

    pub fn storage_read(&self, key: StorageKey) -> Result<Option<U256>, StorageError> {
        match self.storage_changes.map.get(&key) {
            None => self.initial_storage.borrow().storage_read(key),
            value => Ok(value.copied()),
        }
    }

    pub fn storage_write(&mut self, key: StorageKey, value: U256) {
        self.storage_changes.map.insert(key, value);
    }

    pub fn transient_storage_read(&self, key: StorageKey) -> Result<Option<U256>, StorageError> {
        Ok(self.transient_storage.map.get(&key).copied())
    }

    pub fn transient_storage_write(&mut self, key: StorageKey, value: U256) {
        self.transient_storage.map.insert(key, value);
    }

    pub fn record_l2_to_l1_log(&mut self, msg: L2ToL1Log) {
        self.l2_to_l1_logs.entries.push(msg);
    }

    pub fn record_event(&mut self, event: Event) {
        self.events.entries.push(event);
    }

    pub fn decommit(&mut self, hash: U256) -> Result<Option<Vec<U256>>, StorageError> {
        self.contracts_storage.borrow().decommit(hash)
    }
}

#[derive(Clone, Default, PartialEq, Debug)]
// a copy of rollbackable fields
pub struct StateSnapshot {
    // this casts allows us to get the Snapshot type from the Rollbackable trait
    storage_changes: <RollbackableHashMap<StorageKey, U256> as Rollbackable>::Snapshot,
    transient_storage: <RollbackableHashMap<StorageKey, U256> as Rollbackable>::Snapshot,
    l2_to_l1_logs: <RollbackableVec<L2ToL1Log> as Rollbackable>::Snapshot,
    events: <RollbackableVec<Event> as Rollbackable>::Snapshot,
    pubdata: <RollbackablePrimitive<i32> as Rollbackable>::Snapshot,
    pubdata_costs: <RollbackableVec<i32> as Rollbackable>::Snapshot,
    storage_refunds: <RollbackableVec<u32> as Rollbackable>::Snapshot,
}

impl Rollbackable for VMState {
    type Snapshot = StateSnapshot;

    fn rollback(&mut self, snapshot: Self::Snapshot) {
        self.storage_changes.rollback(snapshot.storage_changes);
        self.transient_storage.rollback(snapshot.transient_storage);
        self.l2_to_l1_logs.rollback(snapshot.l2_to_l1_logs);
        self.events.rollback(snapshot.events)
    }

    fn snapshot(&self) -> Self::Snapshot {
        Self::Snapshot {
            l2_to_l1_logs: self.l2_to_l1_logs.snapshot(),
            storage_changes: self.storage_changes.snapshot(),
            transient_storage: self.transient_storage.snapshot(),
            events: self.events.snapshot(),
            pubdata: self.pubdata.snapshot(),
            pubdata_costs: self.pubdata_costs.snapshot(),
            storage_refunds: self.storage_refunds.snapshot(),
        }
    }
}
