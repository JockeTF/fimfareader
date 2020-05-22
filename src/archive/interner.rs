//! Interner module.

use std::collections::HashSet;
use std::sync::RwLock;

pub struct Interner<T: 'static>(RwLock<HashSet<&'static T>>);

impl<T> Interner<T>
where
    T: Eq + std::hash::Hash,
{
    pub fn new() -> Self {
        Self(RwLock::new(HashSet::new()))
    }

    fn get(&self, value: &T) -> Option<&'static T> {
        let store = self.0.read().unwrap();

        store.get(value).map(|value| *value)
    }

    fn set(&self, value: T) -> &'static T {
        let boxed: Box<T> = Box::new(value);
        let leaked: &'static T = Box::leak(boxed);
        let mut store = self.0.write().unwrap();

        store.insert(leaked);

        leaked
    }

    pub fn intern(&self, value: T) -> &'static T {
        self.get(&value).unwrap_or_else(|| self.set(value))
    }
}
