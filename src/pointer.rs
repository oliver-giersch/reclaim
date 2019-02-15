use core::mem;

use crate::owned::Owned;
use crate::{Reclaim, Shared, StatelessAlloc, Unlinked};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Pointer (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Trait for nullable and non-nullable markable pointer types.
pub trait Pointer {
    type Item;

    /// Consumes `self`, returning a raw pointer.
    fn into_raw(self) -> *mut Self::Item;

    /// Constructs a `Self` from a raw pointer.
    ///
    /// # Safety
    ///
    /// The caller has to ensure that raw is a valid for the respective `Self`
    /// type. If `Self` is nullable, a null pointer is a valid value. Otherwise all
    /// values must be valid pointers.
    unsafe fn from_raw(raw: *mut Self::Item) -> Self;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Owned & Option<Owned>
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: Reclaim<A>, A: StatelessAlloc> Pointer for Owned<T, R, A> {
    type Item = T;

    fn into_raw(self) -> *mut Self::Item {
        Owned::into_raw(self)
    }

    unsafe fn from_raw(raw: *mut Self::Item) -> Self {
        Owned::from_raw(raw)
    }
}

impl<T, R: Reclaim<A>, A: StatelessAlloc> Pointer for Option<Owned<T, R, A>> {
    type Item = T;

    fn into_raw(self) -> *mut Self::Item {
        unsafe { mem::transmute(self) }
    }

    unsafe fn from_raw(raw: *mut Self::Item) -> Self {
        mem::transmute(raw)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Shared & Option<Shared>
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'g, T, R: Reclaim<A>, A: StatelessAlloc> Pointer for Shared<'g, T, R, A> {
    type Item = T;

    fn into_raw(self) -> *mut Self::Item {
        Shared::into_raw(self)
    }

    unsafe fn from_raw(raw: *mut Self::Item) -> Self {
        Shared::from_raw(raw)
    }
}

impl<'g, T, R: Reclaim<A>, A: StatelessAlloc> Pointer for Option<Shared<'g, T, R, A>> {
    type Item = T;

    fn into_raw(self) -> *mut Self::Item {
        unsafe { mem::transmute(self) }
    }

    unsafe fn from_raw(raw: *mut Self::Item) -> Self {
        mem::transmute(raw)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Unlinked & Option<Unlinked>
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: Reclaim<A>, A: StatelessAlloc> Pointer for Unlinked<T, R, A> {
    type Item = T;

    fn into_raw(self) -> *mut Self::Item {
        Unlinked::into_raw(self)
    }

    unsafe fn from_raw(raw: *mut Self::Item) -> Self {
        Unlinked::from_raw(raw)
    }
}

impl<T, R: Reclaim<A>, A: StatelessAlloc> Pointer for Option<Unlinked<T, R, A>> {
    type Item = T;

    fn into_raw(self) -> *mut Self::Item {
        unsafe { mem::transmute(self) }
    }

    unsafe fn from_raw(raw: *mut Self::Item) -> Self {
        mem::transmute(raw)
    }
}
