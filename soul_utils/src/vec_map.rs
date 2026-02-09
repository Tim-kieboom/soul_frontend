use std::{marker::PhantomData, ops::{Index, IndexMut}};

/// A trait representing a type that can act as an index into a [`VecMap`].
///
/// Implementers of this trait define how to create a new index from a raw `usize`
/// and how to extract its underlying numeric value.
pub trait VecMapIndex {
    /// Constructs a new index from a raw `usize` value.
    fn new_index(value: usize) -> Self;

    /// Returns the numeric representation of this index.
    fn index(&self) -> usize;
}
impl VecMapIndex for usize {
    fn new_index(value: usize) -> Self {
        value
    }

    fn index(&self) -> usize {
        *self
    }
}

/// A sparse map-like data structure backed by a vector.
///
/// Each index `I` maps to an optional value `T`. This allows efficient indexing
/// by numeric-like types while maintaining the flexibility of a `HashMap`-like interface.
///
/// Internally, the structure stores a `Vec<Option<T>>`, expanding as needed.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VecMap<I: VecMapIndex, T> {
    pub vec: Vec<Option<T>>, 
    _marker: PhantomData<I>,
}
impl<I: VecMapIndex, T> VecMap<I, T> {

    pub const fn new() -> Self {
        Self::const_default()
    }

    pub const fn const_default() -> Self {
        Self {
            vec: vec![],
            _marker: PhantomData,
        }
    } 

    /// Creates a new [`VecMap`] with a given capacity of `cap`.
    ///
    /// The internal vector will contain `cap` empty (`None`) entries.
    pub fn with_capacity(cap: usize) -> Self {
        let mut vec = Vec::with_capacity(cap);
        vec.resize_with(cap, || None);

        Self { 
            vec, 
            _marker: PhantomData, 
        }
    }

    /// Inserts or replaces a value at the specified index.
    ///
    /// If the index exceeds the current vector length, the internal vector
    /// automatically grows to accommodate it.
    pub fn insert(&mut self, index: I, value: T) -> Option<T> {
        let index = index.index();
        if index >= self.vec.len() {
            self.vec.resize_with(index + 1, || None);
        }
        self.vec[index].replace(value)
    }

    pub fn clear(&mut self) {
        self.vec.clear();
    }

    /// Returns the current length of the internal vector (including `None` entries).
    pub fn cap(&self) -> usize {
        self.vec.len()
    }

    /// returns amount of entries 
    pub fn len(&self) -> usize {
        self.vec.iter().filter(|o| o.is_some()).count()
    }
    
    /// Creates a [`VecMap`] from a vector of index-value pairs.
    pub fn from_vec(entries: Vec<(I, T)>) -> Self {
        let max_index = entries.iter()
            .map(|(i, _)| i.index())
            .max()
            .unwrap_or(0);
        
        let mut this = Self::with_capacity(max_index);
        for (index, value) in entries {
            this.insert(index, value);
        }
        this
    }

    /// Checks whether the given index `I` currently contains a value.
    pub fn contains(&self, index: I) -> bool {
        self.vec.get(index.index()).is_some_and(|el| el.is_some())
    }

    /// Returns a reference to the value at the given index, if present.
    pub fn get(&self, index: I) -> Option<&T> {
        match self.vec.get(index.index()) {
            Some(Some(val)) => Some(val),
            _ => None,
        }
    }

    /// Returns a mutable reference to the value at the given index, if present.
    pub fn get_mut(&mut self, index: I) -> Option<&mut T> {
        match self.vec.get_mut(index.index()) {
            Some(Some(val)) => Some(val),
            _ => None,
        }
    }

    /// Extends this map by inserting multiple `(index, value)` pairs from an iterator.
    pub fn extend<Iter>(&mut self, vec: Iter) 
    where 
        Iter: Iterator<Item = (I, T)>
    {
        for (index, value) in vec {
            self.insert(index, value);
        }
    }

