use std::{
    collections::{hash_map, HashMap},
    default::Default,
    hash::Hash,
    rc::Rc,
};

/// Helper struct for normalizing / deduplicating User objects. The idea is
/// that, since we're often receiving large sets of tweets from a single user,
/// we can save a lot of space by having all the Tweets have an `Rc` to a
/// single User instance.
#[derive(Debug)]
pub struct DedupeTable<K, V> {
    table: HashMap<K, Rc<V>>,
}

impl<K: Eq + Hash, V: Eq> DedupeTable<K, V> {
    /// Create a new, empty `DedupeTable`
    pub fn new() -> Self {
        Self {
            table: HashMap::new(),
        }
    }

    /// Attempt to deduplicate a value, based on a key. If another value
    /// with the same key exists in the table, and it compares equal to the
    /// incoming value, an `Rc` to the existing element is returned; otherwise,
    /// the value is replaced in the table, and an `Rc` to the new value is
    /// returned.
    pub fn dedup_item(&mut self, key: K, value: V) -> &Rc<V> {
        use hash_map::Entry::*;

        match self.table.entry(key) {
            Occupied(mut entry) => {
                let existing = entry.into_mut();

                if **existing != value {
                    *existing = Rc::new(value);
                }

                existing
            }
            Vacant(entry) => entry.insert(Rc::new(value)),
        }
    }

    pub fn get_item(&self, key: &K) -> Option<&Rc<V>> {
        self.table.get(key)
    }

    pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
        match self.table.entry(key) {
            hash_map::Entry::Occupied(value) => Entry::Occupied(value.into_mut()),
            hash_map::Entry::Vacant(slot) => Entry::Vacant(VacantEntry { inner: slot }),
        }
    }
}

#[derive(Debug)]
pub enum Entry<'a, K, V> {
    Occupied(&'a Rc<V>),
    Vacant(VacantEntry<'a, K, V>),
}

#[derive(Debug)]
pub struct VacantEntry<'a, K, V> {
    inner: hash_map::VacantEntry<'a, K, Rc<V>>,
}

impl<'a, K, V> VacantEntry<'a, K, V> {
    pub fn key(&self) -> &K {
        self.inner.key()
    }

    pub fn insert(self, value: V) -> &'a Rc<V> {
        self.inner.insert(Rc::new(value))
    }
}

impl<K: Eq + Hash, V: Eq> Default for DedupeTable<K, V> {
    fn default() -> Self {
        Self::new()
    }
}
