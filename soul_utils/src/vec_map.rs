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
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VecMap<I: VecMapIndex, T> {
    pub vec: Vec<Option<T>>, 
    _marker: PhantomData<I>,
}
impl<I: VecMapIndex, T> VecMap<I, T> {

    pub fn new() -> Self {
        Self::default()
    }

    pub const fn const_default() -> Self {
        Self {
            vec: vec![],
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

    /// Creates a new [`VecMap`] with a given capacity of `cap`.
    ///
    /// The internal vector will contain `cap` empty (`None`) entries.
    pub fn with_capacity(cap: usize) -> Self 
    where 
        T: Clone
    {
        Self { 
            vec: vec![None; cap], 
            _marker: PhantomData, 
        }
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
    pub fn from_vec(vec: Vec<(I, T)>) -> Self {
        let mut this = Self::new();
        for (index, value) in vec {
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

impl<I: VecMapIndex, T> Index<I> for VecMap<I, T> {
    type Output = T;

    fn index(&self, index: I) -> &Self::Output {
        self.vec[index.index()]
            .as_ref()
            .expect("expected value to be Some(_)")
    }
}
impl<I: VecMapIndex, T> IndexMut<I> for VecMap<I, T> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        self.vec[index.index()]
            .as_mut()
            .expect("expected value to be Some(_)")
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
    pub fn new() -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    // Simple index type for testing
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    struct TestIndex(usize);

    impl VecMapIndex for TestIndex {
        fn new_index(value: usize) -> Self {
            TestIndex(value)
        }

        fn index(&self) -> usize {
            self.0
        }
    }

    type TestMap<T> = VecMap<TestIndex, T>;

    #[test]
    fn test_new_and_default() {
        let map: TestMap<i32> = TestMap::new();
        assert_eq!(map.cap(), 0);
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn test_const_default() {
        let _map: TestMap<i32> = TestMap::const_default();
    }

    #[test]
    fn test_with_capacity() {
        let map: TestMap<i32> = TestMap::with_capacity(5);
        assert_eq!(map.cap(), 5);
        assert_eq!(map.len(), 0);
        for i in 0..5 {
            assert!(!map.contains(TestIndex(i)));
        }
    }

    #[test]
    fn test_insert_basic() {
        let mut map: TestMap<i32> = TestMap::new();
        assert_eq!(map.insert(TestIndex(0), 42), None);
        assert_eq!(map.cap(), 1);
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(TestIndex(0)), Some(&42));
    }

    #[test]
    fn test_insert_grows_vector() {
        let mut map: TestMap<i32> = TestMap::new();
        assert_eq!(map.insert(TestIndex(10), 100), None);
        assert_eq!(map.cap(), 11);
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(TestIndex(10)), Some(&100));
        assert_eq!(map.get(TestIndex(0)), None);
    }

    #[test]
    fn test_insert_overwrite() {
        let mut map: TestMap<i32> = TestMap::new();
        assert_eq!(map.insert(TestIndex(0), 42), None);
        assert_eq!(map.insert(TestIndex(0), 99), Some(42));
        assert_eq!(map.get(TestIndex(0)), Some(&99));
    }

    #[test]
    fn test_get_and_contains() {
        let mut map: TestMap<i32> = TestMap::new();
        map.insert(TestIndex(1), 10);
        map.insert(TestIndex(3), 30);

        assert!(!map.contains(TestIndex(0)));
        assert!(map.contains(TestIndex(1)));
        assert!(!map.contains(TestIndex(2)));
        assert!(map.contains(TestIndex(3)));

        assert_eq!(map.get(TestIndex(0)), None);
        assert_eq!(map.get(TestIndex(1)), Some(&10));
        assert_eq!(map.get(TestIndex(2)), None);
        assert_eq!(map.get(TestIndex(3)), Some(&30));
    }

    #[test]
    fn test_remove() {
        let mut map: TestMap<i32> = TestMap::new();
        map.insert(TestIndex(0), 42);
        map.insert(TestIndex(2), 24);

        assert_eq!(map.remove(TestIndex(1)), None);
        assert_eq!(map.len(), 2);

        assert_eq!(map.remove(TestIndex(0)), Some(42));
        assert_eq!(map.len(), 1);
        assert_eq!(map.get(TestIndex(0)), None);

        assert_eq!(map.remove(TestIndex(2)), Some(24));
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn test_from_slice() {
        let slice = vec![(TestIndex(5), "hello"), (TestIndex(0), "world")];
        let map = TestMap::from_slice(&slice);
        
        assert_eq!(map.cap(), 6);
        assert_eq!(map.len(), 2);
        assert_eq!(map.get(TestIndex(0)), Some(&"world"));
        assert_eq!(map.get(TestIndex(5)), Some(&"hello"));
    }

    #[test]
    fn test_from_vec() {
        let pairs = vec![(TestIndex(2), 200), (TestIndex(7), 700)];
        let map = TestMap::from_vec(pairs);
        
        assert_eq!(map.cap(), 8);
        assert_eq!(map.len(), 2);
        assert_eq!(map.get(TestIndex(2)), Some(&200));
        assert_eq!(map.get(TestIndex(7)), Some(&700));
    }

    #[test]
    fn test_extend() {
        let mut map: TestMap<i32> = TestMap::new();
        let extra = vec![(TestIndex(1), 10), (TestIndex(3), 30)];
        map.extend(extra.into_iter());
        
        assert_eq!(map.len(), 2);
        assert_eq!(map.get(TestIndex(1)), Some(&10));
        assert_eq!(map.get(TestIndex(3)), Some(&30));
    }

    #[test]
    fn test_keys_iterator() {
        let mut map: TestMap<i32> = TestMap::new();
        map.insert(TestIndex(0), 1);
        map.insert(TestIndex(2), 3);
        map.insert(TestIndex(2), 4); // overwrite

        let keys: Vec<_> = map.keys().collect();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&TestIndex(0)));
        assert!(keys.contains(&TestIndex(2)));
    }

    #[test]
    fn test_entries_iterator() {
        let mut map: TestMap<i32> = TestMap::new();
        map.insert(TestIndex(1), 10);
        map.insert(TestIndex(3), 30);

        let entries: Vec<_> = map.entries().collect();
        assert_eq!(entries.len(), 2);
        
        let mut found_10 = false;
        let mut found_30 = false;
        for (idx, val) in entries {
            if idx.index() == 1 && *val == 10 { found_10 = true; }
            if idx.index() == 3 && *val == 30 { found_30 = true; }
        }
        assert!(found_10);
        assert!(found_30);
    }

    #[test]
    fn test_into_entries() {
        let mut map: TestMap<i32> = TestMap::new();
        
        let expected = vec![(TestIndex(0), 42), (TestIndex(5), 24)];
        for (index, value) in expected.iter().copied() {
            map.insert(index, value);
        }

        let entries: Vec<_> = map.into_entries().collect();
        assert_eq!(expected, entries);
    }

    #[test]
    fn test_values_iterator() {
        let mut map: TestMap<String> = TestMap::new();
        map.insert(TestIndex(2), "foo".to_string());
        map.insert(TestIndex(4), "bar".to_string());

        let values: Vec<_> = map.values().collect();
        assert_eq!(values.len(), 2);
        assert!(values.iter().any(|s| *s == "foo"));
        assert!(values.iter().any(|s| *s == "bar"));
    }

    #[test]
    fn test_into_values() {
        let mut map: TestMap<i32> = TestMap::new();
        map.insert(TestIndex(1), 10);
        map.insert(TestIndex(3), 30);

        let values: Vec<i32> = map.into_values().collect();
        assert_eq!(values.len(), 2);
        assert!(values.contains(&10));
        assert!(values.contains(&30));
    }

    #[test]
    fn test_get_mut() {
        let mut map: TestMap<i32> = TestMap::new();
        map.insert(TestIndex(0), 42);

        if let Some(val) = map.get_mut(TestIndex(0)) {
            *val = 99;
        }
        assert_eq!(map.get(TestIndex(0)), Some(&99));
    }

    #[test]
    fn test_clone() {
        let mut original: TestMap<String> = TestMap::new();
        original.insert(TestIndex(10), "test".to_string());

        let cloned = original.clone();
        assert_eq!(original.cap(), cloned.cap());
        assert_eq!(original.len(), cloned.len());
        assert_eq!(original.get(TestIndex(10)), cloned.get(TestIndex(10)));
    }

    #[test]
    fn test_partial_eq() {
        let mut map1: TestMap<i32> = TestMap::new();
        map1.insert(TestIndex(0), 1);
        map1.insert(TestIndex(2), 3);

        let mut map2: TestMap<i32> = TestMap::new();
        map2.insert(TestIndex(0), 1);
        map2.insert(TestIndex(2), 3);

        assert_eq!(map1, map2);
    }
}
