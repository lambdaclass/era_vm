use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

pub trait Rollbackable {
    type Snapshot;
    fn rollback(&mut self, snapshot: Self::Snapshot);
    fn snapshot(&self) -> Self::Snapshot;
}

#[derive(Debug, Default, Clone)]
pub struct RollbackableHashMap<K: Clone + Hash, V: Clone> {
    map: HashMap<K, V>,
}

impl<K: Clone + Hash + Eq, V: Clone> RollbackableHashMap<K, V> {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: K, value: V) {
        self.map.insert(key, value);
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key)
    }

    pub fn inner_ref(&self) -> &HashMap<K, V> {
        &self.map
    }

    pub fn get_logs_after_snapshot(
        &self,
        snapshot: <RollbackableHashMap<K, V> as Rollbackable>::Snapshot,
    ) -> HashMap<K, (Option<V>, V)> {
        let mut changes = HashMap::new();

        for (key, value) in self.map.iter() {
            changes.insert(key.clone(), (snapshot.get(key).cloned(), value.clone()));
        }

        changes
    }
}

impl<K: Clone + Hash, V: Clone> Rollbackable for RollbackableHashMap<K, V> {
    type Snapshot = HashMap<K, V>;
    fn rollback(&mut self, snapshot: Self::Snapshot) {
        self.map = snapshot;
    }

    fn snapshot(&self) -> Self::Snapshot {
        self.map.clone()
    }
}

impl<K: Clone + Hash, V: Clone> Iterator for RollbackableHashMap<K, V> {
    type Item = (K, V);
    fn next(&mut self) -> Option<Self::Item> {
        self.map.iter().next().map(|(k, v)| (k.clone(), v.clone()))
    }
}

#[derive(Debug, Default, Clone)]
pub struct RollbackableVec<T: Clone> {
    entries: Vec<T>,
}

impl<T: Clone> RollbackableVec<T> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn push(&mut self, value: T) {
        self.entries.push(value);
    }

    pub fn entries(&self) -> &[T] {
        &self.entries
    }

    pub fn get_logs_after_snapshot(
        &self,
        snapshot: <RollbackableVec<T> as Rollbackable>::Snapshot,
    ) -> &[T] {
        &self.entries[snapshot..]
    }
}

impl<T: Clone> Rollbackable for RollbackableVec<T> {
    // here, we can avoid cloning and we can just store the length since we never pop entries
    // and we always push at the end
    type Snapshot = usize;

    fn rollback(&mut self, snapshot: Self::Snapshot) {
        self.entries.truncate(snapshot);
    }

    fn snapshot(&self) -> Self::Snapshot {
        self.entries.len()
    }
}

#[derive(Debug, Default, Clone)]
pub struct RollbackablePrimitive<T: Copy> {
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

impl<T: Copy> RollbackablePrimitive<T> {
    pub fn new(value: T) -> Self {
        Self { value }
    }

    pub fn value(&self) -> T {
        self.value
    }

    pub fn set(&mut self, value: T) {
        self.value = value;
    }
}

#[derive(Debug, Default, Clone)]
pub struct RollbackableHashSet<K: Clone> {
    map: HashSet<K>,
}

impl<K: Clone> Rollbackable for RollbackableHashSet<K> {
    type Snapshot = HashSet<K>;
    fn rollback(&mut self, snapshot: Self::Snapshot) {
        self.map = snapshot;
    }

    fn snapshot(&self) -> Self::Snapshot {
        self.map.clone()
    }
}

impl<K: Clone + Eq + Hash> RollbackableHashSet<K> {
    pub fn insert(&mut self, value: K) -> bool {
        self.map.insert(value)
    }

    pub fn contains(&self, value: &K) -> bool {
        self.map.contains(value)
    }

    pub fn inner_ref(&self) -> &HashSet<K> {
        &self.map
    }
}
