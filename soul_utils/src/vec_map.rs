use std::{fmt::DebugList, marker::PhantomData};

pub trait AsIndex {
    fn new(value: usize) -> Self;
    fn index(&self) -> usize;
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct VecMap<I: AsIndex, T> {
    pub vec: Vec<Option<T>>, 
    _marker: PhantomData<I>,
}
impl<I: AsIndex + Clone, T: Clone> VecMap<I, T> {
    pub fn from_slice(slice: &[(I, T)]) -> Self {
        let mut this = Self::new();
        for (index, value) in slice.iter().cloned() {
            this.insert(index, value);
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
            this.insert(index, value);
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
            self.insert(index, value);
        }
    }

    pub fn insert(&mut self, index: I, value: T) {
        
        let index = index.index();
        let mut entry = Some(value);
        if let Some(option_value) = self.vec.get_mut(index) {
            std::mem::swap(&mut entry, option_value);
        } else {
            let len = self.vec.len();
            if index > len {
                self.vec.resize_with(index+1, Default::default);
            }
            self.vec.insert(index, entry);
        }
    }

    pub fn remove(&mut self, index: I) -> Option<T> {
        
        let mut entry = None;
        if let Some(option_value) = self.vec.get_mut(index.index()) {
            std::mem::swap(&mut entry, option_value);
        }

        entry
    }
}