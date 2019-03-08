#![feature(allocator_api, const_fn, trait_alias)]

#![no_std]

#[cfg(test)]
extern crate std;

use core::alloc::Alloc;
use core::cmp;
use core::marker::PhantomData;
use core::mem;
use core::sync::atomic::Ordering;

pub use typenum::{
    Unsigned, U0, U1, U10, U11, U12, U13, U14, U15, U16, U17, U18, U19, U2, U20, U21, U22, U23,
    U24, U25, U26, U27, U28, U29, U3, U4, U5, U6, U7, U8, U9,
};

use memoffset::offset_of;

pub mod align;

mod atomic;
mod marked;
mod owned;
mod pointer;
#[cfg(test)]
mod tests;

pub use crate::atomic::{Atomic, Compare, CompareExchangeFailure, Store};
pub use crate::marked::{AtomicMarkedPtr, MarkedNonNull, MarkedPtr};
pub use crate::owned::Owned;

pub trait StatelessAlloc = Alloc + Copy + Clone + Default;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Reclaim (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

// TODO: Alloc Associated?
pub unsafe trait Reclaim
where
    Self: Sized,
{
    /// TODO: Doc...
    type Allocator: StatelessAlloc;
    /// Header that prepends every record. For reclamation Schemes that do not
    /// require any header data for records managed by them, `()` is the best
    /// choice.
    type RecordHeader: Default + Sized;

    /// TODO: Doc
    fn allocator() -> Self::Allocator {
        Default::default()
    }

    /// Reclaims a record and caches it until it is safe to de-allocates it, i.e. when no other
    /// threads can be guaranteed to hold any live references to it.
    ///
    /// # Safety
    ///
    /// The caller needs to ensure that the record is actually unlinked from the data structure.
    /// While an `Unlinked` can only be safely obtained by atomic operations that actually extract
    /// a value, but it is still possible to enter the same record twice into the same data
    /// structure, although this is not advisable generally.
    unsafe fn reclaim<T, N: Unsigned>(unlinked: Unlinked<T, N, Self>)
    where
        T: 'static;

    /// Reclaims a record and de-allocates it when no other threads hold any references to it.
    ///
    /// # Safety
    ///
    /// The reclamation scheme ensures to only call a type's `Drop` implementation after its
    /// reclamation. The caller has to ensure that the `drop` method does not use any non-static
    /// references contained in the type.
    unsafe fn reclaim_unchecked<T, N: Unsigned>(unlinked: Unlinked<T, N, Self>);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Protect (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Doc...
pub trait Protected
where
    Self: Sized,
{
    /// Generic type that is protected from reclamation
    type Item: Sized;
    /// The number of markable bits
    type MarkBits: Unsigned;
    /// TODO: Doc...
    type Reclaimer: Reclaim;

    /// Creates a new `Protected`.
    ///
    /// In case of region-based reclamation schemes (such as EBR), a call to `new` is guaranteed
    /// to create an active region guard.
    fn new() -> Self;

    /// Returns a `Shared` value wrapped in a `Some` from the internally protected pointer. If no
    /// value or a null-pointer has been acquired, `None` is returned.
    /// The `Shared` that is returned is guaranteed to be protected from reclamation during the
    /// lifetime of the `Protected`.
    fn shared(&self) -> Option<Shared<Self::Item, Self::MarkBits, Self::Reclaimer>>;

    /// Takes an atomic snapshot of the value stored within `atomic` at the moment of the call's
    /// invocation and stores it within `self`. The corresponding `Shared` wrapped in a `Some` (or
    /// `None`) is returned.
    ///
    /// The successfully acquired value is guaranteed to be protected from concurrent reclamation.
    fn acquire(
        &mut self,
        atomic: &Atomic<Self::Item, Self::MarkBits, Self::Reclaimer>,
        order: Ordering,
    ) -> Option<Shared<Self::Item, Self::MarkBits, Self::Reclaimer>>;

    /// TODO: Doc...
    fn acquire_if_equal(
        &mut self,
        atomic: &Atomic<Self::Item, Self::MarkBits, Self::Reclaimer>,
        compare: MarkedPtr<Self::Item, Self::MarkBits>,
        order: Ordering,
    ) -> Result<Option<Shared<Self::Item, Self::MarkBits, Self::Reclaimer>>, NotEqual>;

    /// TODO: Doc...
    fn release(&mut self);
}

/// A ZST struct that represents the failure state of an `acquire_if_equal` operation.
pub struct NotEqual;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Record
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Type that is allocated, whenever a new `Owned<T>` or `Atomic<T>` is created.
///
/// The header is never exposed and has to be manually accessed if and when needed.
/// One example use case could be reference-counted records, where the count is stored
/// in the header and increased or decreased whenever a `Protected` is acquired or goes
/// out of scope.
pub struct Record<T, R: Reclaim> {
    header: R::RecordHeader,
    elem: T,
}

impl<T, R: Reclaim> Record<T, R> {
    /// Creates a new record with the specified `elem` and a default header.
    #[inline]
    pub fn new(elem: T) -> Self {
        Self {
            header: Default::default(),
            elem,
        }
    }

    /// Creates a new record with the specified `header` and `elem`.
    #[inline]
    pub fn with_header(elem: T, header: R::RecordHeader) -> Self {
        Self { header, elem }
    }

    /// Returns a reference to header for the record at the pointed-to location of `ptr`.
    ///
    /// # Safety
    ///
    /// The pointer `elem` must be a valid, non-null and unmarked pointer to a `T`, that was at some
    /// point constructed as part `Record`.
    /// Otherwise, the pointer-arithmetic used to access the header will fail and memory-safety
    /// violated.
    #[inline]
    pub unsafe fn get_header<'a>(elem: *mut T) -> &'a R::RecordHeader {
        let header = (elem as usize) - Self::offset_elem() + Self::offset_header();
        &*(header as *mut _)
    }

    #[inline]
    pub unsafe fn get_header_mut<'a>(elem: *mut T) -> &'a mut R::RecordHeader {
        let header = (elem as usize) - Self::offset_elem() + Self::offset_header();
        &mut *(header as *mut _)
    }

    /// TODO: Doc...
    #[inline]
    pub unsafe fn get_record(elem: *mut T) -> *mut Self {
        let record = (elem as usize) - Self::offset_elem();
        record as *mut _
    }

    /// TODO: Doc...
    #[inline]
    pub fn offset_header() -> usize {
        offset_of!(Self, header)
    }

    /// TODO: Doc...
    #[inline]
    pub fn offset_elem() -> usize {
        offset_of!(Self, elem)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Shared
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A `Shared` represents a reference to value stored in a concurrent data structure.
pub struct Shared<'g, T, N: Unsigned, R: Reclaim> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<(&'g T, R)>,
}

impl<'g, T, N: Unsigned, R: Reclaim> Clone for Shared<'g, T, N, R> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
            _marker: PhantomData,
        }
    }
}

