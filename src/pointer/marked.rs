use core::mem;

use crate::pointer::{
    Marked::{self, Null, OnlyTag, Value},
    NonNullable,
};

impl<T: NonNullable> Marked<T> {
    /// Returns `true` if the marked contains a `Value`.
    #[inline]
    pub fn is_value(&self) -> bool {
        match *self {
            Value(_) => true,
            _ => false,
        }
    }

    /// Returns `true` if the marked contains a `OnlyTag` value.
    #[inline]
    pub fn is_tag(&self) -> bool {
        match *self {
            OnlyTag(_) => true,
            _ => false,
        }
    }

    /// Returns `true` if the marked contains a `OnlyTag` value.
    #[inline]
    pub fn is_null(&self) -> bool {
        match *self {
            Null => true,
            _ => false,
        }
    }

    /// TODO: Doc...
    #[inline]
    pub fn as_ref(&self) -> Marked<&T> {
        match *self {
            Value(ref value) => Marked(value),
            OnlyTag(ref tag) => OnlyTag(*tag),
            Null => Null
        }
    }

    /// TODO: Doc...
    #[inline]
    pub fn as_mut(&mut self) -> Marked<&mut T> {
        match *self {
            Value(ref mut value) => Marked(value),
            OnlyTag(ref tag) => OnlyTag(*tag),
            Null => Null
        }
    }

    /// Moves the value out of the `Marked` if it is `Value(ptr)`.
    #[inline]
    pub fn unwrap_value(self) -> T {
        match self {
            Value(ptr) => ptr,
            _ => panic!("called `Marked::unwrap_value()` on a `Null` or `OnlyTag` value"),
        }
    }

    /// TODO: Doc...
    #[inline]
    pub fn unwrap_value_or_else<F: FnOnce() -> T>(self, f: F) -> T {
        match self {
            Value(ptr) => ptr,
            _ => f(),
        }
    }

    /// Moves the value out of the `Marked` if it is `Value(ptr)`.
    #[inline]
    pub fn unwrap_tag(self) -> usize {
        match self {
            OnlyTag(tag) => tag,
            _ => panic!("called `Marked::unwrap_tag()` on a `Value` or `Null` value"),
        }
    }

    /// Converts `self` into an option, returning `Some` if a `Value` is
    /// contained.
    #[inline]
    pub fn into_option(self) -> Option<T> {
        match self {
            Value(ptr) => Some(ptr),
            _ => None,
        }
    }

    /// TODO: Doc...
    #[inline]
    pub fn take(&mut self) -> Self {
        mem::replace(self, Null)
    }

    /// TODO: Doc...
    #[inline]
    pub fn replace(&mut self, value: T) -> Self {
        mem::replace(self, value)
    }
}
