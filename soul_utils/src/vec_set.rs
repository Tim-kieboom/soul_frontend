use std::ops::Index;

use crate::vec_map::{VecMap, VecMapIndex};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VecSet<I: VecMapIndex> {
    map: VecMap<I, ()>,
}
impl<I: VecMapIndex> VecSet<I> {
    pub const fn new() -> Self {
        Self { map: VecMap::new() }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            map: VecMap::with_capacity(cap),
        }
    }

    pub fn from_vecmap(map: VecMap<I, ()>) -> Self {
        Self { map }
    }

    pub fn from_vec(keys: Vec<I>) -> Self {
        let max_index = keys.iter().map(|i| i.index()).max().unwrap_or(0);

        let mut this = Self::with_capacity(max_index);
        for index in keys {
            this.insert(index);
        }
        this
    }

    pub fn from_slice(keys: &[I]) -> Self {
        let max_index = keys.iter().map(|i| i.index()).max().unwrap_or(0);

        let mut this = Self::with_capacity(max_index);
        for key in keys {
            let index = I::new_index(key.index());
            this.insert(index);
        }
        this
    }

    pub fn insert(&mut self, index: I) -> Option<()> {
        self.map.insert(index, ())
    }

    pub fn contains(&self, index: I) -> bool {
        self.map.contains(index)
    }

    pub fn remove(&mut self, index: I) -> Option<()> {
        self.map.remove(index)
    }

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
