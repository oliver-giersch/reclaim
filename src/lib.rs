//! An abstract & generalized interface supporting various schemes for
//! concurrent memory reclamation.
//!
//! # Memory Management in Rust
//!
//! Unlike garbage collected languages such as *Go* or *Java*, memory
//! management in *Rust* is ultimately manual and more akin to *C++*.
//! Rust's ownership model in combination with the standard library's smart
//! pointer types `Box`, `Rc` and `Arc` make memory management as painless as
//! possible and are able to handle the vast majority of use-cases, while at the
//! same time preventing the classic memory bugs such as *use-after-free*,
//! *double-free* or access to dangling pointers/references.
//! Consequently, there is usually little need for the small additional comfort
//! provided by a fully automated **Garbage Collector** (GC).
//!
//! ## The Need for Automatic Memory Reclamation
//!
//! In the domain of concurrent lock-free data structures, however, the
//! aforementioned memory management schemes are insufficient for determining,
//! when a removed entry can be actually dropped and de-allocated:
//! Just because an entry has been removed (*unlinked*) from some shared data
//! structure does not guarantee, that no other thread could be in the process
//! of reading that same entry at the same time.
//! This is due to the possibility of stale references that were created before
//! the unlinking occurred.
//! The only thing that can be ascertained, due to nature of atomic *swap* and
//! *compare-and-swap* operations, is that other threads can not acquire *new*
//! references after an entry has been unlinked.
//!
//! ## Extending the Grace Period
//!
//! Concurrent memory reclamation schemes work by granting every value
//! (*record*) earmarked for deletion (*retired*) a certain **grace period**
//! before being actually dropped and deallocated, during which it is cached and
//! can still be safely read by other threads with live references to it.
//! How to determine the length of this grace period is up to each individual
//! reclamation scheme.
//!
//! # The Reclaim Interface
//!
//! Due to the restrictions of atomic instructions to machine-word sized chunks
//! of memory, lock-free data structures are necessarily required to work with
//! pointers as well, which is inherently unsafe.
//! Nonetheless, this crate attempts to provide abstractions and an API that
//! encapsulates this unsafety behind safe interface as much as possible.
//! Consequently, the majority of functions and methods exposed are safe to call
//! and can not lead to memory-unsafety.
//!
//! This is primarily achieved by shifting the burden of explicitly maintaining
//! safety invariants to the process of actually reclaiming allocated records.
//! Also construction and insertion into shared variables ([`Atomic`]) is only
//! safely allowed with *valid* (heap allocated) values.
//! All values that are read from shared variables have reference semantics and
//! are non-nullable, although not all types can be safely de-referenced.
//! The [`Option`] and [`Marked`] wrapper types are used to ensure `null`
//! pointer safety.
//! Under these circumstances, memory-unsafety (e.g. use-after-free errors) can
//! only be introduced by incorrectly [`retiring`][LocalReclaim::retire_local]
//! (and hence reclaiming) memory, which is consequently `unsafe`.
//!
//! # Marked Pointer & Reference Types
//!
//! It is a ubiquitous technique in lock-free programming to use the lower bits
//! of a pointer address to store additional information alongside an address.
//! Common use-cases are ABA problem mitigation or to mark a node of a linked
//! list for removal.
//!
//! Accordingly, this crate allows all pointer and reference types
//! ([`MarkedPtr`], [`Shared`], etc.) to be marked.
//! The number of usable mark bits is encoded in the type itself as a generic
//! parameter `N`.
//! However, the number of available mark bits has a physical upper bound, which
//! is dictated by the alignment of the pointed-to type.
//! For instance, a `bool` has an alignment of 1, hence pointers to boolean
//! values can not, in fact, be marked.
//! On a 64-bit system, an `usize` has an alignment of 8, which means a pointer
//! to one can use up to 3 mark bits.
//! Since the number `N` is encoded in the pointer types themselves, attempting
//! to declare types with more available mark bits than what the pointed-to
//! type's alignment will lead to a (currently fairly cryptic) compile time
//! error.
//! Note, that tags are allowed to overflow. This can lead to surprising results
//! when attempting to e.g. mark a pointer that is declared to support zero mark
//! bits (`N = 0`), as the tag will be silently truncated.
//!
//! # Terminology
//!
//! Throughout this crate's API and its documentation a certain terminology is
//! consistently used, which is summarized below:
//!
//! - record
//!   
//!   A heap allocated value which is managed by some reclamation scheme.
//!
//! - unlink
//!   
//!   The act removing the pointer to a *record* from shared memory through an
//!   atomic operation such as *compare-and-swap*.
//!
//! - retire
//!
//!   The act of marking an *unlinked* record as no longer in use and handing
//!   off the responsibility for de-allocation to the reclamation scheme.
//!
//! - reclaim
//!
//!   The act of dropping and de-allocating a *retired* record.
//!   The reclamation scheme is responsible for guaranteeing that *retired*
//!   records are kept alive (cached) **at least** until their respective *grace
//!   periods* have expired.

