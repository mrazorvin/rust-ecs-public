use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
};

#[derive(Debug)]
pub enum Placeholder<T> {
    Content(T),
    Placeholder(Key),
    None,
}

impl<T> Placeholder<T> {
    pub fn take(&mut self) -> Placeholder<T> {
        std::mem::take(self)
    }

    pub fn replace(&mut self, value: T) -> Placeholder<T> {
        std::mem::replace(self, Placeholder::Content(value))
    }

    pub fn get(&mut self, key: impl ToKey) -> Placeholder<T> {
        std::mem::replace(self, Placeholder::Placeholder(key.to_key()))
    }
}

impl<T: Debug> Deref for Placeholder<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Placeholder::Content(content) => content,
            Placeholder::Placeholder(placeholder) => {
                unreachable!("Cannot deref Placeholder({:?})", placeholder)
            }
            Placeholder::None => {
                unreachable!("Cannot deref Placeholder")
            }
        }
    }
}

impl<T: Debug> DerefMut for Placeholder<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Placeholder::Content(ref mut content) => content,
            Placeholder::Placeholder(_) => {
                unreachable!("Cannot deref Placeholder")
            }
            Placeholder::None => {
                unreachable!("Cannot deref mut Placehodler")
            }
        }
    }
}

impl<T> Default for Placeholder<T> {
    fn default() -> Self {
        Placeholder::None
    }
}

#[derive(Debug)]
pub enum Key {
    String(String),
    Slice(&'static str),
    Info((&'static str, u32)),
}

pub trait ToKey {
    fn to_key(self) -> Key;
}

impl ToKey for (&'static str, u32) {
    fn to_key(self) -> Key {
        Key::Info(self)
    }
}

impl ToKey for String {
    fn to_key(self) -> Key {
        Key::String(self)
    }
}

impl ToKey for &'static str {
    fn to_key(self) -> Key {
        Key::Slice(self)
    }
}
