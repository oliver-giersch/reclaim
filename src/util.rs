//! Common and useful utility traits.

use core::hint::unreachable_unchecked;
use core::ptr::{self, NonNull};

use crate::pointer::{
    Marked::{self, Null, Value},
    MarkedNonNullable,
};

////////////////////////////////////////////////////////////////////////////////////////////////////
// UnwrapPtr (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait that adds a method to ergonomically extract a `*const T' from an
/// [`Option`] of a non-nullable pointer or reference type.
pub trait UnwrapPtr {
    /// The type to which the [`Option`] contains a pointer or reference.
    type Item: Sized;

    /// Unwraps the [`Option`] and returns the contained value converted to a
    /// `const` pointer or `null`.
    fn unwrap_ptr(self) -> *const Self::Item;
}

/********** blanket impls *************************************************************************/

impl<'a, T> UnwrapPtr for Option<&'a T> {
    type Item = T;

    #[inline]
    fn unwrap_ptr(self) -> *const Self::Item {
        match self {
            Some(value) => value as *const _,
            None => ptr::null(),
        }
    }
}

impl<'a, T> UnwrapPtr for Option<&'a mut T> {
    type Item = T;

    #[inline]
    fn unwrap_ptr(self) -> *const Self::Item {
        match self {
            Some(value) => value as *mut _,
            None => ptr::null(),
        }
    }
}

impl<T> UnwrapPtr for Option<NonNull<T>> {
    type Item = T;

    #[inline]
    fn unwrap_ptr(self) -> *const Self::Item {
        match self {
            Some(value) => value.as_ptr() as *const _,
            None => ptr::null(),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// UnwrapMutPtr (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait that adds a method to ergonomically extract a `*mut T' from an
/// [`Option`] of a non-nullable pointer or reference type.
pub trait UnwrapMutPtr: UnwrapPtr {
    /// Unwraps the [`Option`] and returns the contained value converted to a
    /// `mut` pointer or `null`.
    fn unwrap_mut_ptr(self) -> *mut <Self as UnwrapPtr>::Item;
}

/********** blanket impls *************************************************************************/

impl<'a, T> UnwrapMutPtr for Option<&'a mut T> {
    #[inline]
    fn unwrap_mut_ptr(self) -> *mut Self::Item {
        self.unwrap_ptr() as *mut _
    }
}

impl<T> UnwrapMutPtr for Option<NonNull<T>> {
    #[inline]
    fn unwrap_mut_ptr(self) -> *mut Self::Item {
        self.unwrap_ptr() as *mut _
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// UnwrapUnchecked (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait for adding an `unsafe` unwrapping method to [`Option`] like types.
pub trait UnwrapUnchecked {
    /// The contained type that will be unwrapped.
    type Item: Sized;

    /// Unwraps the contained item in an [`Option`] like type **without**
    /// checking if the value actually exists.
    ///
    /// # Safety
    ///
    /// The caller has to ensure `self` actually contains an item, otherwise,
    /// there will be undefined behaviour.
    ///
    /// # Panics
    ///
    /// This method may panic in debug builds, if it is called on a value that
    /// does not contain an item
    unsafe fn unwrap_unchecked(self) -> Self::Item;
}

/********** blanket impls *************************************************************************/

impl<T> UnwrapUnchecked for Option<T> {
    type Item = T;

    #[inline]
    unsafe fn unwrap_unchecked(self) -> Self::Item {
        debug_assert!(self.is_some(), "`unwrap_unchecked` called on a `None`");
        match self {
            Some(value) => value,
            None => unreachable_unchecked(),
        }
    }
}

impl<T, E> UnwrapUnchecked for Result<T, E> {
    type Item = T;

    #[inline]
    unsafe fn unwrap_unchecked(self) -> Self::Item {
        debug_assert!(self.is_ok(), "`unwrap_unchecked` called on an `Err`");
        match self {
            Ok(value) => value,
            Err(_) => unreachable_unchecked(),
        }
    }
}

impl<T: MarkedNonNullable> UnwrapUnchecked for Marked<T> {
    type Item = T;

    #[inline]
    unsafe fn unwrap_unchecked(self) -> Self::Item {
        debug_assert!(self.is_value(), "`unwrap_unchecked` called on a `Null`");
        match self {
            Value(value) => value,
            Null(_) => unreachable_unchecked(),
        }
    }
}
