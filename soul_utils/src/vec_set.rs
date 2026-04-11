use std::ops::Index;

use crate::vec_map::{VecMap, VecMapIndex};

/// A set-like data structure backed by a [`VecMap`].
///
/// Each unique index `I` maps to a presence marker `()`.
/// This provides O(1) lookup and set operations using ID types.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VecSet<I: VecMapIndex> {
    map: VecMap<I, ()>,
}
impl<I: VecMapIndex> VecSet<I> {
    /// Creates a new empty [`VecSet`].
    pub const fn new() -> Self {
        Self { map: VecMap::new() }
    }

    /// Creates a new [`VecSet`] with the given capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            map: VecMap::with_capacity(cap),
        }
    }

    /// Creates a [`VecSet`] from a [`VecMap`] with `()` values.
    pub fn from_vecmap(map: VecMap<I, ()>) -> Self {
        Self { map }
    }

    /// Creates a [`VecSet`] from a vector of indices.
    pub fn from_vec(keys: Vec<I>) -> Self {
        let max_index = keys.iter().map(|i| i.index()).max().unwrap_or(0);

        let mut this = Self::with_capacity(max_index);
        for index in keys {
            this.insert(index);
        }
        this
    }

    /// Creates a [`VecSet`] from a slice of indices.
    pub fn from_slice(keys: &[I]) -> Self {
        let max_index = keys.iter().map(|i| i.index()).max().unwrap_or(0);

        let mut this = Self::with_capacity(max_index);
        for key in keys {
            let index = I::new_index(key.index());
            this.insert(index);
        }
        this
    }

    /// Inserts an index into the set.
    ///
    /// Returns the previous value at that index, or `None` if it was empty.
    pub fn insert(&mut self, index: I) -> Option<()> {
        self.map.insert(index, ())
    }

    /// Returns the number of indices in the set.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Checks whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Checks whether the set contains the given index.
    pub fn contains(&self, index: I) -> bool {
        self.map.contains(index)
    }

    /// Removes and returns the index from the set, if present.
    pub fn remove(&mut self, index: I) -> Option<()> {
        self.map.remove(index)
    }

    /// Returns an iterator over all indices in the set.
    pub fn entries(&self) -> impl Iterator<Item = I> {
        self.map.keys()
    }
}
impl<I: VecMapIndex> Index<I> for VecSet<I> {
    type Output = bool;

    fn index(&self, index: I) -> &Self::Output {
        if self.map.contains(index) {
            &true
        } else {
            &false
        }
    }
}
impl<I: VecMapIndex> FromIterator<I> for VecSet<I> {
    fn from_iter<Iter: IntoIterator<Item = I>>(iter: Iter) -> Self {
        let mut set = Self::new();
        for i in iter {
            set.insert(i);
        }
        set
    }
}
impl<I: VecMapIndex> Default for VecSet<I> {
    fn default() -> Self {
        Self::new()
    }
}
