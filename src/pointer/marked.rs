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

    /// Moves the value out of the `Marked<T>` if it is `Value(ptr)`.
    #[inline]
    pub fn unwrap_value(self) -> T {
        match self {
            Value(ptr) => ptr,
            _ => panic!("called `Marked::unwrap_value()` on a `Null` or `OnlyTag` value"),
        }
    }

    /// Moves the value out of the `Marked<T>` if it is `Value(ptr)`.
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
}
