#![feature(allocator_api)]
#![cfg_attr(feature = "no_std", no_std)]

use core::alloc::Alloc;
use core::marker::PhantomData;
use core::sync::atomic::Ordering;

#[cfg(not(feature = "no_std"))]
use std::alloc::Global;

use cfg_if::cfg_if;
use memoffset::offset_of;

mod atomic;
mod marked;
mod owned;
mod pointer;

pub use crate::atomic::{Compare, CompareExchangeFailure, Store};
pub use crate::marked::{AtomicMarkedPtr, MarkedNonNull, MarkedPtr};

cfg_if! {
    if #[cfg(feature = "no_std")] {
        pub use crate::atomic::Atomic;
        pub use crate::owned::Owned;
    } else {
        pub type Atomic<T, R, A = Global> = crate::atomic::Atomic<T, R, A>;
        pub type Owned<T, R, A = Global> = crate::owned::Owned<T, R, A>;
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Reclaim (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub unsafe trait Reclaim<A: Alloc>
where
    Self: Sized,
{
    /// Header that prepends every record.
    type RecordHeader: Default + Sized;

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

/// TODO: Doc...
pub struct Record<T, R: Reclaim<A>, A: Alloc> {
    header: R::RecordHeader,
    elem: T,
}

impl<T, R: Reclaim<A>, A: Alloc> Record<T, R, A> {
    /// TODO: Doc...
    pub fn new(elem: T) -> Self {
        Self {
            header: Default::default(),
            elem,
        }
    }

    /// TODO: Doc...
    pub unsafe fn get_header<'a>(elem: *mut T) -> &'a R::RecordHeader {
        let header = (elem as usize) - Self::offset_elem() + Self::offset_header();
        &*(header as *mut _)
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
pub trait Protected<A: Alloc>: Drop + Sized {
    /// TODO: Doc...
    type Item: Sized;
    /// TODO: Doc...
    type Reclaimer: Reclaim<A>;

    /// TODO: Doc...
    fn new() -> Self;

    /// Returns a `Shared` value wrapped in a `Some` from the protected pointer that is safe
    /// from reclamation during the lifetime of `&self`, but only if a non-null value was previously
    /// acquired. Otherwise, `None` is returned.
    fn shared(&self) -> Option<Shared<Self::Item, Self::Reclaimer, A>>;

    /// TODO: Doc...
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

/// A `Shared` represents a reference to value stored in a concurrent data structure.
pub struct Shared<'g, T, R: Reclaim<A>, A: Alloc> {
    inner: MarkedNonNull<T>,
    _marker: PhantomData<(&'g R, A)>,
}

impl<'g, T, R: Reclaim<A>, A: Alloc> Clone for Shared<'g, T, R, A> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
            _marker: PhantomData,
        }
    }
}

impl<'g, T, R: Reclaim<A>, A: Alloc> Copy for Shared<'g, T, R, A> {}

impl<'g, T, R: Reclaim<A>, A: Alloc> Shared<'g, T, R, A> {
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

pub struct Unlinked<T, R: Reclaim<A>, A: Alloc> {
    inner: MarkedNonNull<T>,
    _marker: PhantomData<(R, A)>,
}

impl<T, R: Reclaim<A>, A: Alloc> Unlinked<T, R, A> {
    /// TODO: Doc...
    pub unsafe fn from_raw(raw: *mut T) -> Self {
        unimplemented!()
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
