use crate::{
    rollbacks::{
        Rollbackable, RollbackableHashMap, RollbackableHashSet, RollbackablePrimitive,
        RollbackableVec,
    },
    store::{Storage, StorageKey},
};
use std::collections::{HashMap, HashSet};
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

#[derive(Debug, Clone)]
pub struct VMState {
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
    decommitted_hashes: RollbackableHashSet<U256>,

    // Just a cache so that we don't query the db every time
    pub initial_values: HashMap<StorageKey, Option<U256>>,
}

impl VMState {
    pub fn new() -> Self {
        Self {
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
            decommitted_hashes: RollbackableHashSet::<U256>::default(),
            initial_values: HashMap::default(),
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
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

    pub fn get_l2_to_l1_logs_after_snapshot(
        &self,
        snapshot: <RollbackableVec<L2ToL1Log> as Rollbackable>::Snapshot,
    ) -> &[L2ToL1Log] {
        self.l2_to_l1_logs.get_logs_after_snapshot(snapshot)
    }

    pub fn events(&self) -> &Vec<Event> {
        &self.events.entries
    }

    pub fn get_events_after_snapshot(
        &self,
        snapshot: <RollbackableVec<Event> as Rollbackable>::Snapshot,
    ) -> &[Event] {
        self.events.get_logs_after_snapshot(snapshot)
    }

    pub fn refunds(&self) -> &Vec<u32> {
        &self.refunds.entries
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

    pub fn storage_read(&mut self, key: StorageKey, storage: &mut dyn Storage) -> (U256, u32) {
        let value = self.storage_read_inner(&key, storage).unwrap_or_default();

        let refund =
            if storage.is_free_storage_slot(&key) || self.read_storage_slots.map.contains(&key) {
                WARM_READ_REFUND
            } else {
                self.read_storage_slots.map.insert(key);
                0
            };

        self.pubdata_costs.entries.push(0);
        self.refunds.entries.push(refund);

        (value, refund)
    }

    pub fn storage_read_with_no_refund(
        &mut self,
        key: StorageKey,
        storage: &mut dyn Storage,
    ) -> U256 {
        let value = self.storage_read_inner(&key, storage).unwrap_or_default();

        if !storage.is_free_storage_slot(&key) && !self.read_storage_slots.map.contains(&key) {
            self.read_storage_slots.map.insert(key);
        };

        self.pubdata_costs.entries.push(0);

        value
    }

    fn storage_read_inner(&mut self, key: &StorageKey, storage: &mut dyn Storage) -> Option<U256> {
        match self.storage_changes.map.get(key) {
            None => {
                let cached_value = *self.initial_values.get(key).unwrap_or(&None);
                if cached_value.is_none() {
                    let value = storage.storage_read(key);
                    self.initial_values.insert(*key, value);
                    return value;
                }
                cached_value
            }
            value => value.copied(),
        }
    }

    pub fn storage_write(
        &mut self,
        key: StorageKey,
        value: U256,
        storage: &mut dyn Storage,
    ) -> u32 {
        self.storage_changes.map.insert(key, value);

        self.initial_values
            .entry(key)
            .or_insert_with(|| storage.storage_read(&key)); // caching the value

        if storage.is_free_storage_slot(&key) {
            let refund = WARM_WRITE_REFUND;
            self.refunds.entries.push(refund);
            self.pubdata_costs.entries.push(0);
            return refund;
        }

        // after every write, we store the current cost paid
        // on subsequent writes, we don't charge for what has already been paid
        // but for the newer price, which if it is lower might end up in a refund
        let current_cost = storage.cost_of_writing_storage(&key, value);
        let prev_cost = *self.paid_changes.map.get(&key).unwrap_or(&0);
        self.paid_changes.map.insert(key, current_cost);

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

        // Note, that the diff may be negative, e.g. in case the new write returns to the original value.
        // The end result is that users pay as much pubdata in total as would have been required to set
        // the slots to their final values.
        // The only case where users may overpay is when a previous transaction ends up with a negative pubdata total.
        let pubdata_cost = (current_cost as i32) - (prev_cost as i32);
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

    pub(crate) fn clear_transient_storage(&mut self) {
        self.transient_storage = RollbackableHashMap::default();
    }

    pub fn record_l2_to_l1_log(&mut self, msg: L2ToL1Log) {
        self.l2_to_l1_logs.entries.push(msg);
    }

    pub fn record_event(&mut self, event: Event) {
        self.events.entries.push(event);
    }

    pub fn decommit(&mut self, hash: U256, storage: &mut dyn Storage) -> (Option<Vec<U256>>, bool) {
        let was_decommitted = !self.decommitted_hashes.map.insert(hash);
        (storage.decommit(hash), was_decommitted)
    }

    pub fn decommitted_hashes(&self) -> &HashSet<U256> {
        &self.decommitted_hashes.map
    }

    /// returns the values that have actually changed from the initial storage
    pub fn get_storage_changes(&self) -> Vec<(StorageKey, Option<U256>, U256)> {
        self.storage_changes
            .map
            .iter()
            .filter_map(|(key, &value)| {
                let initial_value = self.initial_values.get(key).unwrap_or(&None);
                if initial_value.unwrap_or_default() == value {
                    None
                } else {
                    Some((*key, *initial_value, value))
                }
            })
            .collect()
    }

    /// returns the values that have actually changed from the snapshot storage or the initial if the former does not exist
    /// also, a flag is also retured indicating if the value existed on the initial storage
    pub fn get_storage_changes_from_snapshot(
        &self,
        snapshot: <RollbackableHashMap<StorageKey, U256> as Rollbackable>::Snapshot,
    ) -> Vec<(StorageKey, Option<U256>, U256, bool)> {
        self.storage_changes
            .get_logs_after_snapshot(snapshot)
            .iter()
            .map(|(key, (before, after))| {
                let initial = self.initial_values.get(key).unwrap_or(&None);
                (*key, before.or(*initial), *after, initial.is_none())
            })
            .collect()
    }
}

#[derive(Clone, Default, PartialEq, Debug)]
// a copy of rollbackable fields
pub struct StateSnapshot {
    // this casts allows us to get the Snapshot type from the Rollbackable trait
    pub storage_changes: <RollbackableHashMap<StorageKey, U256> as Rollbackable>::Snapshot,
    pub transient_storage: <RollbackableHashMap<StorageKey, U256> as Rollbackable>::Snapshot,
    pub l2_to_l1_logs: <RollbackableVec<L2ToL1Log> as Rollbackable>::Snapshot,
    pub events: <RollbackableVec<Event> as Rollbackable>::Snapshot,
    pub pubdata: <RollbackablePrimitive<i32> as Rollbackable>::Snapshot,
    pub paid_changes: <RollbackableHashMap<StorageKey, u32> as Rollbackable>::Snapshot,
}

pub struct FullStateSnapshot {
    pub internal_snapshot: StateSnapshot,
    pub pubdata_costs: <RollbackableVec<i32> as Rollbackable>::Snapshot,
    pub refunds: <RollbackableVec<u32> as Rollbackable>::Snapshot,
    pub decommited_hashes: <RollbackableHashSet<U256> as Rollbackable>::Snapshot,
    pub read_storage_slots: <RollbackableHashSet<StorageKey> as Rollbackable>::Snapshot,
    pub written_storage_slots: <RollbackableHashSet<StorageKey> as Rollbackable>::Snapshot,
}

impl Rollbackable for VMState {
    type Snapshot = StateSnapshot;

    fn rollback(&mut self, snapshot: Self::Snapshot) {
        self.storage_changes.rollback(snapshot.storage_changes);
        self.transient_storage.rollback(snapshot.transient_storage);
        self.l2_to_l1_logs.rollback(snapshot.l2_to_l1_logs);
        self.events.rollback(snapshot.events);
        self.pubdata.rollback(snapshot.pubdata);
        self.paid_changes.rollback(snapshot.paid_changes);
    }

    fn snapshot(&self) -> Self::Snapshot {
        Self::Snapshot {
            l2_to_l1_logs: self.l2_to_l1_logs.snapshot(),
            storage_changes: self.storage_changes.snapshot(),
            transient_storage: self.transient_storage.snapshot(),
            events: self.events.snapshot(),
            pubdata: self.pubdata.snapshot(),
            paid_changes: self.paid_changes.snapshot(),
        }
    }
}

impl VMState {
    pub fn external_rollback(&mut self, snapshot: FullStateSnapshot) {
        self.rollback(snapshot.internal_snapshot);
        self.pubdata_costs.rollback(snapshot.pubdata_costs);
        self.refunds.rollback(snapshot.refunds);
        self.decommitted_hashes.rollback(snapshot.decommited_hashes);
        self.read_storage_slots
            .rollback(snapshot.read_storage_slots);
        self.written_storage_slots
            .rollback(snapshot.written_storage_slots);
    }

    pub fn full_state_snapshot(&self) -> FullStateSnapshot {
        FullStateSnapshot {
            internal_snapshot: self.snapshot(),
            decommited_hashes: self.decommitted_hashes.snapshot(),
            pubdata_costs: self.pubdata_costs.snapshot(),
            read_storage_slots: self.read_storage_slots.snapshot(),
            refunds: self.refunds.snapshot(),
            written_storage_slots: self.written_storage_slots.snapshot(),
        }
    }
}
