use core::mem;

use crate::pointer::{
    Marked::{self, Null, OnlyTag, Value},
    NonNullable,
};

impl<T: NonNullable> Marked<T> {
    /// Returns `true` if the marked value contains a [`Value`][Value].
    #[inline]
    pub fn is_value(&self) -> bool {
        match *self {
            Value(_) => true,
            _ => false,
        }
    }

    /// Returns `true` if the marked value contains a [`OnlyTag`][OnlyTag].
    #[inline]
    pub fn is_tag(&self) -> bool {
        match *self {
            OnlyTag(_) => true,
            _ => false,
        }
    }

    /// Returns `true` if the marked value contains a [`Null`][Null].
    #[inline]
    pub fn is_null(&self) -> bool {
        match *self {
            Null => true,
            _ => false,
        }
    }

    /// Converts from `Marked<T>` to `Marked<&T>`.
    #[inline]
    pub fn as_ref(&self) -> Marked<&T> {
        match *self {
            Value(ref value) => Value(value),
            OnlyTag(ref tag) => OnlyTag(*tag),
            Null => Null,
        }
    }

    /// Converts from `Marked<T>` to `Marked<&mut T>`.
    #[inline]
    pub fn as_mut(&mut self) -> Marked<&mut T> {
        match *self {
            Value(ref mut value) => Value(value),
            OnlyTag(ref tag) => OnlyTag(*tag),
            Null => Null,
        }
    }

    /// Moves the pointer out of the `Marked` if it is [`Value(ptr)`][Value].
    #[inline]
    pub fn unwrap_value(self) -> T {
        match self {
            Value(ptr) => ptr,
            _ => panic!("called `Marked::unwrap_value()` on a `Null` or `OnlyTag` value"),
        }
    }

    #[inline]
    pub fn unwrap_value_or_else<F: FnOnce() -> T>(self, f: F) -> T {
        match self {
            Value(ptr) => ptr,
            _ => f(),
        }
    }

    /// Moves the pure tag out of the `Marked` if it is
    /// [`OnlyTag(tag)`][OnlyTag].
    #[inline]
    pub fn unwrap_tag(self) -> usize {
        match self {
            OnlyTag(tag) => tag,
            _ => panic!("called `Marked::unwrap_tag()` on a `Value` or `Null` value"),
        }
    }

    #[inline]
    pub fn map<U: NonNullable>(self, func: impl (FnOnce(T) -> U)) -> Marked<U> {
        match self {
            Value(ptr) => Value(func(ptr)),
            OnlyTag(tag) => OnlyTag(tag),
            Null => Null,
        }
    }

    #[inline]
    pub fn map_or_else<U: NonNullable>(
        self,
        default: impl FnOnce() -> U,
        func: impl FnOnce(T) -> U,
    ) -> U {
        match self {
            Value(ptr) => func(ptr),
            _ => default(),
        }
    }

    /// Converts `self` from `Marked<T>` to [`Option<T>`][Option].
    #[inline]
    pub fn value(self) -> Option<T> {
        match self {
            Value(ptr) => Some(ptr),
            _ => None,
        }
    }

    #[inline]
    pub fn only_tag(self) -> Option<usize> {
        match self {
            OnlyTag(tag) => Some(tag),
            _ => None,
        }
    }

    #[inline]
    pub fn take(&mut self) -> Self {
        mem::replace(self, Null)
    }

    #[inline]
    pub fn replace(&mut self, value: T) -> Self {
        mem::replace(self, Value(value))
    }
}
