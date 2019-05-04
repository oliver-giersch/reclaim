use core::mem;

use crate::pointer::{
    Marked::{self, Null, OnlyTag, Pointer},
    NonNullable,
};

impl<T: NonNullable> Marked<T> {
    /// Returns `true` if the marked contains a `Value`.
    #[inline]
    pub fn is_value(&self) -> bool {
        match *self {
            Pointer(_) => true,
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
            Pointer(ref value) => Pointer(value),
            OnlyTag(ref tag) => OnlyTag(*tag),
            Null => Null,
        }
    }

    /// TODO: Doc...
    #[inline]
    pub fn as_mut(&mut self) -> Marked<&mut T> {
        match *self {
            Pointer(ref mut value) => Pointer(value),
            OnlyTag(ref tag) => OnlyTag(*tag),
            Null => Null,
        }
    }

    /// Moves the value out of the `Marked` if it is `Value(ptr)`.
    #[inline]
    pub fn unwrap_value(self) -> T {
        match self {
            Pointer(ptr) => ptr,
            _ => panic!("called `Marked::unwrap_value()` on a `Null` or `OnlyTag` value"),
        }
    }

    /// TODO: Doc...
    #[inline]
    pub fn unwrap_value_or_else<F: FnOnce() -> T>(self, f: F) -> T {
        match self {
            Pointer(ptr) => ptr,
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

    /// TODO: Doc...
    #[inline]
    pub fn map<U: NonNullable>(self, func: impl (FnOnce(T) -> U)) -> Marked<U> {
        match self {
            Pointer(ptr) => Pointer(func(ptr)),
            OnlyTag(tag) => OnlyTag(tag),
            Null => Null,
        }
    }

    /// Converts `self` into an option, returning `Some` if a `Value` is
    /// contained.
    #[inline]
    pub fn into_option(self) -> Option<T> {
        match self {
            Pointer(ptr) => Some(ptr),
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
        mem::replace(self, Pointer(value))
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// NonNull
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'a, T> NonNullable for &'a T {}
impl<'a, T> NonNullable for &'a mut T {}
