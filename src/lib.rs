//! A generic trait based interface for abstracting over various schemes for
//! concurrent memory reclamation.
//!
//! # Memory Management in Rust
//!
//! Unlike garbage collected languages such as *Go* or *Java*, memory
//! management in *Rust* is primarily scope or ownership based and more akin to
//! *C++*.
//! Rust's ownership model in combination with the standard library's smart
//! pointer types `Box`, `Rc` and `Arc` make memory management as painless as
//! possible and are able to handle the vast majority of use-cases, while at the
//! same time preventing the classic memory bugs such as *use-after-free*,
//! *double-free* or memory leaks.
//! Consequently, there is usually little need for the relatively small
//! additional comfort provided by a fully automated **Garbage Collector** (GC).
//!
//! ## The Need for Automatic Memory Reclamation
//!
//! In the domain of concurrent lock-free data structures, however, the
//! aforementioned memory management schemes are insufficient for determining,
//! when a removed entry can be actually dropped and de-allocated:
//! Just because an entry has been removed (*unlinked*) from some shared data
//! structure does not guarantee, that no other thread could still be in the
//! process of reading that same entry at the same time.
//! This is due to the possible existence of stale references that were created
//! by other threads before the unlinking occurred.
//! The only fact that can be ascertained, due to nature of atomic *swap* and
//! *compare-and-swap* operations, is that other threads can not acquire *new*
//! references after an entry has been unlinked.
//!
//! ## Extending the Grace Period
//!
//! Concurrent memory reclamation schemes work by granting every value
//! (*record*) earmarked for deletion (*retired*) a certain **grace period**
//! before being actually dropped and de-allocated.
//! During this period the value will be cached and can still be safely read by
//! other threads with live references to it, but no new references must be
//! possible.
//! Determining the exact length of this grace period is up to each individual
//! reclamation scheme.
//! It is usually either not possible or not practical to determine the exact
//! moment at which it becomes safe to reclaim a retired.
//! Hence, reclamation schemes commonly tend to only guarantee grace periods
//! that are *at least* as long as to ensure no references can possibly exist
//! afterwards.
//!
//! # The Reclaim Interface
//!
//! Lock-free data structures are usually required to work on atomic pointers to
//! heap allocated memory.
//! This is due to the restrictions of atomic CPU instructions to machine-word
//! sized values, such as pointers.
//! Working with raw pointers is inherently *unsafe* in the Rust sense.
//! Consequently, this crate avoids and discourages the use of raw pointers as
//! much as possible in favor of safer abstractions with stricter constraints
//! for their usage.
//! In effect, the vast majority of this crate's public API is safe to use under
//! any circumstances.
//! This, however, is achieved by shifting and concentrating the burden of
//! manually maintaining safety invariants into one specific key aspect:
//! The retiring (and eventual reclamation) of records.
//!
//! ## Traits and Types
//!
//! The `reclaim` crate primarily exposes four different traits, which are
//! relevant for users of generic code and implementors of reclamation schemes
//! alike.
//! The first trait is [`Reclaim`], which provides generic functionality for
//! retiring records.
//! Note, that this trait does not presume the presence of an operating system
//! and functionality like thread local storage.
//! Hence, this trait can even be used in `#[no_std]` environments.
//! However, in order to use this trait's associated methods, an explicit
//! reference to the current thread's (local) state is required.
//! For environments with implicit access to thread local storage, the
//! [`GlobalReclaim`] trait exists as an extension to [`Reclaim`].
//! This trait additionally requires an associated type
//! [`Guard`][GlobalReclaim::Guard], which must implement both the [`Default`]
//! and the [`Protect`] trait.
//!
//! Types implementing the [`Protect`] trait can be used to safely read values
//! from shared memory that are subsequently safe from concurrent reclamation
//! and can hence be safely de-referenced.
//! Note that a single guard can only protect one value at a time.
//! This follows the design of many reclamation schemes, such as *hazard
//! pointers*.
//! This is represented by the requirement to pass a *mutable* reference to a
//! guard in order to safely load a shared value.
//!
//! Some reclamation schemes (e.g. epoch based ones) do not require individual
//! protection of values, but instead protect arbitrary numbers of shared at
//! once.
//! The guard types for these schemes can additionally implement the
//! [`ProtectRegion`] trait.
//! Such guards do not have to carry any state and protect values simply by
//! their existence.
//! Consequently, it is also possible to call eg [`Atomic::load`] with a shared
//! reference to a guard implementing that trait.
//!
//! ## The `Atomic` Type
//!
//! The [`Atomic`] markable concurrent pointer type is the main point of
//! interaction with this crate.
//! It can only be safely created as a `null` pointer or with valid heap
//! allocation backing it up.
//! It supports all common atomic operations like `load`, `store`,
//! `compare_exchange`, etc.
//! The key aspect of this type is that together with a guard, shared values
//! can be safely loaded and de-referenced while other threads can concurrently
//! reclaim removed values.
//! In addition to the [`Shared`] type, which represents a shared reference that
//! is protected from reclamation, other atomic operations can yield
//! [`Unprotected`] or [`Unlinked`] values.
//! The former are explicitly not protected from reclamation and can be loaded
//! without any guards.
//! They are not safe to de-reference, but can be used to e.g. swing a pointer
//! from one linked list node to another.
//! [`Unlinked`] values are the result of *swap* or *compare-and-swap*
//! operations and represent values/references to which no new references can be
//! acquired any more by other threads.
//! They are like *owned* values that are also borrowed, since other threads may
//! still reference them.
//! All of these three different types are guaranteed to never be null at the
//! type level.
//!
//! ## Marked Pointer & Reference Types
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

    pub use crate::GlobalReclaim;
    pub use crate::Protect;
    pub use crate::ProtectRegion;
    pub use crate::Reclaim;
}

