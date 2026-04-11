use std::collections::HashMap;

use crate::{
    ids::{IdAlloc, IdGenerator},
    vec_map::{VecMap, VecMapIndex},
};
use std::hash::Hash;

/// A bidirectional map that allows O(1) lookup from both keys and values.
///
/// This structure maintains two mappings: key → value and value → key.
/// Both directions support efficient lookup operations.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct BiMap<K: VecMapIndex, V: Hash + PartialEq + Eq> {
    key_to_value: VecMap<K, V>,
    value_to_key: HashMap<V, K>,
}
impl<K, V> BiMap<K, V>
where
    K: VecMapIndex + IdAlloc + Copy,
    V: Hash + Clone + PartialEq + Eq,
{
    /// Creates a new empty [`BiMap`].
    pub fn new() -> Self {
        Self {
            key_to_value: VecMap::new(),
            value_to_key: HashMap::new(),
        }
    }

    /// Clears all entries from both mappings.
    pub fn clear(&mut self) {
        self.key_to_value.clear();
        self.value_to_key.clear();
    }

    /// Creates a [`BiMap`] from a fixed-size array of key-value pairs.
    pub fn from_array<const N: usize>(vec: [(K, V); N]) -> Self {
        let mut this = Self::new();

        for (key, value) in vec {
            this.force_insert(key, value)
        }
        this
    }

    /// Inserts a value and allocates a new key, or returns an existing key if the value already exists.
    pub fn insert(&mut self, alloc: &mut IdGenerator<K>, value: V) -> K {
        match self.value_to_key.get(&value) {
            Some(id) => return *id,
            None => {
                let id = alloc.alloc();
                self.value_to_key.insert(value.clone(), id);
                self.key_to_value.insert(id, value);
                return id;
            }
        }
    }

    /// Forces insertion of a key-value pair, overwriting any existing entry for either the key or value.
    pub fn force_insert(&mut self, key: K, value: V) {
        self.key_to_value.insert(key, value.clone());
        self.value_to_key.insert(value, key);
    }

    /// Looks up a value by its key.
    pub fn get_value(&self, key: K) -> Option<&V> {
        self.key_to_value.get(key)
    }

    /// Looks up a key by its value.
    pub fn get_key(&self, value: &V) -> Option<K> {
        self.value_to_key.get(value).copied()
    }

    /// Returns an iterator over all key-value pairs.
    pub fn entries(&self) -> impl Iterator<Item = (K, &V)> {
        self.key_to_value.entries()
    }

    /// Consumes the map and returns an iterator over all key-value pairs.
    pub fn into_entries(self) -> impl IntoIterator<Item = (K, V)> {
        self.key_to_value.into_entries()
    }

    /// Returns an iterator over all keys.
    pub fn keys(&self) -> impl Iterator<Item = K> {
        self.key_to_value.keys()
    }
}
impl<K, V> Default for BiMap<K, V>
where
    K: VecMapIndex + IdAlloc + Copy,
    V: Hash + Clone + PartialEq + Eq,
{
    fn default() -> Self {
        Self {
            key_to_value: Default::default(),
            value_to_key: Default::default(),
        }
    }
}

impl<K, V> serde::Serialize for BiMap<K, V>
where
    K: serde::Serialize + VecMapIndex + IdAlloc + Copy,
    V: serde::Serialize + Hash + Clone + PartialEq + Eq,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.key_to_value.serialize(serializer)
    }
}
