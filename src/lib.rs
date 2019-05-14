//! An abstract & generalized interface supporting various schemes for
//! concurrent memory reclamation.
//!
//! # Memory Management in Rust
//!
//! Unlike garbage-collected languages such as *Go* and *Java*, memory
//! management in *Rust* is ultimately manual and more akin to *C++*.
//! Rust's ownership model in combination with standard library's smart pointer
//! types `Box`, `Rc` and `Arc` make memory management as painless as possible
//! and are able to handle the vast majority of use-cases.
//! Consequently, there is usually little need for the small additional comfort
//! provided by a fully automated **Garbage Collector** (GC).
//!
//! ## The Need for Automatic Memory Reclamation
//!
//! In the domain of concurrent lock-free data structures, however, the
//! aforementioned memory management schemes are insufficient for determining,
//! when a removed entry can be actually dropped and deallocated:
//! Just because an entry has been removed (unlinked) from some list, it can
//! usually not be guaranteed, that no other thread could be in the process of
//! reading that entry at the same time, even after the entry has been removed.
//! The only thing that can be ascertained, due to nature of atomic *swap* and
//! *compare-and-swap* operations, is that other threads can not acquire *new*
//! references after an entry has been removed.
//!
//! With atomic reference counting (as it's done by e.g. the `Arc` type), it is
//! principally possible to determine when ...
//!
//! ## Concurrent & Lock-Free Garbage Collection
//!
//! Concurrent memory reclamation schemes work by granting every value
//! (*record*) earmarked for deletion (*retired*) a certain **grace period**
//! before being actually dropped and deallocated, during which it is cached and
//! can still be safely read by other threads with live references to it.
//! How to determine the length of this grace period is up to each individual
//! reclamation scheme
//!
//! # The `Reclaim` Interface
//!
//! ...
//!
//! # Pointer Types & Tagging
//!
//! ...

// TODO: remove once 1.35 is there
#![cfg_attr(not(feature = "std"), feature(alloc))]
#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![warn(missing_docs)]

#[cfg(not(feature = "std"))]
extern crate alloc;

use core::fmt;
use core::marker::PhantomData;
use core::ptr::NonNull;
use core::sync::atomic::Ordering;

// TODO: replace with const generics once available
pub use typenum;

use memoffset::offset_of;
use typenum::Unsigned;

#[macro_use]
mod macros;

pub mod align;
pub mod leak;
pub mod prelude {
    //! Useful and/or required types and traits for this crate.

    pub use crate::pointer::{
        Marked::{self, Null, Value},
        MarkedPointer,
        NonNullable,
    };

    pub use crate::LocalReclaim;
    pub use crate::Protect;
    pub use crate::Reclaim;
}

mod atomic;
mod owned;
mod pointer;
mod shared;
mod unlinked;
mod unprotected;

