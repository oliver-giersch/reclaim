//! An abstract interface for concurrent memory reclamation that is based on traits.

#![feature(const_fn)]
#![cfg_attr(not(feature = "with_std"), feature(alloc))]
#![cfg_attr(not(any(test, feature = "with_std")), no_std)]

#[cfg(not(feature = "with_std"))]
extern crate alloc;

use core::marker::PhantomData;
use core::mem;
use core::ptr::NonNull;
use core::sync::atomic::Ordering;

// TODO: replace with const generics once available
pub use typenum::{
    Unsigned, U0, U1, U10, U11, U12, U13, U14, U15, U16, U17, U18, U19, U2, U20, U21, U22, U23,
    U24, U25, U26, U27, U28, U29, U3, U4, U5, U6, U7, U8, U9,
};

use memoffset::offset_of;

pub mod align;
pub mod leak;

mod atomic;
mod marked;
mod owned;
mod pointer;

pub use crate::atomic::{Atomic, CompareExchangeFailure};
pub use crate::marked::{AtomicMarkedPtr, MarkedNonNull, MarkedPtr};
pub use crate::owned::Owned;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Reclaim (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Doc...
pub unsafe trait Reclaim
where
    Self: Sized,
{
    /// Every reclaimable record allocates this type alongside to store additional reclamation
    /// scheme specific data. When no such data is necessary, `()` is the recommended choice.
    type RecordHeader: Default + Sized;

    /// Retires a record and caches it **at least** until it is safe to deallocate it, i.e. when no
    /// other threads can possibly have any live references to it.
    ///
    /// # Safety
    ///
    /// The caller has to guarantee that the record is *actually* unlinked from its data structure,
    /// i.e. there is no way for another thread to acquire a new reference to it. While an
    /// `Unlinked` can only be safely obtained by atomic operations that do in fact unlink a value,
    /// it is still possible to enter the same record twice into the same data structure using only
    /// safe code.
    unsafe fn retire<T, N: Unsigned>(unlinked: Unlinked<T, N, Self>)
    where
        T: 'static;

    /// Retires a record and caches it **at least** until it is safe to deallocate it, i.e. when no
    /// other threads can possibly have any live references to it.
    ///
    /// # Safety
    ///
    /// The caller has to guarantee that the record is *actually* unlinked from its data structure,
    /// i.e. there is no way for another thread to acquire a new reference to it. While an
    /// `Unlinked` can only be safely obtained by atomic operations that do in fact unlink a value,
    /// it is still possible to enter the same record twice into the same data structure using only
    /// safe code.
    ///
    /// In addition to these invariant, this method additionally requires the caller to ensure the
    /// `Drop` implementation for `T` (if any) does not access any **non-static** references. The
    /// `Reclaim` interface makes no guarantees about the precise time a retired record is actually
    /// reclaimed. Hence it is not possible to ensure any references stored within the record have
    /// not become invalid.
    unsafe fn retire_unchecked<T, N: Unsigned>(unlinked: Unlinked<T, N, Self>);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Protect (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Doc...
pub trait Protect
where
    Self: Sized,
{
    /// Generic type of value protected from reclamation
    type Item: Sized;
    /// Number of bits available for storing additional information
    type MarkBits: Unsigned;
    /// Reclamation scheme associated with this type of guard
    type Reclaimer: Reclaim;

    /// Creates a new empty instance of `Self`.
    fn new() -> Self;

    /// Gets a shared reference to the protected value that is tied to the lifetime of `&self`
    fn shared(&self) -> Option<Shared<Self::Item, Self::MarkBits, Self::Reclaimer>>;

    /// Atomically takes a snapshot of `atomic`'s value and returns a shared and protected reference
    /// to it.
    ///
    /// The loaded value is stored within self. If the value of `atomic` is null, no protection
    /// occurs. Any previously protected value is no longer protected, regardless of the loaded
    /// value.
    fn acquire(
        &mut self,
        atomic: &Atomic<Self::Item, Self::MarkBits, Self::Reclaimer>,
        order: Ordering,
    ) -> Option<Shared<Self::Item, Self::MarkBits, Self::Reclaimer>>;

    /// Atomically takes a snapshot of `atomic`'s value and returns a shared and protected reference
    /// to it if the loaded value is equal to `expected`.
    ///
    /// A successfully loaded value is stored within self. If the value of `atomic` is null, no
    /// protection occurs. After a successful load, any previously protected value is no longer
    /// protected, regardless of the loaded value. In case of a unsuccessful load, the previously
    /// protected value does not change.
    fn acquire_if_equal(
        &mut self,
        atomic: &Atomic<Self::Item, Self::MarkBits, Self::Reclaimer>,
        expected: MarkedPtr<Self::Item, Self::MarkBits>,
        order: Ordering,
    ) -> Result<Option<Shared<Self::Item, Self::MarkBits, Self::Reclaimer>>, NotEqual>;

    /// Releases the currently protected value, which is no longer guaranteed to be protected
    /// afterwards.
    fn release(&mut self);
}

/// A zero-size "marker" type that represents the failure state of an `acquire_if_equal` operation.
#[derive(Debug, Default)]
pub struct NotEqual;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Record
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Record type that is managed by a specific reclamation scheme.
///
/// Whenever a new `Owned` or (non-null) `Atomic` is created a value of this type is allocated on
/// the heap. The record and its header is never exposed to the data structure using a given memory
/// reclamation scheme and should only be accessed by the reclamation scheme itself.
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

    /// Creates a new record with the specified `elem` and `header`.
    #[inline]
    pub fn with_header(elem: T, header: R::RecordHeader) -> Self {
        Self { header, elem }
    }

    /// Gets a reference to header for the record at the pointed-to location of `elem`.
    ///
    /// # Safety
    ///
    /// The pointer `elem` must be a valid pointer to an instance of `T` that was allocated as part
    /// of a `Record`. Otherwise, the pointer arithmetic used to calculate the header's address will
    /// be incorrect and lead to undefined behavior.
    #[inline]
    pub unsafe fn get_header<'a>(elem: NonNull<T>) -> &'a R::RecordHeader {
        let header = (elem.as_ptr() as usize) - Self::offset_elem() + Self::offset_header();
        &*(header as *mut _)
    }

    /// Gets mutable a reference to header for the record at the pointed-to location of `elem`.
    ///
    /// # Safety
    ///
    /// The pointer `elem` must be a valid pointer to an instance of `T` that was allocated as part
    /// of a `Record`. Otherwise, the pointer arithmetic used to calculate the header's address will
    /// be incorrect and lead to undefined behavior.
    /// Additionally the caller has to ensure the aliasing rules are not violated by creating a
    /// mutable record.
    #[inline]
    pub unsafe fn get_header_mut<'a>(elem: NonNull<T>) -> &'a mut R::RecordHeader {
        let header = (elem.as_ptr() as usize) - Self::offset_elem() + Self::offset_header();
        &mut *(header as *mut _)
    }

    /// Gets the pointer to the record belonging to the element pointed to by `elem`.
    ///
    /// # Safety
    ///
    /// The pointer `elem` must be a valid pointer to an instance of `T` that was allocated as part
    /// of a `Record`. Otherwise, the pointer arithmetic used to calculate the header's address will
    /// be incorrect and lead to undefined behavior.
    #[inline]
    pub unsafe fn get_record(elem: NonNull<T>) -> NonNull<Self> {
        let record = (elem.as_ptr() as usize) - Self::offset_elem();
        NonNull::new_unchecked(record as *mut _)
    }

    /// Gets the offset in bytes from the address of a record to its header field.
    #[inline]
    pub fn offset_header() -> usize {
        offset_of!(Self, header)
    }

    /// Gets the offset in bytes from the address of a record to its element field.
    #[inline]
    pub fn offset_elem() -> usize {
        offset_of!(Self, elem)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Shared
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A shared reference to a value that is actively protected from reclamation by other threads.
#[derive(Eq, Ord)]
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
    /// TODO: Doc...
    #[inline]
    pub fn with_tag(shared: Self, tag: usize) -> Self {
        Self {
            inner: MarkedNonNull::compose(shared.inner.decompose_non_null(), tag),
            _marker: PhantomData,
        }
    }

    /// TODO: Doc + Test...
    #[inline]
    pub fn wrapping_add_tag(shared: Self, add: usize) -> Self {
        Self::with_tag(shared, shared.tag().wrapping_add(add))
    }

    /// TODO: Doc + Test...
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
    pub unsafe fn deref(&self) -> &'g T {
        self.inner.as_ref()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Unlinked
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A reference to a value that has been unlinked and is hence no longer reachable by other threads.
///
/// `Unlinked` values are the result of (successful) atomic swap or compare-and-swap operations.
/// Concurrent data structure implementations have to maintain the invariant that no unique value
/// (pointer) can exist more than once within any data structure. Only then is it safe to `retire`
/// unlinked values, enabling them to be eventually reclaimed. Note that, while it must be
/// impossible for other threads to create new references to an already unlinked value, it is
/// possible and explicitly allowed for other threads to still have live references that have been
/// created before the value was unlinked.
#[derive(Eq, Ord)]
pub struct Unlinked<T, N: Unsigned, R: Reclaim> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<(T, R)>,
}

impl<T, N: Unsigned, R: Reclaim> Unlinked<T, N, R> {
    /// TODO: Doc...
    #[inline]
    pub unsafe fn deref(&self) -> &T {
        self.inner.as_ref()
    }

    /// TODO: Doc...
    #[inline]
    pub unsafe fn retire(self)
    where
        T: 'static,
    {
        R::retire(self)
    }

    /// TODO: Doc...
    #[inline]
    pub unsafe fn retire_unchecked(self) {
        R::retire_unchecked(self)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Unprotected
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Eq, Ord)]
pub struct Unprotected<T, N: Unsigned, R: Reclaim> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<(R)>,
}

impl<T, N: Unsigned, R: Reclaim> Clone for Unprotected<T, N, R> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
            _marker: PhantomData,
        }
    }
}

impl<T, N: Unsigned, R: Reclaim> Copy for Unprotected<T, N, R> {}

impl<T, N: Unsigned, R: Reclaim> Unprotected<T, N, R> {
    /// TODO: Doc...
    ///
    /// # Safety
    ///
    /// This is generally unsound to call. Only when the caller is able to ensure no memory
    /// reclamation is happening concurrently can it be considered to be safe to dereference an
    /// unprotected pointer loaded from a concurrent data structure. This is e.g. the case when
    /// there are mutable references involved (e.g. during `drop`).
    #[inline]
    pub unsafe fn deref_unprotected(&self) -> &T {
        self.inner.as_ref()
    }
}