    /// Removes and returns the value at the specified index, if present.
    pub fn remove(&mut self, index: I) -> Option<T> {
        self.vec.get_mut(index.index()).and_then(|slot| slot.take())
    }  

    /// Returns an iterator over existing `index`.
    pub fn keys(&self) -> impl Iterator<Item = I> {
        self.vec.iter().enumerate().filter(|(_, el)| el.is_some()).map(|(i, _)| I::new_index(i))
    }

    /// Returns an iterator over existing `(index, &value)` pairs.
    pub fn entries(&self) -> impl Iterator<Item = (I, &T)> {
        self.vec.iter().enumerate().flat_map(|(i, el)| {
            el.as_ref().map(|val| (I::new_index(i), val))
        })
    }
    /// Consumes the map and returns an iterator over `(index, value)` pairs.
    pub fn into_entries(self) -> impl Iterator<Item = (I, T)> {
        self.vec.into_iter().enumerate().flat_map(|(i, el)| {
            el.map(|val| (I::new_index(i), val))
        })
    }

    /// Returns an iterator over references to all existing values.
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.vec.iter().flat_map(|el| el)
    }

    /// Consumes the map and returns an iterator over all values.
    pub fn into_values(self) -> impl Iterator<Item = T> {
        self.vec.into_iter().flat_map(|el| el)
    }
}
impl<I: VecMapIndex, T: PartialEq> PartialEq for VecMap<I, T> {
    fn eq(&self, other: &Self) -> bool {
        fn index_to_usize<I: VecMapIndex, T>(tuple: (I, &T)) -> (usize, &T) {
            (tuple.0.index(), tuple.1)
        }
        
        let self_iter = self.entries().map(index_to_usize);
        let other_iter = other.entries().map(index_to_usize);
        self_iter.eq(other_iter)
    }
}
impl<I: VecMapIndex, T: Eq> Eq for VecMap<I, T> {}
impl<I: VecMapIndex, T> Index<I> for VecMap<I, T> {
    type Output = T;

    fn index(&self, index: I) -> &Self::Output {
        self.vec[index.index()]
            .as_ref()
            .expect(&format!("entry in VecMap[{}] not found", index.index()))
    }
}
impl<I: VecMapIndex, T> IndexMut<I> for VecMap<I, T> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.vec[index.index()]
            .as_mut()
            .expect(&format!("entry in VecMap[{}] not found", index.index()))
    }
}
impl<I: VecMapIndex, T> FromIterator<(I, T)> for VecMap<I, T> {
    fn from_iter<Iter: IntoIterator<Item = (I, T)>>(iter: Iter) -> Self {
        let mut store = VecMap::new();
        for (index, value) in iter {
            store.insert(index, value);
        }
        store
    }
}
impl<I: VecMapIndex, T> Default for VecMap<I, T> {
    fn default() -> Self {
        Self::const_default()
    }
}
impl<I: VecMapIndex + Clone, T: Clone> VecMap<I, T> {
    /// Constructs a [`VecMap`] from a slice of index-value pairs.
    ///
    /// Each `(I, T)` pair is inserted into the map, resizing the underlying vector as needed.
    pub fn from_slice(slice: &[(I, T)]) -> Self {
        let mut this = Self::new();
        for (index, value) in slice.iter().cloned() {
            this.insert(index, value);
        }
        this
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VecSet<I: VecMapIndex> {
    map: VecMap<I, ()>
}
impl<I: VecMapIndex> VecSet<I> {
    pub const fn new() -> Self {
        Self { map: VecMap::new() }
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
impl<I: VecMapIndex + Clone> VecSet<I> {
    /// Constructs a [`VecSet`] from a slice of index-value pairs.
    ///
    /// Each `I` is inserted into the map, resizing the underlying vector as needed.
    pub fn from_slice(slice: &[I]) -> Self {
        let mut this = Self::new();
        for index in slice.iter().cloned() {
            this.insert(index);
        }
        this
    }
}