pub use crate::atomic::{Atomic, CompareExchangeFailure};
pub use crate::pointer::{AtomicMarkedPtr, Marked, MarkedNonNull, MarkedPointer, MarkedPtr};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Reclaim (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait for retiring and reclaiming entries removed from concurrent
/// collections and data structures.
///
/// Implementing this trait requires first implementing the [`LocalReclaim`]
/// trait for the same type and is usually only possible in `std` environments
/// with access to thread local storage.
pub unsafe trait Reclaim
where
    Self: LocalReclaim,
{
    /// Retires a record and caches it **at least** until it is safe to
    /// deallocate it.
    ///
    /// For further information, refer to the documentation of
    /// [`retire_local`][`LocalReclaim::retire_local`].
    ///
    /// # Safety
    ///
    /// The same caveats as with [`retire_local`][`LocalReclaim::retire_local`]
    /// apply.
    unsafe fn retire<T: 'static, N: Unsigned>(unlinked: Unlinked<T, Self, N>);

    /// Retires a record and caches it **at least** until it is safe to
    /// deallocate it.
    ///
    /// For further information, refer to the documentation of
    /// [`retire_local_unchecked`][`LocalReclaim::retire_local_unchecked`].
    ///
    /// # Safety
    ///
    /// The same caveats as with [`retire_local`][`LocalReclaim::retire_local`]
    /// apply.
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

/// A trait, which constitutes the foundation for the [`Reclaim`] trait and
/// requires explicit references to thread local storage for caching retired
/// records.
///
/// This type is specifically designed to be compatible with `#[no_std]`
/// environments, where implicit access to thread local static variables is not
/// possible.
/// If a reclamation scheme does not require or deliberately chooses to avoid
/// using thread local storage for the sake of simplicity or portability
/// (usually at the cost of performance), it is valid to implement `Self::Local`
/// as `()` and pass all retired records directly through to global storage.
/// Note, that this will generally require more and also more frequent
/// synchronization.
pub unsafe trait LocalReclaim
where
    Self: Sized,
{
    // TODO: type Allocator;
    // TODO: type Guarded<T, const N: usize>: Protect<Item = T, MARK_BITS = N>;

    /// The type used for storing all relevant thread local state.
    type Local: Sized;

    /// Every record allocates this type alongside itself to store additional
    /// reclamation scheme specific data.
    /// When no such data is required, `()` is the recommended choice.
    type RecordHeader: Default + Sync + Sized;

    /// Retires a record and caches it **at least** until it is safe to
    /// deallocate it, i.e. when no other threads can possibly have any live
    /// references to it.
    ///
    /// # Safety
    ///
    /// The caller has to guarantee that the record is *actually* unlinked from
    /// its data structure, i.e. there is no way for another thread to acquire a
    /// new reference to it.
    /// While an `Unlinked` can only be safely obtained by atomic operations
    /// that do in fact unlink a value, it is still possible to enter the same
    /// record twice into the same data structure using only safe code.
    ///
    /// This variant also mandates, that correct synchronization of atomic
    /// operations around calls to functions that retire records is ensured.
    /// Consider the following (incorrect) example:
    ///
    /// ```ignore
    /// use core::sync::atomic::Ordering;
    ///
    /// let g = Atomic::from(Owned::new(1));
    ///
    /// // thread 1
    /// if let Some(shared) = g.load(Ordering::Relaxed, &mut guard) {
    ///     assert_eq!(*shared, &1);
    /// }
    ///
    /// // thread 2
    /// let expected = g.load_unprotected(Ordering::Relaxed); // reads &1
    /// let unlinked = g.compare_exchange(
    ///     expected,
    ///     Owned::null(),
    ///     Ordering::Relaxed,
    ///     Ordering::Relaxed
    /// ).unwrap();
    ///
    /// unsafe { unlinked.retire() };
    /// ```
    ///
    /// In this example, the invariant can not be guaranteed to be maintained,
    /// due to the incorrect (relaxed) memory orderings.
    /// Thread 2 can potentially unlink the shared value, retire and reclaim it,
    /// without the `compare_exchange` operation ever becoming visible to
    /// thread 1. The thread could then proceed to load and read the previous
    /// value instead of the inserted `null`, accessing freed memory.
    unsafe fn retire_local<T: 'static, N: Unsigned>(
        local: &Self::Local,
        unlinked: Unlinked<T, Self, N>,
    );

    /// Retires a record and caches it **at least** until it is safe to
    /// deallocate it, i.e. when no other threads can possibly have any live
    /// references to it.
    ///
    /// # Safety
    ///
    /// The same restrictions as with the [`retire_local`][LocalReclaim::retire_local]
    /// apply to this method as well.
    ///
    /// In addition to these invariants, this method additionally requires the
    /// caller to ensure the `Drop` implementation for `T` (if there is one) or
    /// any contained type does not access any **non-static** references.
    /// The `Reclaim` interface makes no guarantees about the precise time a
    /// retired record is actually reclaimed.
    /// Hence it is not possible to ensure any references stored within the
    /// record have not become invalid.
    unsafe fn retire_local_unchecked<T, N: Unsigned>(
        local: &Self::Local,
        unlinked: Unlinked<T, Self, N>,
    );
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Protect (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait applicable for pointer types that *protect* their pointed-to value
/// from reclamation during their lifetime.
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

    /// Gets a [`Shared`] reference to the protected value, which is tied to the
    /// lifetime of `self`.
    fn shared(&self) -> Option<Shared<Self::Item, Self::Reclaimer, Self::MarkBits>> {
        self.marked().value()
    }

    /// Gets a [`Shared`] reference wrapped in a [`Marked`] for the protected
    /// value, which is tied to the lifetime of `self`.
    fn marked(&self) -> Marked<Shared<Self::Item, Self::Reclaimer, Self::MarkBits>>;

    /// Atomically takes a snapshot of `atomic` and returns a protected
    /// [`Shared`] reference wrapped in a [`Marked`] to it.
    ///
    /// The loaded value is stored within `self`. If the value of `atomic` is
    /// `null` or a pure tag (marked `null` pointer), no protection has to be
    /// established. Any previously protected value will be overwritten and be
    /// no longer protected, regardless of the loaded value.
    ///
    /// # Panics
    ///
    /// **May** panic if `order` is [`Acquire`][acquire] or [`AcqRel`][acq_rel].
    ///
    /// [acquire]: core::sync::atomic::Ordering::Acquire
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
    fn acquire(
        &mut self,
        atomic: &Atomic<Self::Item, Self::Reclaimer, Self::MarkBits>,
        order: Ordering,
    ) -> Marked<Shared<Self::Item, Self::Reclaimer, Self::MarkBits>>;

    /// Atomically takes a snapshot of `atomic` and returns a protected
    /// [`Shared`] reference wrapped in a [`Marked`] to it, if the loaded value
    /// is equal to `expected`.
    ///
    /// A *successfully* loaded value is stored within `self`. If the value of
    /// `atomic` is `null` or a pure tag (marked `null` pointer), no protection
    /// has to be established. After a *successful* load, any previously
    /// protected value will be overwritten and be no longer protected,
    /// regardless of the loaded value. In case of a unsuccessful load, the
    /// previously protected value does not change.
    ///
    /// # Errors
    ///
    /// This method returns an [`Err(NotEqualError)`][NotEqualError] result, if
    /// the atomically loaded snapshot from `atomic` does not match the
    /// `expected` value.
    ///
    /// # Panics
    ///
    /// **May** panic if `order` is [`Acquire`][acquire] or [`AcqRel`][acq_rel].
    ///
    /// [acquire]: core::sync::atomic::Ordering::Acquire
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
    fn acquire_if_equal(
        &mut self,
        atomic: &Atomic<Self::Item, Self::Reclaimer, Self::MarkBits>,
        expected: MarkedPtr<Self::Item, Self::MarkBits>,
        order: Ordering,
    ) -> AcquireResult<Self::Item, Self::Reclaimer, Self::MarkBits>;

    /// Clears the current internal value and its protected state.
    ///
    /// Subsequent calls to [`shared`][Protect::shared] and [`marked`][Protect::marked]
    /// must return [`None`][Option::None] and [`Null`][Marked::Null],
    /// respectively.
    fn release(&mut self);
}

/// Result type for [`acquire_if_equal`][Protect::acquire_if_equal] operations.
pub type AcquireResult<'g, T, R, N> = Result<Marked<Shared<'g, T, R, N>>, NotEqualError>;

/// A zero-size marker type that represents the failure state of an
/// [`acquire_if_equal`](Protect::acquire_if_equal) operation.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct NotEqualError;

impl fmt::Display for NotEqualError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "acquired value does not match `expected`.")
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Record
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A record type that is associated to a specific reclamation scheme.
///
/// Whenever a new [`Owned`] or (non-null) [`Atomic`] is created, a value of
/// this type is allocated on the heap as a wrapper for the desired record.
/// The record and its header are never directly exposed to the data structure
/// using a given memory reclamation scheme and should only be accessed by the
/// reclamation scheme itself.
pub struct Record<T, R: LocalReclaim> {
    /// The record's header
    header: R::RecordHeader,
    /// The record's wrapped (inner) element
    elem: T,
}

impl<T, R: LocalReclaim> Record<T, R> {
    /// Creates a new record with the specified `elem` and a default header.
    #[inline]
    pub fn new(elem: T) -> Self {
        Self { header: Default::default(), elem }
    }

    /// Creates a new record with the specified `elem` and `header`.
    #[inline]
    pub fn with_header(elem: T, header: R::RecordHeader) -> Self {
        Self { header, elem }
    }

    /// Returns a reference to the record's header.
    #[inline]
    pub fn header(&self) -> &R::RecordHeader {
        &self.header
    }

    /// Returns a reference to the record's element.
    #[inline]
    pub fn elem(&self) -> &T {
        &self.elem
    }

    /// Calculates the address of the [`Record`] for the given pointer to a
    /// wrapped non-nullable `elem` and returns the resulting pointer.
    ///
    /// # Safety
    ///
    /// The `elem` pointer must be a valid pointer to an instance of `T` that
    /// was constructed as part of a [`Record`]. Otherwise, the pointer
    /// arithmetic used to determine the address will result in a pointer to
    /// unrelated memory, which is likely to lead to undefined behaviour.
    #[inline]
    pub unsafe fn from_raw_non_null(elem: NonNull<T>) -> NonNull<Self> {
        Self::from_raw(elem.as_ptr())
    }

    /// Calculates the address of the [`Record`] for the given pointer to a
    /// wrapped `elem` and returns the resulting pointer.
    ///
    /// # Safety
    ///
    /// The `elem` pointer must be a valid pointer to an instance of `T` that
    /// was constructed as part of a [`Record`]. Otherwise, the pointer
    /// arithmetic used to determine the address will result in a pointer to
    /// unrelated memory, which is likely to lead to undefined behaviour.
    #[inline]
    pub unsafe fn from_raw(elem: *mut T) -> NonNull<Self> {
        let addr = (elem as usize) - Self::offset_elem();
        NonNull::new_unchecked(addr as *mut _)
    }

    /// Gets a reference to header for the record at the pointed-to location of
    /// the pointer `elem`.
    ///
    /// # Safety
    ///
    /// The pointer `elem` must be a valid pointer to an instance of `T` that
    /// was allocated as part of a `Record`.
    /// Otherwise, the pointer arithmetic used to calculate the header's address
    /// will be incorrect and lead to undefined behavior.
    #[inline]
    pub unsafe fn header_from_raw<'a>(elem: *mut T) -> &'a R::RecordHeader {
        let header = (elem as usize) - Self::offset_elem() + Self::offset_header();
        &*(header as *mut _)
    }

    /// Gets a reference to header for the record at the pointed-to location of
    /// the non-nullable pointer `elem`.
    ///
    /// # Safety
    ///
    /// The pointer `elem` must be a valid pointer to an instance of `T` that
    /// was allocated as part of a `Record`.
    /// Otherwise, the pointer arithmetic used to calculate the header's address
    /// will be incorrect and lead to undefined behavior.
    #[inline]
    pub unsafe fn header_from_raw_non_null<'a>(elem: NonNull<T>) -> &'a R::RecordHeader {
        let header = (elem.as_ptr() as usize) - Self::offset_elem() + Self::offset_header();
        &*(header as *mut _)
    }

    /// Gets the offset in bytes from the address of a record to its header
    /// field.
    #[inline]
    pub fn offset_header() -> usize {
        offset_of!(Self, header)
    }

    /// Gets the offset in bytes from the address of a record to its element
    /// field.
    #[inline]
    pub fn offset_elem() -> usize {
        offset_of!(Self, elem)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Owned
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A pointer type for heap allocated values like `Box`.
///
/// `Owned` values function like marked pointers and are also guaranteed to
/// allocate the appropriate [`RecordHeader`][LocalReclaim::RecordHeader] type
/// for its generic [`LocalReclaim`] parameter alongside its actual content.
#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct Owned<T, R: LocalReclaim, N: Unsigned> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<(T, R)>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Shared
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A shared reference to a value that is actively protected from reclamation by
/// other threads.
///
/// Valid `Shared` values are always borrowed from guard values implementing the
/// [`Protect`] trait.
pub struct Shared<'g, T, R, N> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<(&'g T, R)>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Unlinked
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A reference to a value that has been unlinked and is hence no longer
/// reachable by other threads.
///
/// `Unlinked` values are the result of (successful) atomic *swap* or
/// *compare-and-swap* operations on [`Atomic`](Atomic) values.
/// They are move-only types, but don't have full ownership semantics.
/// Dropping an `Unlinked` value without explicitly retiring it leads to a
/// memory leak.
///
/// Concurrent data structure implementations have to maintain the invariant
/// that no unique value (pointer) can exist more than once within any data
/// structure.
/// Only then is it safe to [`retire`][Reclaim::retire] unlinked values,
/// enabling them to be eventually reclaimed.
/// Note that, while it must be impossible for other threads to create new
/// references to an already unlinked value, it is possible and explicitly
/// allowed for other threads to still have live references that have been
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

/// A non-null pointer value loaded from an [`Atomic`](Atomic), but without any
/// guarantees protecting it from reclamation.
#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct Unprotected<T, R, N> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<R>,
}
