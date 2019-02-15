#![feature(allocator_api, trait_alias)]
#![cfg_attr(not(feature = "global_alloc"), no_std)]

use core::alloc::Alloc;
use core::marker::PhantomData;
use core::sync::atomic::Ordering;

#[cfg(feature = "global_alloc")]
use std::alloc::Global;

use memoffset::offset_of;

mod atomic;
mod marked;
mod owned;
mod pointer;

pub use crate::atomic::{Atomic, Compare, CompareExchangeFailure, Store};
pub use crate::marked::{AtomicMarkedPtr, MarkedNonNull, MarkedPtr};
pub use crate::owned::Owned;

pub trait StatelessAlloc = Alloc + Copy + Clone + Default;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Reclaim (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait Reclaim<A: StatelessAlloc>
where
    Self: Sized,
{
    /// Header that prepends every record. For reclamation Schemes that do not
    /// require any header data for records managed by them, `()` is the best
    /// choice.
    type RecordHeader: Default + Sized;

    /// TODO: Doc
    fn allocator() -> A {
        A::default()
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
    unsafe fn reclaim<T>(unlinked: Unlinked<T, Self, A>)
    where
        T: 'static;

    /// Reclaims a record and de-allocates it when no other threads hold any references to it.
    ///
    /// # Safety
    ///
    /// The reclamation scheme ensures to only call a type's `Drop` implementation after its
    /// reclamation. The caller has to ensure that the `drop` method does not use any non-static
    /// references contained in the type.
    unsafe fn reclaim_unchecked<T>(unlinked: Unlinked<T, Self, A>);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Record
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Type that is allocated, whenever a new `Owned<T>` or `Atomic<T>` is created.
///
/// The header is never exposed and has to be manually accessed if and when needed.
/// One example use case could be reference-counted records, where the count is stored
/// in the header and increased or decreased whenever a `Protected` is acquired or goes
/// out of scope.
pub struct Record<T, R: Reclaim<A>, A: StatelessAlloc> {
    header: R::RecordHeader,
    elem: T,
}

impl<T, R: Reclaim<A>, A: StatelessAlloc> Record<T, R, A> {
    /// Creates a new record with the specified `elem` and a default header.
    pub fn new(elem: T) -> Self {
        Self {
            header: Default::default(),
            elem,
        }
    }

    /// Creates a new record with the specified `header` and `elem`.
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
    pub unsafe fn get_header<'a>(elem: *mut T) -> &'a R::RecordHeader {
        let header = (elem as usize) - Self::offset_elem() + Self::offset_header();
        &*(header as *mut _)
    }

    pub unsafe fn get_header_mut<'a>(elem: *mut T) -> &'a mut R::RecordHeader {
        let header = (elem as usize) - Self::offset_elem() + Self::offset_header();
        &mut *(header as *mut _)
    }

    /// TODO: Doc...
    pub unsafe fn get_record(elem: *mut T) -> *mut Self {
        let record = (elem as usize) - Self::offset_elem();
        record as *mut _
    }

    /// TODO: Doc...
    pub fn offset_header() -> usize {
        offset_of!(Self, header)
    }

    /// TODO: Doc...
    pub fn offset_elem() -> usize {
        offset_of!(Self, elem)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Protect (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Doc...
pub trait Protected<A: StatelessAlloc>: Drop + Sized {
    /// TODO: Doc...
    type Item: Sized;
    /// TODO: Doc...
    type Reclaimer: Reclaim<A>;

    /// Creates a new `Protected`.
    ///
    /// In case of region-based reclamation schemes (such as EBR), a call to `new` is guaranteed
    /// to create an active region guard.
    fn new() -> Self;

    /// Returns a `Shared` value wrapped in a `Some` from the internally protected pointer. If no
    /// value or a null-pointer has been acquired, `None` is returned.
    /// The `Shared` that is returned is guaranteed to be protected from reclamation during the
    /// lifetime of the `Protected`.
    fn shared(&self) -> Option<Shared<Self::Item, Self::Reclaimer, A>>;

    /// Takes an atomic snapshot of the value stored within `atomic` at the moment of the call's
    /// invocation and stores it within `self`. The corresponding `Shared` wrapped in a `Some` (or
    /// `None`) is returned.
    ///
    /// The successfully acquired value is guaranteed to be protected from concurrent reclamation.
    fn acquire(
        &mut self,
        atomic: &Atomic<Self::Item, Self::Reclaimer, A>,
        order: Ordering,
    ) -> Option<Shared<Self::Item, Self::Reclaimer, A>>;

    /// TODO: Doc...
    fn acquire_if_equal(
        &mut self,
        atomic: &Atomic<Self::Item, Self::Reclaimer, A>,
        compare: MarkedPtr<Self::Item>,
        order: Ordering,
    ) -> Result<Option<Shared<Self::Item, Self::Reclaimer, A>>, NotEqual>;

    /// TODO: Doc...
    fn release(&mut self);
}

/// A ZST struct that represents the failure state of an `acquire_if_equal` operation.
pub struct NotEqual;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Shared
////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(feature = "global_alloc")]
/// A `Shared` represents a reference to value stored in a concurrent data structure.
pub struct Shared<'g, T, R: Reclaim<A>, A: StatelessAlloc = Global> {
    inner: MarkedNonNull<T>,
    _marker: PhantomData<(&'g T, R, A)>,
}

#[cfg(not(feature = "global_alloc"))]
/// A `Shared` represents a reference to value stored in a concurrent data structure.
pub struct Shared<'g, T, R: Reclaim<A>, A: Alloc> {
    inner: MarkedNonNull<T>,
    _marker: PhantomData<(&'g T, R, A)>,
}

impl<'g, T, R: Reclaim<A>, A: StatelessAlloc> Clone for Shared<'g, T, R, A> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
            _marker: PhantomData,
        }
    }
}

impl<'g, T, R: Reclaim<A>, A: StatelessAlloc> Copy for Shared<'g, T, R, A> {}

impl<'g, T, R: Reclaim<A>, A: StatelessAlloc> Shared<'g, T, R, A> {
    /// # Safety
    ///
    /// The caller must ensure that the provided pointer is both non-null and valid (may be marked)
    /// and is guaranteed to not be reclaimed during the lifetime of the shared reference.
    pub unsafe fn from_raw(raw: *mut T) -> Self {
        debug_assert!(!raw.is_null());
        Self {
            inner: MarkedNonNull::new_unchecked(raw),
            _marker: PhantomData,
        }
    }

    /// TODO: Doc...
    pub fn into_raw(self) -> *mut T {
        self.inner.into_inner().as_ptr()
    }

    /// TODO: Doc...
    pub fn with_tag(shared: Self, tag: usize) -> Self {
        Self {
            inner: MarkedNonNull::compose(shared.inner.decompose_non_null(), tag),
            _marker: PhantomData,
        }
    }

    /// TODO: Doc...
    pub fn wrapping_add_tag(shared: Self, add: usize) -> Self {
        Self::with_tag(shared, shared.tag().wrapping_add(add))
    }

    /// TODO: Doc...
    pub fn wrapping_sub_tag(shared: Self, sub: usize) -> Self {
        Self::with_tag(shared, shared.tag().wrapping_sub(sub))
    }

    /// TODO: Doc...
    pub fn tag(&self) -> usize {
        self.inner.decompose_tag()
    }

    /// TODO: Doc...
    pub unsafe fn deref(&self) -> &T {
        &*self.inner.decompose_ptr()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Unlinked
////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(feature = "global_alloc")]
pub struct Unlinked<T, R: Reclaim<A>, A: StatelessAlloc = Global> {
    inner: MarkedNonNull<T>,
    _marker: PhantomData<(T, R, A)>,
}

#[cfg(not(feature = "global_alloc"))]
pub struct Unlinked<T, R: Reclaim<A>, A: Alloc> {
    inner: MarkedNonNull<T>,
    _marker: PhantomData<(T, R, A)>,
}

impl<T, R: Reclaim<A>, A: StatelessAlloc> Unlinked<T, R, A> {
    /// TODO: Doc...
    pub unsafe fn from_raw(raw: *mut T) -> Self {
        Self {
            inner: MarkedNonNull::new_unchecked(raw),
            _marker: PhantomData,
        }
    }

    /// Consumes the `unlinked` and returns the internal non-null (potentially marked) raw pointer.
    pub fn into_raw(unlinked: Self) -> *mut T {
        unlinked.inner.into_inner().as_ptr()
    }

    /// TODO: Doc...
    pub unsafe fn deref(&self) -> &T {
        self.inner.as_ref()
    }
}