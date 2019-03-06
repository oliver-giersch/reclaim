use core::cmp;
use core::mem;

use typenum::Unsigned;

use crate::owned::Owned;
use crate::{MarkedNonNull, MarkedPtr};
use crate::{Reclaim, Shared, Unlinked};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Pointer (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Trait for nullable and non-nullable markable pointer types.
pub trait MarkedPointer {
    type Item;
    type MarkBits: Unsigned;

    /// TODO: Doc...
    fn as_marked(&self) -> MarkedPtr<Self::Item, Self::MarkBits>;

    /// Consumes `self` and returns a marked pointer
    fn into_marked(self) -> MarkedPtr<Self::Item, Self::MarkBits>;

    /// Constructs a `Self` from a raw pointer.
    ///
    /// # Safety
    ///
    /// The caller has to ensure that raw is a valid pointer for the respective `Self` type.
    /// If `Self` is nullable, a null pointer is a valid value. Otherwise all values must be valid
    /// pointers.
    unsafe fn from_marked(marked: MarkedPtr<Self::Item, Self::MarkBits>) -> Self;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Owned & Option<Owned>
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, N: Unsigned, R: Reclaim> MarkedPointer for Owned<T, N, R> {
    type Item = T;
    type MarkBits = N;

    fn as_marked(&self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        self.as_marked()
    }

    fn into_marked(self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        Owned::into_marked(self)
    }

    unsafe fn from_marked(marked: MarkedPtr<Self::Item, Self::MarkBits>) -> Self {
        debug_assert!(!marked.is_null());
        Owned::from_marked_non_null(MarkedNonNull::new_unchecked(marked))
    }
}

impl<T, N: Unsigned, R: Reclaim> MarkedPointer for Option<Owned<T, N, R>> {
    type Item = T;
    type MarkBits = N;

    fn as_marked(&self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        unsafe { mem::transmute_copy(self) }
    }

    fn into_marked(self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        unsafe { mem::transmute(self) }
    }

    unsafe fn from_marked(marked: MarkedPtr<Self::Item, Self::MarkBits>) -> Self {
        mem::transmute(marked)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Shared & Option<Shared>
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'g, T, N: Unsigned, R: Reclaim> MarkedPointer for Shared<'g, T, N, R> {
    type Item = T;
    type MarkBits = N;

    fn as_marked(&self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        self.as_marked()
    }

    fn into_marked(self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        Shared::into_marked(self)
    }

    unsafe fn from_marked(marked: MarkedPtr<Self::Item, Self::MarkBits>) -> Self {
        debug_assert!(!marked.is_null());
        Shared::from_marked_non_null(MarkedNonNull::new_unchecked(marked))
    }
}

impl<'g, T, N: Unsigned, R: Reclaim> MarkedPointer for Option<Shared<'g, T, N, R>> {
    type Item = T;
    type MarkBits = N;

    fn as_marked(&self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        unsafe { mem::transmute_copy(self) }
    }

    fn into_marked(self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        unsafe { mem::transmute(self) }
    }

    unsafe fn from_marked(marked: MarkedPtr<Self::Item, Self::MarkBits>) -> Self {
        mem::transmute(marked)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Unlinked & Option<Unlinked>
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, N: Unsigned, R: Reclaim> MarkedPointer for Unlinked<T, N, R> {
    type Item = T;
    type MarkBits = N;

    fn as_marked(&self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        self.as_marked()
    }

    fn into_marked(self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        Unlinked::into_marked(self)
    }

    unsafe fn from_marked(marked: MarkedPtr<Self::Item, Self::MarkBits>) -> Self {
        debug_assert!(!marked.is_null());
        Unlinked::from_marked_non_null(MarkedNonNull::new_unchecked(marked))
    }
}

impl<T, N: Unsigned, R: Reclaim> MarkedPointer for Option<Unlinked<T, N, R>> {
    type Item = T;
    type MarkBits = N;

    fn as_marked(&self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        unsafe { mem::transmute_copy(self) }
    }

    fn into_marked(self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        unsafe { mem::transmute(self) }
    }

    unsafe fn from_marked(marked: MarkedPtr<Self::Item, Self::MarkBits>) -> Self {
        mem::transmute(marked)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Blanket impls
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, N: Unsigned, U> PartialEq<U> for MarkedPtr<T, N>
where
    U: MarkedPointer<Item = T, MarkBits = N>,
{
    fn eq(&self, other: &U) -> bool {
        self.eq(&other.as_marked())
    }
}

impl<T, N: Unsigned, U> PartialOrd<U> for MarkedPtr<T, N>
where
    U: MarkedPointer<Item = T, MarkBits = N>,
{
    fn partial_cmp(&self, other: &U) -> Option<cmp::Ordering> {
        self.partial_cmp(&other.as_marked())
    }
}

impl<T, N: Unsigned, U> PartialEq<U> for MarkedNonNull<T, N>
where
    U: MarkedPointer<Item = T, MarkBits = N>,
{
    fn eq(&self, other: &U) -> bool {
        self.into_marked().eq(&other.as_marked())
    }
}

impl<T, N: Unsigned, U> PartialOrd<U> for MarkedNonNull<T, N>
where
    U: MarkedPointer<Item = T, MarkBits = N>,
{
    fn partial_cmp(&self, other: &U) -> Option<cmp::Ordering> {
        self.into_marked().partial_cmp(&other.as_marked())
    }
}
