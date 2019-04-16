//! An abstract & unified interface supporting various schemes for concurrent memory reclamation.
//!
//! # Memory Management in Rust
//!
//! ...
//!
//! # The Need for Automatic Memory Reclamation
//!
//! ...
//!
//! # Concurrent & Lock-Free Gargage Collection
//!
//! ...
//!
//! # The `Reclaim` Interface
//!
//! ...
//!
//! # Pointer Types & Tagging
//!
//! ...

#![cfg_attr(not(feature = "std"), feature(alloc))]
#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![warn(missing_docs)]

#[cfg(not(feature = "std"))]
extern crate alloc;

use core::marker::PhantomData;
use core::ptr::NonNull;
use core::sync::atomic::Ordering;

// TODO: replace with const generics once available
pub use typenum;

use memoffset::offset_of;
use typenum::Unsigned;

// module must be declared first in order to correctly use the macros defined inside
#[macro_use]
mod pointer;

pub mod align;
pub mod leak;
pub mod prelude {
    //! Useful and/or required traits for this crate.
    pub use crate::MarkedPointer;
    pub use crate::Protect;
    pub use crate::Reclaim;
}

mod atomic;
mod marked;
mod owned;
mod shared;
mod unlinked;
mod unprotected;

pub use crate::atomic::{Atomic, CompareExchangeFailure};
pub use crate::marked::{AtomicMarkedPtr, MarkedNonNull, MarkedPtr};
pub use crate::pointer::MarkedPointer;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Reclaim (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait for retiring and reclaiming entries in concurrent collections and data structures.
pub unsafe trait Reclaim
where
    Self: LocalReclaim,
{
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
    unsafe fn retire<T: 'static, N: Unsigned>(unlinked: Unlinked<T, Self, N>);

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
    unsafe fn retire_unchecked<T, N: Unsigned>(unlinked: Unlinked<T, Self, N>);
}

unsafe impl<R> Reclaim for R
where
    R: LocalReclaim<Local = ()>,
{
    #[inline]
    unsafe fn retire<T: 'static, N: Unsigned>(unlinked: Unlinked<T, Self, N>) {
        Self::retire_local(&(), unlinked)
    }

    #[inline]
    unsafe fn retire_unchecked<T, N: Unsigned>(unlinked: Unlinked<T, Self, N>) {
        Self::retire_local_unchecked(&(), unlinked)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// RetireLocal (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Doc...
pub unsafe trait LocalReclaim
where
    Self: Sized,
{
    // TODO: type Allocator;
    // TODO: type Guarded<T, const N: usize>: Protect<Item = T, MARK_BITS = N>;

    /// TODO: Doc...
    type Local: Sized;

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
    unsafe fn retire_local<T: 'static, N: Unsigned>(
        local: &Self::Local,
        unlinked: Unlinked<T, Self, N>,
    );

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
    unsafe fn retire_local_unchecked<T, N: Unsigned>(
        local: &Self::Local,
        unlinked: Unlinked<T, Self, N>,
    );
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Protect (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait for representing pointer types that *protect* their pointed-to value from reclamation
/// during their lifetime.
pub unsafe trait Protect
where
    Self: Sized + Clone,
{
    /// Generic type of value protected from reclamation
    type Item: Sized;
    /// Reclamation scheme associated with this type of guard
    type Reclaimer: LocalReclaim;
    /// Number of bits available for storing additional information
    type MarkBits: Unsigned;

    /// Gets a shared reference to the protected value that is tied to the lifetime of `&self`
    fn shared(&self) -> Option<Shared<Self::Item, Self::Reclaimer, Self::MarkBits>>;

    /// Atomically takes a snapshot of `atomic`'s value and returns a shared and protected reference
    /// to it.
    ///
    /// The loaded value is stored within self. If the value of `atomic` is null, no protection
    /// occurs. Any previously protected value is no longer protected, regardless of the loaded
    /// value.
    fn acquire(
        &mut self,
        atomic: &Atomic<Self::Item, Self::Reclaimer, Self::MarkBits>,
        order: Ordering,
    ) -> Option<Shared<Self::Item, Self::Reclaimer, Self::MarkBits>>;

    /// Atomically takes a snapshot of `atomic`'s value and returns a shared and protected reference
    /// to it if the loaded value is equal to `expected`.
    ///
    /// A successfully loaded value is stored within self. If the value of `atomic` is null, no
    /// protection occurs. After a successful load, any previously protected value is no longer
    /// protected, regardless of the loaded value. In case of a unsuccessful load, the previously
    /// protected value does not change.
    ///
    /// # Errors
    ///
    /// This method returns an error result ([`NotEqual`](NotEqual)) if the atomically loaded
    /// snapshot from `atomic` does not match the `expected` value.
    fn acquire_if_equal(
        &mut self,
        atomic: &Atomic<Self::Item, Self::Reclaimer, Self::MarkBits>,
        expected: MarkedPtr<Self::Item, Self::MarkBits>,
        order: Ordering,
    ) -> AcquireResult<Self::Item, Self::Reclaimer, Self::MarkBits>;

    /// Releases the currently protected value, which is no longer guaranteed to be protected
    /// afterwards.
    fn release(&mut self);
}

/// Result type for [`acquire_if_equal`](Protect::acquire_if_equal) operations.
pub type AcquireResult<'g, T, R, N> = Result<Option<Shared<'g, T, R, N>>, NotEqual>;

/// A zero-size marker type that represents the failure state of an `acquire_if_equal` operation.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct NotEqual;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Record
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Record type that is managed by a specific reclamation scheme.
///
/// Whenever a new `Owned` or (non-null) `Atomic` is created a value of this type is allocated on
/// the heap. The record and its header is never exposed to the data structure using a given memory
/// reclamation scheme and should only be accessed by the reclamation scheme itself.
pub struct Record<T, R: LocalReclaim> {
    header: R::RecordHeader,
    elem: T,
}

impl<T, R: LocalReclaim> Record<T, R> {
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
        debug_assert_ne!(record, 0);
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
// Owned
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A pointer type for heap allocation like [`Box`](std::boxed::Box) that can be marked and is
/// guaranteed to allocate the appropriate [`Record`] type for its generic [`Reclaim`] type
/// parameter alongside its owned value.
#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct Owned<T, R: LocalReclaim, N: Unsigned> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<(T, R)>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Shared
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A shared reference to a value that is actively protected from reclamation by other threads.
///
/// Valid `Shared` values are always borrowed from guard values implementing the
/// [`Protect`](Protect) trait.
#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct Shared<'g, T, R, N> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<(&'g T, R)>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Unlinked
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A reference to a value that has been unlinked and is hence no longer reachable by other threads.
///
/// `Unlinked` values are the result of (successful) atomic swap or compare-and-swap operations on
/// [`Atomic`](Atomic) values.
///
/// Concurrent data structure implementations have to maintain the invariant that no unique value
/// (pointer) can exist more than once within any data structure. Only then is it safe to `retire`
/// unlinked values, enabling them to be eventually reclaimed. Note that, while it must be
/// impossible for other threads to create new references to an already unlinked value, it is
/// possible and explicitly allowed for other threads to still have live references that have been
/// created before the value was unlinked.
#[derive(Eq, Ord, PartialEq, PartialOrd)]
#[must_use = "unlinked values are meant to be retired, otherwise a memory leak is highly likely"]
pub struct Unlinked<T, R, N> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<(T, R)>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Unprotected
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A non-null value loaded from an [`Atomic`](Atomic) but without any guarantees
/// protecting it from reclamation.
#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct Unprotected<T, R, N> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<R>,
}