impl<'g, T, N: Unsigned, R: Reclaim> Copy for Shared<'g, T, N, R> {}

impl<'g, T, N: Unsigned, R: Reclaim> Shared<'g, T, N, R> {
    /// # Safety
    ///
    /// The caller must ensure that the provided pointer is both non-null and valid (may be marked)
    /// and is guaranteed to not be reclaimed during the lifetime of the shared reference.
    #[inline]
    pub unsafe fn from_marked(marked: MarkedPtr<T, N>) -> Option<Self> {
        // this is safe because ...
        mem::transmute(marked)
    }

    /// TODO: Doc...
    #[inline]
    pub unsafe fn from_marked_non_null(marked: MarkedNonNull<T, N>) -> Self {
        Self {
            inner: marked,
            _marker: PhantomData,
        }
    }

    /// TODO: Doc...
    #[inline]
    pub fn as_marked(&self) -> MarkedPtr<T, N> {
        self.inner.into_marked()
    }

    /// TODO: Doc...
    #[inline]
    pub fn into_marked(self) -> MarkedPtr<T, N> {
        self.inner.into_marked()
    }

    /// TODO: Doc...
    #[inline]
    pub fn into_marked_non_null(self) -> MarkedNonNull<T, N> {
        self.inner
    }

    /// TODO: Doc...
    #[inline]
    pub fn with_tag(shared: Self, tag: usize) -> Self {
        Self {
            inner: MarkedNonNull::compose(shared.inner.decompose_non_null(), tag),
            _marker: PhantomData,
        }
    }

    /// TODO: Doc...
    #[inline]
    pub fn wrapping_add_tag(shared: Self, add: usize) -> Self {
        Self::with_tag(shared, shared.tag().wrapping_add(add))
    }

    /// TODO: Doc...
    #[inline]
    pub fn wrapping_sub_tag(shared: Self, sub: usize) -> Self {
        Self::with_tag(shared, shared.tag().wrapping_sub(sub))
    }

    /// TODO: Doc...
    #[inline]
    pub fn tag(&self) -> usize {
        self.inner.decompose_tag()
    }

    /// TODO: Doc...
    #[inline]
    pub unsafe fn deref(&self) -> &T {
        &*self.inner.decompose_ptr()
    }
}

impl<'g, T, N: Unsigned, R: Reclaim> PartialEq<MarkedPtr<T, N>> for Shared<'g, T, N, R> {
    #[inline]
    fn eq(&self, other: &MarkedPtr<T, N>) -> bool {
        self.inner.into_marked().eq(other)
    }
}

impl<'g, T, N: Unsigned, R: Reclaim> PartialOrd<MarkedPtr<T, N>> for Shared<'g, T, N, R> {
    #[inline]
    fn partial_cmp(&self, other: &MarkedPtr<T, N>) -> Option<cmp::Ordering> {
        self.inner.into_marked().partial_cmp(other)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Unlinked
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Non-nullable Pointer type that has been successfully unlinked from a concurrent data structure
/// by a swap or compare-and-swap operation.
///
/// This implies that no threads can acquire any new references to this value anymore, but there may
/// still be live references to it that have acquired before the unlink operation has been made.
/// As long as the invariant that no unique value is inserted more than once in the same data
/// structure, it is safe to reclaim `Unlinked` types.
pub struct Unlinked<T, N: Unsigned, R: Reclaim> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<(T, R)>,
}

impl<T, N: Unsigned, R: Reclaim> Unlinked<T, N, R> {
    /// TODO: Doc...
    #[inline]
    pub unsafe fn from_marked(marked: MarkedPtr<T, N>) -> Option<Self> {
        mem::transmute(marked)
    }

    /// TODO: Doc...
    #[inline]
    pub unsafe fn from_marked_non_null(marked: MarkedNonNull<T, N>) -> Self {
        Self {
            inner: marked,
            _marker: PhantomData,
        }
    }

    /// TODO: Doc...
    #[inline]
    pub fn as_marked(&self) -> MarkedPtr<T, N> {
        self.inner.into_marked()
    }

    /// TODO: Doc...
    #[inline]
    pub fn into_marked(self) -> MarkedPtr<T, N> {
        self.inner.into_marked()
    }

    /// TODO: Doc...
    #[inline]
    pub fn into_marked_non_null(self) -> MarkedNonNull<T, N> {
        self.inner
    }

    /// TODO: Doc...
    #[inline]
    pub unsafe fn deref(&self) -> &T {
        self.inner.as_ref()
    }
}
