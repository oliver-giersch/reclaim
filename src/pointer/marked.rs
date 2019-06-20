use core::mem;

use crate::pointer::{
    Marked::{self, Null, Value},
    NonNullable,
};
use crate::MarkedPointer;

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl inherent
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T: NonNullable> Marked<T> {
    /// Returns `true` if the marked value contains a [`Value`].
    #[inline]
    pub fn is_value(&self) -> bool {
        match *self {
            Value(_) => true,
            _ => false,
        }
    }

    /// Returns `true` if the marked value is a [`Null`].
    #[inline]
    pub fn is_null(&self) -> bool {
        match *self {
            Null(_) => true,
            _ => false,
        }
    }

    /// Converts from `Marked<T>` to `Marked<&T>`.
    #[inline]
    pub fn as_ref(&self) -> Marked<&T> {
        match self {
            Value(value) => Value(value),
            Null(tag) => Null(*tag),
        }
    }

    /// Converts from `Marked<T>` to `Marked<&mut T>`.
    #[inline]
    pub fn as_mut(&mut self) -> Marked<&mut T> {
        match self {
            Value(value) => Value(value),
            Null(tag) => Null(*tag),
        }
    }

    /// Moves the pointer out of the `Marked` if it is [`Value(ptr)`][Value].
    #[inline]
    pub fn unwrap_value(self) -> T {
        match self {
            Value(ptr) => ptr,
            _ => panic!("called `Marked::unwrap_value()` on a `Null` value"),
        }
    }

    /// Extracts the tag out of the `Marked` if it is [`Null(tag)`][Null].
    #[inline]
    pub fn unwrap_null(self) -> usize {
        match self {
            Null(tag) => tag,
            _ => panic!("called `Marked::unwrap_tag()` on a `Value`"),
        }
    }

    /// Returns the contained value or the result of the given `func`.
    #[inline]
    pub fn unwrap_value_or_else(self, func: impl (FnOnce(usize) -> T)) -> T {
        match self {
            Value(ptr) => ptr,
            Null(tag) => func(tag),
        }
    }

    /// Maps a `Marked<T>` to `Marked<U>` by applying a function to a contained
    /// value.
    #[inline]
    pub fn map<U: NonNullable>(self, func: impl (FnOnce(T) -> U)) -> Marked<U> {
        match self {
            Value(ptr) => Value(func(ptr)),
            Null(tag) => Null(tag),
        }
    }

    /// Applies a function to the contained value (if any), or computes a
    /// default value using `func`, if no value is contained.
    #[inline]
    pub fn map_or_else<U: NonNullable>(
        self,
        default: impl FnOnce(usize) -> U,
        func: impl FnOnce(T) -> U,
    ) -> U {
        match self {
            Value(ptr) => func(ptr),
            Null(tag) => default(tag),
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

    /// Takes the value of the [`Marked`], leaving a [`Null`] variant in its
    /// place.
    #[inline]
    pub fn take(&mut self) -> Self {
        mem::replace(self, Null(0))
    }

    /// Replaces the actual value in the [`Marked`] with the given `value`,
    /// returning the old value.
    #[inline]
    pub fn replace(&mut self, value: T) -> Self {
        mem::replace(self, Value(value))
    }
}

// TODO: remove?
impl<T: NonNullable + MarkedPointer> Marked<T> {
    /// Decomposes the inner marked pointer, returning only the separated tag.
    #[inline]
    pub fn decompose_tag(&self) -> usize {
        match self {
            Value(ptr) => ptr.as_marked_ptr().decompose_tag(),
            Null(tag) => *tag,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl Default
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T: NonNullable> Default for Marked<T> {
    #[inline]
    fn default() -> Self {
        Null(0)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl From
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T: NonNullable> From<Option<T>> for Marked<T> {
    #[inline]
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(ptr) => Value(ptr),
            None => Null(0),
        }
    }
}
