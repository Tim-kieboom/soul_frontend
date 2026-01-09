use std::marker::PhantomData;

pub trait AsIndex {
    fn new(value: usize) -> Self;
    fn index(&self) -> usize;
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VecMap<I: AsIndex, T> {
    pub vec: Vec<Option<T>>, 
    _marker: PhantomData<I>,
}
impl<I: AsIndex, T: Clone> VecMap<I, T> {
    pub fn from_slice(slice: &[(I, T)]) -> Self {
        let mut this = Self::new();
        for (index, value) in slice {
            this.raw_insert(index.index(), Some(value.clone()));
        }
        this
    }
}
impl<I: AsIndex, T> VecMap<I, T> {
    
    pub const fn new() -> Self {
        Self {
            vec: vec![],
            _marker: PhantomData,
        }
    }

    pub fn from_vec(vec: Vec<(I, T)>) -> Self {
        let mut this = Self::new();
        for (index, value) in vec {
            this.raw_insert(index.index(), Some(value));
        }
        this
    }

    pub fn contains(&self, index: I) -> bool {
        self.vec.get(index.index()).is_some_and(|el| el.is_some())
    }

    pub fn get(&self, index: I) -> Option<&T> {
        match self.vec.get(index.index()) {
            Some(Some(val)) => Some(val),
            _ => None,
        }
    }

    pub fn get_mut(&mut self, index: I) -> Option<&mut T> {
        match self.vec.get_mut(index.index()) {
            Some(Some(val)) => Some(val),
            _ => None,
        }
    }

    pub fn extend<Iter>(&mut self, vec: Iter) 
    where 
        Iter: Iterator<Item = (I, T)>
    {
        for (index, value) in vec {
            self.raw_insert(index.index(), Some(value));
        }
    }

    pub fn insert(&mut self, index: I, value: T) -> Option<T> {
        self.raw_insert(index.index(), Some(value))
    }

    pub fn remove(&mut self, index: I) -> Option<T> {
        self.raw_insert(index.index(), None)
    }

    fn raw_insert(&mut self, index: usize, mut value: Option<T>) -> Option<T> {
        if let Some(option_value) = self.vec.get_mut(index) {
            std::mem::swap(&mut value, option_value);
        }
        value
    }
}