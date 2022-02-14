//
use std::{borrow::Borrow, collections::HashMap, hash::Hash, ops};

use slotmap::{Key, SlotMap};

macro_rules! new_key_type {
    (
        $(
            $(#[$outer:meta])* $vis:vis struct $name:ident;
        )*
    ) => {
        slotmap::new_key_type! {
            $(
                $(#[$outer])*
                $vis struct $name;
            )*
        }

        $(
            impl<K, V> std::ops::Index<$name>
                for crate::arena::ArenaMap<K, V, $name>
            {
                type Output = V;

                #[track_caller]
                fn index(&self, index: $name) -> &Self::Output {
                    self.get_by_index(index).expect("no entry found for key")
                }
            }

            impl<K, V> std::ops::IndexMut<$name>
                for crate::arena::ArenaMap<K, V, $name>
            {
                #[track_caller]
                fn index_mut(&mut self, index: $name) -> &mut Self::Output {
                    self.get_by_index_mut(index).expect("no entry found for key")
                }
            }
        )*
    };
}

#[derive(Debug)]
pub struct ArenaMap<K, V, I>
where
    I: Key,
{
    pub(crate) map: HashMap<K, I>,
    pub(crate) arena: SlotMap<I, V>,
}

impl<K, I, V> ArenaMap<K, V, I>
where
    I: Key,
{
    pub(crate) fn alloc(&mut self, value: V) -> I {
        self.arena.insert(value)
    }

    pub(crate) fn insert(&mut self, key: K, value: V) -> I
    where
        K: Eq + Hash,
    {
        let index = self.alloc(value);
        self.map.insert(key, index);
        index
    }

    pub(crate) fn insert_index(&mut self, key: K, index: I)
    where
        K: Eq + Hash,
    {
        self.map.insert(key, index);
    }

    pub fn contains_key<Q>(&mut self, key: &Q) -> bool
    where
        K: Eq + Hash + Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        self.map.contains_key(key)
    }

    pub fn contains_index(&mut self, index: I) -> bool {
        self.arena.contains_key(index)
    }

    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Eq + Hash + Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        self.arena.get(*self.map.get(key)?)
    }

    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Eq + Hash + Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        self.arena.get_mut(*self.map.get(key)?)
    }

    pub fn get_by_index(&self, index: I) -> Option<&V> {
        self.arena.get(index)
    }

    pub fn get_by_index_mut(&mut self, index: I) -> Option<&mut V> {
        self.arena.get_mut(index)
    }

    pub fn get_index<Q>(&self, key: &Q) -> Option<I>
    where
        K: Eq + Hash + Borrow<Q>,
        Q: ?Sized + Eq + Hash,
    {
        self.map.get(key).copied()
    }
}

impl<K, I, V> Default for ArenaMap<K, V, I>
where
    K: Eq + Hash,
    I: Key,
{
    fn default() -> Self {
        Self { map: HashMap::default(), arena: SlotMap::default() }
    }
}

impl<K, I, V, Q> ops::Index<&Q> for ArenaMap<K, V, I>
where
    K: Eq + Hash + Borrow<Q>,
    Q: ?Sized + Eq + Hash,
    I: Key,
{
    type Output = V;

    #[track_caller]
    fn index(&self, index: &Q) -> &Self::Output {
        self.get(index).expect("no entry found for key")
    }
}

impl<K, I, V, Q> ops::IndexMut<&Q> for ArenaMap<K, V, I>
where
    K: Eq + Hash + Borrow<Q>,
    Q: ?Sized + Eq + Hash,
    I: Key,
{
    #[track_caller]
    fn index_mut(&mut self, index: &Q) -> &mut Self::Output {
        self.get_mut(index).expect("no entry found for key")
    }
}