mod atomic;
mod internal;
mod owned;
mod pointer;
mod retired;
mod shared;
mod unlinked;
mod unprotected;

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

pub use crate::atomic::{Atomic, CompareExchangeFailure};
pub use crate::pointer::{
    AtomicMarkedPtr, InvalidNullError, Marked, MarkedNonNull, MarkedPointer, MarkedPtr, NonNullable,
};
pub use crate::retired::Retired;

////////////////////////////////////////////////////////////////////////////////////////////////////
// GlobalReclaim (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait for retiring and reclaiming entries removed from concurrent
/// collections and data structures.
///
/// Implementing this trait requires first implementing the [`Reclaim`]
/// trait for the same type and is usually only possible in `std` environments
/// with access to thread local storage.
///
/// # Examples
///
/// Defining a concurrent data structure generic over the employed reclamation
/// scheme:
///
/// ```
/// use reclaim::typenum::U0;
/// use reclaim::GlobalReclaim;
///
/// type Atomic<T, R> = reclaim::Atomic<T, R, U0>;
///
/// pub struct Stack<T, R: GlobalReclaim> {
///     head: Atomic<Node<T, R>, R>,
/// }
///
/// struct Node<T, R: GlobalReclaim> {
///     elem: T,
///     next: Atomic<Node<T, R>, R>,
/// }
/// ```
pub unsafe trait GlobalReclaim
where
    Self: Reclaim,
{
    /// The type used for protecting concurrently shared references.
    type Guard: Protect<Reclaimer = Self> + Default;

    /// Creates a new [`Guard`][GlobalReclaim::Guard].
    ///
    /// When `Self::Guard` implements [`ProtectRegion`], this operation
    /// instantly establishes protection for loaded values.
    /// Otherwise, the guard must first explicitly protect a specific shared
    /// value.
    fn guard() -> Self::Guard {
        Self::Guard::default()
    }

    /// Retires a record and caches it **at least** until it is safe to
    /// deallocate it.
    ///
    /// For further information, refer to the documentation of
    /// [`retire_local`][`Reclaim::retire_local`].
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
    /// [`retire_local_unchecked`][`Reclaim::retire_local_unchecked`].
    ///
    /// # Safety
    ///
    /// The same caveats as with [`retire_local`][`Reclaim::retire_local`]
    /// apply.
    unsafe fn retire_unchecked<T, N: Unsigned>(unlinked: Unlinked<T, Self, N>);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Reclaim (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait, which constitutes the foundation for the [`GlobalReclaim`] trait.
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
/// implementation of [`GlobalReclaim].
pub unsafe trait Reclaim
where
    Self: Sized + 'static,
{
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
    /// unsafe { unlinked.retire() };
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
    /// The same restrictions as with the [`retire_local`][Reclaim::retire_local]
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

/// A trait for guard types that *protect* a specific value from reclamation
/// during the lifetime of the protecting guard.
pub unsafe trait Protect
where
    Self: Clone + Sized,
{
    /// The reclamation scheme associated with this type of guard
    type Reclaimer: Reclaim;

    /// Converts the guard into a [`Guarded`] by fusing it with a value loaded
    /// from `atomic`.
    ///
    /// # Errors
    ///
    /// If the value loaded from `atomic` is `null`, this method instead `self`
    /// again, wrapped in an [`Err`].
    #[inline]
    fn try_fuse<T, N: Unsigned>(
        mut self,
        atomic: &Atomic<T, Self::Reclaimer, N>,
        order: Ordering,
    ) -> Result<Guarded<T, Self, N>, Self> {
        if let Marked::Value(shared) = self.protect(atomic, order) {
            let ptr = Shared::into_marked_non_null(shared);
            Ok(Guarded { guard: self, ptr })
        } else {
            Err(self)
        }
    }

    /// Releases any current protection that may be provided by the guard.
    ///
    /// By borrowing `self` mutably it is ensured that no loaded values
    /// protected by this guard can be used after calling this method.
    /// If `Self` additionally implements [`ProtectRegion`], this is a no-op
    fn release(&mut self);

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
    fn protect<T, N: Unsigned>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaimer, N>,
        order: Ordering,
    ) -> Marked<Shared<T, Self::Reclaimer, N>>;

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
    fn protect_if_equal<T, N: Unsigned>(
        &mut self,
        atomic: &Atomic<T, Self::Reclaimer, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> AcquireResult<T, Self::Reclaimer, N>;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// ProtectRegion (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A trait for guard types that protect any values loaded during their
/// existence and lifetime.
pub unsafe trait ProtectRegion
where
    Self: Protect,
{
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// AcquireResult
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Result type for [`acquire_if_equal`][Protect::acquire_if_equal] operations.
pub type AcquireResult<'g, T, R, N> = Result<Marked<Shared<'g, T, R, N>>, NotEqualError>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// NotEqualError
////////////////////////////////////////////////////////////////////////////////////////////////////

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
pub struct Record<T, R: Reclaim> {
    /// The record's header
    header: R::RecordHeader,
    /// The record's wrapped (inner) element
    elem: T,
}

impl<T, R: Reclaim> Record<T, R> {
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
// Guarded
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A guard type fused with a protected value.
#[derive(Debug)]
pub struct Guarded<T, G, N: Unsigned> {
    guard: G,
    ptr: MarkedNonNull<T, N>,
}

impl<T, G: Protect, N: Unsigned> Guarded<T, G, N> {
    /// Returns a [`Shared`] reference borrowed from the [`Guarded`].
    #[inline]
    pub fn shared(&self) -> Shared<T, G::Reclaimer, N> {
        Shared { inner: self.ptr, _marker: PhantomData }
    }

    /// Converts the [`Guarded`] into the internally stored guard.
    ///
    /// If `G` does not implement [`ProtectRegion`], the returned guard is
    /// guaranteed to be [`released`][Protect::release] before being returned.
    #[inline]
    pub fn into_guard(self) -> G {
        let mut guard = self.guard;
        guard.release();
        guard
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Owned
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A pointer type for heap allocated values similar to `Box`.
///
/// `Owned` values function like marked pointers and are also guaranteed to
/// allocate the appropriate [`RecordHeader`][Reclaim::RecordHeader] type
/// for its generic [`Reclaim`] parameter alongside their actual content.
#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct Owned<T, R: Reclaim, N: Unsigned> {
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
/// in detail in the documentation for [`retire_local`][Reclaim::retire_local].
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
