//! Interner module.

use std::collections::HashSet;
use std::hash::Hash;
use std::sync::Arc;
use std::sync::LazyLock;
use std::sync::RwLock;

pub struct Interner<T>(RwLock<HashSet<Arc<T>>>);

impl<T> Interner<T>
where
    T: Eq + Hash,
{
    pub const fn r#static() -> LazyLock<Self> {
        LazyLock::new(Self::default)
    }

    fn get(&self, value: &T) -> Option<Arc<T>> {
        let store = self.0.read().unwrap();

        store.get(value).cloned()
    }

    fn set(&self, value: T) -> Arc<T> {
        let mut store = self.0.write().unwrap();

        let arc = Arc::new(value);
        store.insert(arc.clone());

        arc
    }

    pub fn clear(&self) {
        let mut store = self.0.write().unwrap();

        store.clear();
        store.shrink_to_fit();
    }

    pub fn intern(&self, value: T) -> Arc<T> {
        self.get(&value).unwrap_or_else(|| self.set(value))
    }
}

impl<T> Default for Interner<T>
where
    T: Eq + Hash,
{
    fn default() -> Self {
        Self(Default::default())
    }
}
