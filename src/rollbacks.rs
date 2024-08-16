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
    pub map: HashMap<K, V>,
}

impl<K: Clone + Hash, V: Clone> RollbackableHashMap<K, V> {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
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
    pub entries: Vec<T>,
}

impl<T: Clone> RollbackableVec<T> {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
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

#[derive(Debug, Default, Clone)]
pub struct RollbackablePrimitive<T: Copy> {
    pub value: T,
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

#[derive(Debug, Default, Clone)]
pub struct RollbackableHashSet<K: Clone> {
    pub map: HashSet<K>,
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
