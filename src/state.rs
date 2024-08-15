use crate::{
    rollbacks::{
        Rollbackable, RollbackableHashMap, RollbackableHashSet, RollbackablePrimitive,
        RollbackableVec,
    },
    store::{Storage, StorageKey},
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use u256::{H160, U256};
use zkevm_opcode_defs::system_params::{
    STORAGE_ACCESS_COLD_READ_COST, STORAGE_ACCESS_COLD_WRITE_COST, STORAGE_ACCESS_WARM_READ_COST,
    STORAGE_ACCESS_WARM_WRITE_COST,
};

const WARM_READ_REFUND: u32 = STORAGE_ACCESS_COLD_READ_COST - STORAGE_ACCESS_WARM_READ_COST;
const WARM_WRITE_REFUND: u32 = STORAGE_ACCESS_COLD_WRITE_COST - STORAGE_ACCESS_WARM_WRITE_COST;
const COLD_WRITE_AFTER_WARM_READ_REFUND: u32 = STORAGE_ACCESS_COLD_READ_COST;

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
    pub storage: Rc<RefCell<dyn Storage>>,
    storage_changes: RollbackableHashMap<StorageKey, U256>,
    transient_storage: RollbackableHashMap<StorageKey, U256>,
    l2_to_l1_logs: RollbackableVec<L2ToL1Log>,
    events: RollbackableVec<Event>,
    // holds the sum of pubdata_costs
    pubdata: RollbackablePrimitive<i32>,
    pubdata_costs: RollbackableVec<i32>,
    paid_changes: RollbackableHashMap<StorageKey, u32>,
    refunds: RollbackableVec<u32>,

    // this fields don't get rollbacked on reverts(but the bootloader might)
    // that is why we add them as rollbackable as well
    read_storage_slots: RollbackableHashSet<StorageKey>,
    written_storage_slots: RollbackableHashSet<StorageKey>,
}

impl VMState {
    pub fn new(storage: Rc<RefCell<dyn Storage>>) -> Self {
        Self {
            storage,
            storage_changes: RollbackableHashMap::<StorageKey, U256>::default(),
            transient_storage: RollbackableHashMap::<StorageKey, U256>::default(),
            l2_to_l1_logs: RollbackableVec::<L2ToL1Log>::default(),
            events: RollbackableVec::<Event>::default(),
            pubdata: RollbackablePrimitive::<i32>::default(),
            pubdata_costs: RollbackableVec::<i32>::default(),
            paid_changes: RollbackableHashMap::<StorageKey, u32>::default(),
            refunds: RollbackableVec::<u32>::default(),
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

    pub fn pubdata_costs(&self) -> &Vec<i32> {
        &self.pubdata_costs.entries
    }

    pub fn pubdata(&self) -> i32 {
        self.pubdata.value
    }

    pub fn add_pubdata(&mut self, to_add: i32) {
        self.pubdata.value += to_add;
    }

    pub fn storage_read(&mut self, key: StorageKey) -> (U256, u32) {
        let value = self.storage_read_inner(&key).unwrap_or_default();
        let storage = self.storage.borrow();

        let refund =
            if storage.is_free_storage_slot(&key) || self.read_storage_slots.map.contains(&key) {
                WARM_READ_REFUND
            } else {
                self.read_storage_slots.map.insert(key);
                0
            };

        self.pubdata_costs.entries.push(0);

        (value, refund)
    }

    fn storage_read_inner(&self, key: &StorageKey) -> Option<U256> {
        match self.storage_changes.map.get(key) {
            None => self.storage.borrow_mut().storage_read(key),
            value => value.copied(),
        }
    }

    pub fn storage_write(&mut self, key: StorageKey, value: U256) -> u32 {
        self.storage_changes.map.insert(key, value);
        let mut storage = self.storage.borrow_mut();

        if storage.is_free_storage_slot(&key) {
            let refund = WARM_WRITE_REFUND;
            self.refunds.entries.push(refund);
            self.pubdata_costs.entries.push(0);
            return refund;
        }

        // the cost for writing storage is dynamic
        // after every write, we store the prepaid
        // on subsequent writes, we check if it has been already paid
        let cost = storage.cost_of_writing_storage(&key, value);
        let prepaid = *self.paid_changes.map.get(&key).unwrap_or(&0);
        self.paid_changes.map.insert(key, cost);

        let refund = if self.written_storage_slots.map.contains(&key) {
            WARM_WRITE_REFUND
        } else {
            self.written_storage_slots.map.insert(key);

            if self.read_storage_slots.map.contains(&key) {
                COLD_WRITE_AFTER_WARM_READ_REFUND
            } else {
                self.read_storage_slots.map.insert(key);
                0
            }
        };

        // note that this value can be negative
        // that is because the user might have paid for the write
        let pubdata_cost = (cost as i32) - (prepaid as i32);
        self.pubdata.value += pubdata_cost;
        self.refunds.entries.push(refund);
        self.pubdata_costs.entries.push(pubdata_cost);

        refund
    }

    pub fn transient_storage_read(&mut self, key: StorageKey) -> U256 {
        self.pubdata_costs.entries.push(0);
        self.transient_storage
            .map
            .get(&key)
            .copied()
            .unwrap_or_default()
    }

    pub fn transient_storage_write(&mut self, key: StorageKey, value: U256) {
        self.pubdata_costs.entries.push(0);
        self.transient_storage.map.insert(key, value);
    }

    pub fn record_l2_to_l1_log(&mut self, msg: L2ToL1Log) {
        self.l2_to_l1_logs.entries.push(msg);
    }

    pub fn record_event(&mut self, event: Event) {
        self.events.entries.push(event);
    }

    pub fn decommit(&mut self, hash: U256) -> Option<Vec<U256>> {
        self.storage.borrow_mut().decommit(hash)
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
    paid_changes: <RollbackableHashMap<StorageKey, u32> as Rollbackable>::Snapshot,
    refunds: <RollbackableVec<u32> as Rollbackable>::Snapshot,
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
            paid_changes: self.paid_changes.snapshot(),
            refunds: self.refunds.snapshot(),
        }
    }
}
