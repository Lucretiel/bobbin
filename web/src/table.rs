use std::{
    collections::{hash_map::Entry, HashMap},
    default::Default,
    hash::Hash,
    sync::Arc,
};

/// Helper struct for normalizing / deduplicating User objects. The idea is
/// that, since we're often receiving large sets of tweets from a single user,
/// we can save a lot of space by having all the Tweets have an Arc to a
/// single User instance.
#[derive(Debug)]
pub struct DedupeTable<K, V> {
    table: HashMap<K, Arc<V>>,
}

impl<K: Eq + Hash, V: Eq> DedupeTable<K, V> {
    pub fn new() -> Self {
        Self {
            table: HashMap::new(),
        }
    }

    pub fn dedup_item(&mut self, key: K, value: V) -> Arc<V> {
        match self.table.entry(key) {
            Entry::Occupied(mut entry) => {
                let existing = entry.get_mut();
                if **existing == value {
                    existing.clone()
                } else {
                    let replacement = Arc::new(value);
                    existing.clone_from(&replacement);
                    replacement
                }
            }
            Entry::Vacant(entry) => {
                let arc = Arc::new(value);
                entry.insert(arc.clone());
                arc
            }
        }
    }
}

impl<K: Eq + Hash, V: Eq> Default for DedupeTable<K, V> {
    fn default() -> Self {
        Self::new()
    }
}
