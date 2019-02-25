use core::cmp;
use core::mem;

use crate::owned::Owned;
use crate::{Reclaim, Shared, StatelessAlloc, Unlinked};
use crate::MarkedPtr;
use crate::MarkedNonNull;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Pointer (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Trait for nullable and non-nullable markable pointer types.
pub trait MarkedPointer {
    type Item;

    fn as_marked(&self) -> MarkedPtr<Self::Item>;

    /// Consumes `self` and returns a marked pointer
    fn into_marked(self) -> MarkedPtr<Self::Item>;

    /// Constructs a `Self` from a raw pointer.
    ///
    /// # Safety
    ///
    /// The caller has to ensure that raw is a valid pointer for the respective `Self` type.
    /// If `Self` is nullable, a null pointer is a valid value. Otherwise all values must be valid
    /// pointers.
    unsafe fn from_marked(marked: MarkedPtr<Self::Item>) -> Self;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Owned & Option<Owned>
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: Reclaim<A>, A: StatelessAlloc> MarkedPointer for Owned<T, R, A> {
    type Item = T;

    fn as_marked(&self) -> MarkedPtr<Self::Item> {
        self.as_marked()
    }

    fn into_marked(self) -> MarkedPtr<Self::Item> {
        Owned::into_marked(self)
    }

    unsafe fn from_marked(marked: MarkedPtr<Self::Item>) -> Self {
        debug_assert!(!marked.is_null());
        Owned::from_marked_non_null(MarkedNonNull::new_unchecked(marked))
    }
}

impl<T, R: Reclaim<A>, A: StatelessAlloc> MarkedPointer for Option<Owned<T, R, A>> {
    type Item = T;

    fn as_marked(&self) -> MarkedPtr<Self::Item> {
        unsafe { mem::transmute_copy(self) }
    }

    fn into_marked(self) -> MarkedPtr<Self::Item> {
        unsafe { mem::transmute(self) }
    }

    unsafe fn from_marked(marked: MarkedPtr<Self::Item>) -> Self {
        mem::transmute(marked)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Shared & Option<Shared>
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'g, T, R: Reclaim<A>, A: StatelessAlloc> MarkedPointer for Shared<'g, T, R, A> {
    type Item = T;

    fn as_marked(&self) -> MarkedPtr<Self::Item> {
        self.as_marked()
    }

    fn into_marked(self) -> MarkedPtr<Self::Item> {
        Shared::into_marked(self)
    }

    unsafe fn from_marked(marked: MarkedPtr<Self::Item>) -> Self {
        debug_assert!(!marked.is_null());
        Shared::from_marked_non_null(MarkedNonNull::new_unchecked(marked))
    }
}

impl<'g, T, R: Reclaim<A>, A: StatelessAlloc> MarkedPointer for Option<Shared<'g, T, R, A>> {
    type Item = T;

    fn as_marked(&self) -> MarkedPtr<Self::Item> {
        unsafe { mem::transmute_copy(self) }
    }

    fn into_marked(self) -> MarkedPtr<Self::Item> {
        unsafe { mem::transmute(self) }
    }

    unsafe fn from_marked(marked: MarkedPtr<Self::Item>) -> Self {
        mem::transmute(marked)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Unlinked & Option<Unlinked>
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: Reclaim<A>, A: StatelessAlloc> MarkedPointer for Unlinked<T, R, A> {
    type Item = T;

    fn as_marked(&self) -> MarkedPtr<Self::Item> {
        self.as_marked()
    }

    fn into_marked(self) -> MarkedPtr<Self::Item> {
        Unlinked::into_marked(self)
    }

    unsafe fn from_marked(marked: MarkedPtr<Self::Item>) -> Self {
        debug_assert!(!marked.is_null());
        Unlinked::from_marked_non_null(MarkedNonNull::new_unchecked(marked))
    }
}

impl<T, R: Reclaim<A>, A: StatelessAlloc> MarkedPointer for Option<Unlinked<T, R, A>> {
    type Item = T;

    fn as_marked(&self) -> MarkedPtr<Self::Item> {
        unsafe { mem::transmute_copy(self) }
    }

    fn into_marked(self) -> MarkedPtr<Self::Item> {
        unsafe { mem::transmute(self) }
    }

    unsafe fn from_marked(marked: MarkedPtr<Self::Item>) -> Self {
        mem::transmute(marked)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Blanket impls
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, U> PartialEq<U> for MarkedPtr<T> where U: MarkedPointer<Item = T> {
    fn eq(&self, other: &U) -> bool {
        self.eq(&other.as_marked())
    }
}

impl<T, U> PartialOrd<U> for MarkedPtr<T> where U: MarkedPointer<Item = T> {
    fn partial_cmp(&self, other: &U) -> Option<cmp::Ordering> {
        self.partial_cmp(&other.as_marked())
    }
}

impl<T, U> PartialEq<U> for MarkedNonNull<T> where U: MarkedPointer<Item = T> {
    fn eq(&self, other: &U) -> bool {
        self.into_marked().eq(&other.as_marked())
    }
}

impl<T, U> PartialOrd<U> for MarkedNonNull<T> where U: MarkedPointer<Item = T> {
    fn partial_cmp(&self, other: &U) -> Option<cmp::Ordering> {
        self.into_marked().partial_cmp(&other.as_marked())
    }
}