#![cfg_attr(not(any(test, feature = "std")), no_std)]
#![warn(missing_docs)]

#[cfg(not(feature = "std"))]
extern crate alloc;

use core::fmt;
use core::marker::PhantomData;
use core::ptr::NonNull;
use core::sync::atomic::Ordering;

#[cfg(feature = "std")]
use std::error::Error;

// TODO: replace with const generics once available
pub use typenum;

use memoffset::offset_of;
use typenum::Unsigned;

#[macro_use]
mod macros;

pub mod align;
pub mod leak;
pub mod prelude {
    //! Useful and/or required types, discriminants and traits for the `reclaim`
    //! crate.

    pub use crate::pointer::{
        Marked::{self, Null, Value},
        MarkedPointer, NonNullable,
    };

    pub use crate::LocalReclaim;
    pub use crate::Protect;
    pub use crate::Reclaim;
}

mod atomic;
mod owned;
mod pointer;
mod retired;
mod shared;
mod unlinked;
mod unprotected;

pub use crate::atomic::{Atomic, CompareExchangeFailure};
pub use crate::pointer::{
    AtomicMarkedPtr, InvalidNullError, Marked, MarkedNonNull, MarkedPointer, MarkedPtr, NonNullable,
};
pub use crate::retired::Retired;

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

