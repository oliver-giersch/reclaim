use core::mem;

use crate::pointer::{
    Marked::{self, Null, OnlyTag, Ptr},
    NonNullable,
};

impl<T: NonNullable> Marked<T> {
    /// Returns `true` if the marked contains a `Value`.
    #[inline]
    pub fn is_ptr(&self) -> bool {
        match *self {
            Ptr(_) => true,
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
            Ptr(ref value) => Ptr(value),
            OnlyTag(ref tag) => OnlyTag(*tag),
            Null => Null,
        }
    }

    /// TODO: Doc...
    #[inline]
    pub fn as_mut(&mut self) -> Marked<&mut T> {
        match *self {
            Ptr(ref mut value) => Ptr(value),
            OnlyTag(ref tag) => OnlyTag(*tag),
            Null => Null,
        }
    }

    /// Moves the value out of the `Marked` if it is `Value(ptr)`.
    #[inline]
    pub fn unwrap_ptr(self) -> T {
        match self {
            Ptr(ptr) => ptr,
            _ => panic!("called `Marked::unwrap_value()` on a `Null` or `OnlyTag` value"),
        }
    }

    /// TODO: Doc...
    #[inline]
    pub fn unwrap_ptr_or_else<F: FnOnce() -> T>(self, f: F) -> T {
        match self {
            Ptr(ptr) => ptr,
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
            Ptr(ptr) => Ptr(func(ptr)),
            OnlyTag(tag) => OnlyTag(tag),
            Null => Null,
        }
    }

    /// TODO
    #[inline]
    pub fn map_or_else<U: NonNullable>(
        self,
        default: impl FnOnce() -> U,
        func: impl FnOnce(T) -> U,
    ) -> U {
        match self {
            Ptr(ptr) => func(ptr),
            _ => default(),
        }
    }

    /// Converts `self` into an option, returning `Some` if a `Value` is
    /// contained.
    #[inline]
    pub fn ptr(self) -> Option<T> {
        match self {
            Ptr(ptr) => Some(ptr),
            _ => None,
        }
    }

    /// TODO: Doc...
    #[inline]
    pub fn only_tag(self) -> Option<usize> {
        match self {
            OnlyTag(tag) => Some(tag),
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
        mem::replace(self, Ptr(value))
    }
}