/// A trait, which constitutes the foundation for the [`Reclaim`] trait.
///
/// This trait is specifically intended to be fully compatible with `#[no_std]`
/// environments.
/// This is expressed by the requirement to explicitly pass references to thread
/// local state or storage when calling functions that retire records.
///
/// If a reclamation scheme does not require or deliberately chooses to avoid
/// using thread local storage for the sake of simplicity or portability
/// (usually at the cost of performance), it is valid to implement `Self::Local`
/// as `()` and pass all retired records directly through to some global state.
/// Note, that this will generally require more and also more frequent
/// synchronization.
/// For cases in which `Local` is defined to be `()`, there exists a blanket
/// implementation of [`Reclaim].
pub unsafe trait LocalReclaim
where
    Self: Sized,
{
    // TODO: type Allocator: Alloc + Default???;
    // TODO: type Guarded<T, const N: usize>: Protect<Item = T, MARK_BITS = N>;

    /// The type used for storing all relevant thread local state.
    type Local: Sized;

    /// Every record allocates this type alongside itself to store additional
    /// reclamation scheme specific data.
    /// When no such data is required, `()` is the recommended choice.
    type RecordHeader: Default + Sync + Sized;

    /// Retires a record and caches it **at least** until it is safe to
    /// deallocate it.
    ///
    /// How to determine that no other thread can possibly have any (protected)
    /// reference to a record depends on the respective reclamation scheme.
    ///
    /// # Safety
    ///
    /// The caller has to guarantee that the record is **fully** unlinked from
    /// any data structure it was previously inserted in:
    /// There must be no way for another thread to acquire a *new* reference to
    /// the given `unlinked` record.
    ///
    /// While an [`Unlinked`] value can only safely be obtained by atomic
    /// operations that do in fact remove a value from its place in memory (i.e.
    /// *swap* or *compare-and-swap*), this is only the *necessary* condition
    /// for safe reclamation, but not always *sufficient*.
    /// When a unique address to heap allocated memory is inserted in more than
    /// one element of a shared data structure, it is still possible for other
    /// threads to access this address even if its unlinked from one spot.
    ///
    /// This invariant also mandates, that correct synchronization of atomic
    /// operations around calls to functions that retire records is ensured.
    /// Consider the following (incorrect) example:
    ///
    /// ```ignore
    /// # use core::sync::atomic::Ordering::{Relaxed};
    /// # use reclaim::Unlinked;
    ///
    /// let g = Atomic::from(Owned::new(1));
    ///
    /// // thread 1
    /// let expected = g.load_unprotected(Relaxed); // reads &1
    /// let unlinked = g
    ///     .compare_exchange(expected, Owned::null(), Relaxed, Relaxed)
    ///     .unwrap();
    ///
    /// unsafe { Unlinked::retire(unlinked) };
    ///
    /// // thread 2
    /// if let Some(shared) = g.load(Relaxed, &mut guard) {
    ///     assert_eq!(*shared, &1); // !!! may read freed memory
    /// }
    /// ```
    ///
    /// In this example, the invariant can not be guaranteed to be maintained,
    /// due to the incorrect (relaxed) memory orderings.
    /// Thread 1 can potentially unlink the shared value, retire and reclaim it,
    /// without the `compare_exchange` operation ever becoming visible to
    /// thread 2.
    /// The thread could then proceed to load and read the previous
    /// value instead of the inserted `null`, accessing freed memory.
    unsafe fn retire_local<T: 'static, N: Unsigned>(
        local: &Self::Local,
        unlinked: Unlinked<T, Self, N>,
    );

    /// Retires a record and caches it **at least** until it is safe to
    /// deallocate it.
    ///
    /// How to determine that no other thread can possibly have any (protected)
    /// reference to a record depends on the respective reclamation scheme.
    ///
    /// # Safety
    ///
    /// The same restrictions as with the [`retire_local`][LocalReclaim::retire_local]
    /// function apply here as well.
    ///
    /// In addition to these invariants, this method additionally requires the
    /// caller to ensure any `Drop` implementation for `T` or any contained type
    /// does not access any **non-static** references.
    /// The `reclaim` interface makes no guarantees about the precise time a
    /// retired record is actually reclaimed.
    /// Hence, it is not possible to ensure any references stored within the
    /// record have not become invalid at that point.
    unsafe fn retire_local_unchecked<T, N: Unsigned>(
        local: &Self::Local,
        unlinked: Unlinked<T, Self, N>,
    );
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Protect (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait applicable for pointer types that *protect* their pointed-to value
/// from reclamation during the lifetime of the protecting *guard*.
pub unsafe trait Protect
where
    Self: Sized + Clone,
{
    /// The type of the value protected from reclamation
    type Item: Sized;
    /// The reclamation scheme associated with this type of guard
    type Reclaimer: LocalReclaim;
    /// The Number of bits available for storing additional information
    type MarkBits: Unsigned;

    /// Returns an optional [`Shared`] reference to the protected value, which
    /// is tied to the lifetime of `&self`.
    fn shared(&self) -> Option<Shared<Self::Item, Self::Reclaimer, Self::MarkBits>> {
        self.marked().value()
    }

    /// Returns a  [`Shared`] reference wrapped in a [`Marked`] for the
    /// protected value, which is tied to the lifetime of `&self`.
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
    /// *May* panic if `order` is [`Release`][release] or [`AcqRel`][acq_rel].
    ///
    /// [release]: core::sync::atomic::Ordering::Release
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
    fn acquire(
        &mut self,
        atomic: &Atomic<Self::Item, Self::Reclaimer, Self::MarkBits>,
        order: Ordering,
    ) -> Marked<Shared<Self::Item, Self::Reclaimer, Self::MarkBits>>;

    /// Atomically takes a snapshot of `atomic` and returns a protected
    /// [`Shared`] reference wrapped in a [`Marked`] to it, **if** the loaded
    /// value is equal to `expected`.
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
    /// *May* panic if `order` is [`Release`][release] or [`AcqRel`][acq_rel].
    ///
    /// [release]: core::sync::atomic::Ordering::Release
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
    /// must return [`None`] and [`Null`][Marked::Null], respectively.
    fn release(&mut self);
}

/// Result type for [`acquire_if_equal`][Protect::acquire_if_equal] operations.
pub type AcquireResult<'g, T, R, N> = Result<Marked<Shared<'g, T, R, N>>, NotEqualError>;

/// A zero-size marker type that represents the failure state of an
/// [`acquire_if_equal`][Protect::acquire_if_equal] operation.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct NotEqualError;

impl fmt::Display for NotEqualError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "acquired value does not match `expected`.")
    }
}

#[cfg(feature = "std")]
impl Error for NotEqualError {}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Record
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A record type that is associated with a specific reclamation scheme.
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

    /// Returns a reference to the header for the record at the pointed-to
    /// location of the pointer `elem`.
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

    /// Returns a reference to the header for the record at the pointed-to
    /// location of the non-nullable pointer `elem`.
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

    /// Returns the offset in bytes from the address of a record to its header
    /// field.
    #[inline]
    pub fn offset_header() -> usize {
        offset_of!(Self, header)
    }

    /// Returns the offset in bytes from the address of a record to its element
    /// field.
    #[inline]
    pub fn offset_elem() -> usize {
        offset_of!(Self, elem)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Owned
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A pointer type for heap allocated values similar to `Box`.
///
/// `Owned` values function like marked pointers and are also guaranteed to
/// allocate the appropriate [`RecordHeader`][LocalReclaim::RecordHeader] type
/// for its generic [`LocalReclaim`] parameter alongside their actual content.
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
/// `Shared` values have similar semantics to shared references (`&'g T`), i.e.
/// they can be trivially copied, cloned and (safely) de-referenced.
/// However, they do retain potential mark bits of the atomic value from which
/// they were originally read.
/// They are also usually borrowed from guard values implementing the
/// [`Protect`] trait.
pub struct Shared<'g, T, R, N> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<(&'g T, R)>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Unlinked
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A reference to a value that has been removed from its previous location in
/// memory and is hence no longer reachable by other threads.
///
/// `Unlinked` values are the result of (successful) atomic *swap* or
/// *compare-and-swap* operations on [`Atomic`] values.
/// They are move-only types, but they don't have full ownership semantics,
/// either.
/// Dropping an `Unlinked` value without explicitly retiring it almost certainly
/// results in a memory leak.
///
/// The safety invariants around retiring `Unlinked` references are explained
/// in detail in the documentation for [`retire_local`][LocalReclaim::retire_local].
#[derive(Eq, Ord, PartialEq, PartialOrd)]
#[must_use = "unlinked values are meant to be retired, otherwise a memory leak is highly likely"]
pub struct Unlinked<T, R, N> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<(T, R)>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Unprotected
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A reference to a value loaded from an [`Atomic`] that is not actively
/// protected from reclamation.
///
/// `Unprotected` values can not be safely de-referenced under usual
/// circumstances (i.e. other threads can retire and reclaim unlinked records).
/// They do, however, have stronger guarantees than raw (marked) pointers:
/// Since are loaded from [`Atomic`] values they must (at least at one point)
/// have been *valid* references.
#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct Unprotected<T, R, N> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<R>,
}
