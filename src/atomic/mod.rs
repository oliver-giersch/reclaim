mod compare;
mod guard;
mod store;

use core::fmt;
use core::marker::PhantomData;
use core::sync::atomic::Ordering;

use typenum::Unsigned;

use crate::internal::{Compare, Guard, Internal, Store};
use crate::leak::Leaking;
use crate::pointer::{AtomicMarkedPtr, Marked, MarkedNonNull, MarkedPointer, MarkedPtr};
use crate::{AcquireResult, NotEqualError, Owned, Reclaim, Shared, Unlinked, Unprotected};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Atomic
////////////////////////////////////////////////////////////////////////////////////////////////////

/// An atomic markable pointer type to an owned heap allocated value similar to
/// [`AtomicPtr`](core::sync::atomic::AtomicPtr).
///
/// The `Atomic` type has similarities to [`Option<Box>`][Option], as it is a
/// pointer that is either `null` or otherwise must point to a valid, heap
/// allocated value.
/// Note, that the type does not implement the [`Drop`](core::ops::Drop) trait,
/// meaning it does not automatically take care of memory de-allocation when it
/// goes out of scope.
/// Use the [`take`][Atomic::take] method to extract an (optional) [`Owned`]
/// value, which *does* correctly deallocate memory when it goes out of scope.
pub struct Atomic<T, R, N> {
    inner: AtomicMarkedPtr<T, N>,
    _marker: PhantomData<(T, R)>,
}

unsafe impl<T, R: Reclaim, N: Unsigned> Send for Atomic<T, R, N> where T: Send + Sync {}
unsafe impl<T, R: Reclaim, N: Unsigned> Sync for Atomic<T, R, N> where T: Send + Sync {}

impl<T, R, N> Atomic<T, R, N> {
    /// Creates a new `null` pointer.
    #[inline]
    pub const fn null() -> Self {
        Self { inner: AtomicMarkedPtr::null(), _marker: PhantomData }
    }

    /// Gets a reference to the underlying (raw) atomic markable pointer.
    #[inline]
    pub const fn as_raw(&self) -> &AtomicMarkedPtr<T, N> {
        &self.inner
    }
}

impl<T, R: Reclaim, N: Unsigned> Atomic<T, R, N> {
    /// Allocates a new [`Owned`] containing the given `val` and immediately
    /// storing it an `Atomic`.
    #[inline]
    pub fn new(val: T) -> Self {
        Self::from(Owned::from(val))
    }

    /// Creates a new [`Atomic`] from the given `ptr`.
    ///
    /// # Safety
    ///
    /// The given `ptr` argument must be a pointer to a valid heap allocated
    /// instance of `T` that was allocated as part of a [`Record`][crate::Record],
    /// e.g. through an [`Owned`].
    /// The same pointer should also not be used to create more than one
    /// [`Atomic`]s.
    #[inline]
    pub unsafe fn from_raw(ptr: MarkedPtr<T, N>) -> Self {
        Self { inner: AtomicMarkedPtr::new(ptr), _marker: PhantomData }
    }

    /// Loads a raw marked value from the pointer.
    ///
    /// `load_raw` takes an [`Ordering`][ordering] argument, which describes the
    /// memory ordering of this operation.
    ///
    /// # Panics
    ///
    /// Panics if `order` is [`Release`][release] or [`AcqRel`][acq_rel].
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [release]: core::sync::atomic::Ordering::Release
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
    ///
    /// # Example
    ///
    /// Commonly, this is likely going to be used in conjunction with
    /// [`load_if_equal`][Atomic::load_if_equal] or
    /// [`acquire_if_equal`][Protect::acquire_if_equal].
    ///
    /// ```
    /// use std::sync::atomic::Ordering::Relaxed;
    ///
    /// use reclaim::typenum::U0;
    ///
    /// type Atomic<T> = reclaim::leak::Atomic<T, U0>;
    /// type Guarded<T> = reclaim::leak::LeakingGuard<T, U0>;
    ///
    /// let atomic = Atomic::new("string");
    /// let mut guarded = Guarded::default();
    ///
    /// let ptr = atomic.load_raw(Relaxed);
    /// let res = atomic.load_if_equal(ptr, Relaxed, &mut guarded);
    ///
    /// assert!(res.is_ok());
    /// # assert_eq!(&"string", &*res.unwrap().unwrap());
    /// ```
    #[inline]
    pub fn load_raw(&self, order: Ordering) -> MarkedPtr<T, N> {
        self.inner.load(order)
    }

    /// Loads an optional [`Unprotected`] reference from the `Atomic`.
    ///
    /// The returned reference is explicitly **not** protected from reclamation,
    /// meaning another thread could free the value's memory at any time.
    ///
    /// This method is similar to [`load_raw`][Atomic::load_raw], but the
    /// resulting [`Unprotected`] type has stronger guarantees than a raw
    /// [`MarkedPtr`].
    /// It can be useful to load an unprotected pointer if that pointer does not
    /// need to be de-referenced, but is only used to reinsert it in a different
    /// spot, which is e.g. done when removing a value from a linked list.
    ///
    /// # Panics
    ///
    /// Panics if `order` is [`Release`][release] or [`AcqRel`][acq_rel].
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [release]: core::sync::atomic::Ordering::Release
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
    #[inline]
    pub fn load_unprotected(&self, order: Ordering) -> Option<Unprotected<T, R, N>> {
        self.load_marked_unprotected(order).value()
    }

    /// Loads an [`Unprotected`] reference wrapped in a [`Marked`] from the
    /// `Atomic`.
    ///
    /// The returned reference is explicitly **not** protected from reclamation,
    /// meaning another thread could free the value's memory at any time.
    ///
    /// This method is similar to [`load_raw`][Atomic::load_raw], but the
    /// resulting [`Unprotected`] type has stronger guarantees than a raw
    /// [`MarkedPtr`].
    /// It can be useful to load an unprotected pointer if that pointer does not
    /// need to be de-referenced, but is only used to reinsert it in a different
    /// spot, which is e.g. done when removing a value from a linked list.
    ///
    /// # Panics
    ///
    /// Panics if `order` is [`Release`][release] or [`AcqRel`][acq_rel].
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [release]: core::sync::atomic::Ordering::Release
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
    #[inline]
    pub fn load_marked_unprotected(&self, order: Ordering) -> Marked<Unprotected<T, R, N>> {
        MarkedNonNull::new(self.inner.load(order))
            .map(|ptr| Unprotected { inner: ptr, _marker: PhantomData })
    }

    /// Loads a value from the pointer and uses `guard` to protect it.
    ///
    /// If the loaded value is non-null, the value is guaranteed to be protected
    /// from reclamation during the lifetime of `guard`.
    ///
    /// `load` takes an [`Ordering`][ordering] argument, which describes the
    /// memory ordering of this operation.
    ///
    /// # Panics
    ///
    /// *May* panic if `order` is [`Release`][release] or [`AcqRel`][acq_rel].
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [release]: core::sync::atomic::Ordering::Release
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
    #[inline]
    pub fn load<'g>(
        &self,
        order: Ordering,
        guard: impl Guard<'g, Reclaimer = R>,
    ) -> Option<Shared<'g, T, R, N>> {
        guard.load_protected(self, order).value()
    }

    /// Loads a value from the pointer and uses `guard` to protect it, but only
    /// if the loaded value equals `expected`.
    ///
    /// If the loaded value is non-null, the value is guaranteed to be protected
    /// from reclamation during the lifetime of `guard`.
    ///
    /// `load_if_equal` takes an [`Ordering`][ordering] argument, which
    /// describes the memory ordering of this operation.
    ///
    /// # Panics
    ///
    /// *May* panic if `order` is [`Release`][release] or [`AcqRel`][acq_rel].
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [release]: core::sync::atomic::Ordering::Release
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
    #[inline]
    pub fn load_if_equal<'g>(
        &self,
        expected: MarkedPtr<T, N>,
        order: Ordering,
        guard: impl Guard<'g, Reclaimer = R>,
    ) -> Result<Option<Shared<'g, T, R, N>>, NotEqualError> {
        guard.load_protected_if_equal(self, expected, order).map(Marked::value)
    }

    /// Loads a value from the pointer and uses `guard` to protect it.
    /// The (optional) protected [`Shared`] value is wrapped in a [`Marked].
    ///
    /// If the loaded value is non-null, the value is guaranteed to be protected
    /// from reclamation during the lifetime of `guard`.
    ///
    /// The primary difference to [`load`][Atomic::load] is, that the returned
    /// [`Marked`] type is additionally able to represent marked `null`
    /// pointers.
    ///
    /// `load_marked` takes an [`Ordering`][ordering] argument, which describes
    /// the memory ordering of this operation.
    ///
    /// # Panics
    ///
    /// *May* panic if `order` is [`Release`][release] or [`AcqRel`][acq_rel].
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [release]: core::sync::atomic::Ordering::Release
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
    #[inline]
    pub fn load_marked<'g>(
        &self,
        order: Ordering,
        guard: impl Guard<'g, Reclaimer = R>,
    ) -> Marked<Shared<'g, T, R, N>> {
        guard.load_protected(self, order)
    }

    /// Loads a value from the pointer and uses `guard` to protect it, but only
    /// if the loaded value equals `expected`.
    /// The (optional) protected [`Shared`] value is wrapped in a [`Marked].
    ///
    /// If the loaded value is non-null, the value is guaranteed to be protected
    /// from reclamation during the lifetime of `guard`.
    ///
    /// The primary difference to [`load_if_equal`][Atomic::load_if_equal] is,
    /// that the returned [`Marked`] type is additionally able to represent
    /// marked `null` pointers.
    ///
    /// `load_marked_if_equal` takes an [`Ordering`][ordering] argument, which
    /// describes the memory ordering of this operation.
    ///
    /// # Panics
    ///
    /// *May* panic if `order` is [`Release`][release] or [`AcqRel`][acq_rel].
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [release]: core::sync::atomic::Ordering::Release
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
    #[inline]
    pub fn load_marked_if_equal<'g>(
        &self,
        expected: MarkedPtr<T, N>,
        order: Ordering,
        guard: impl Guard<'g, Reclaimer = R>,
    ) -> AcquireResult<'g, T, R, N> {
        guard.load_protected_if_equal(self, expected, order)
    }

    /// Stores either `null` or a valid pointer to an owned heap allocated value
    /// into the pointer.
    ///
    /// Note, that overwriting a non-null value through `store` will very likely
    /// lead to memory leaks, since instances of [`Atomic`] will most commonly
    /// be associated wit some kind of uniqueness invariants in order to be sound.
    ///
    /// `store` takes an [`Ordering`][ordering] argument, which
    /// describes the memory ordering of this operation.
    ///
    /// # Panics
    ///
    /// Panics if `order` is [`Acquire`][acquire] or [`AcqRel`][acq_rel]
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [acquire]: core::sync::atomic::Ordering::Acquire
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
    #[inline]
    pub fn store(&self, ptr: impl Store<Item = T, MarkBits = N, Reclaimer = R>, order: Ordering) {
        self.inner.store(MarkedPointer::into_marked_ptr(ptr), order);
    }

    /// Stores either `null` or a valid pointer to an owned heap allocated value
    /// into the pointer, returning the previous value.
    ///
    /// The returned value can be safely reclaimed as long as the *uniqueness*
    /// invariant is maintained.
    ///
    /// `swap` takes an [`Ordering`][ordering] argument which describes the memory
    /// ordering of this operation. All ordering modes are possible. Note that using
    /// [`Acquire`][acquire] makes the store part of this operation [`Relaxed`][relaxed],
    /// and using [`Release`][release] makes the load part [`Relaxed`][relaxed].
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [relaxed]: core::sync::atomic::Ordering::Relaxed
    /// [acquire]: core::sync::atomic::Ordering::Acquire
    /// [release]: core::sync::atomic::Ordering::Release
    #[inline]
    pub fn swap(
        &self,
        ptr: impl Store<Item = T, Reclaimer = R, MarkBits = N>,
        order: Ordering,
    ) -> Option<Unlinked<T, R, N>> {
        let res = self.inner.swap(MarkedPointer::into_marked_ptr(ptr), order);
        // this is safe because the pointer is no longer accessible by other threads
        // (there can still be outstanding references that were loaded before the swap)
        unsafe { Option::from_marked_ptr(res) }
    }

    /// Stores a value (either null or valid) into the pointer if the current
    /// value is the same as `current`.
    ///
    /// The return value is a result indicating whether the `new` value was
    /// written and containing the previous and now unlinked value.
    /// On success this value is guaranteed to be equal to `current` and can be
    /// safely reclaimed as long as the *uniqueness* invariant is maintained.
    /// On failure, a [struct](CompareExchangeFailure) is returned that contains
    /// both the actual value and the value that was previously attempted to be
    /// inserted (`new`).
    /// This is necessary, because it is possible to attempt insertion of
    /// move-only types such as [`Owned`] or [`Unlinked`], which would otherwise
    /// be irretrievably lost when the `compare_exchange` fails.
    /// The actually loaded value is [`Unprotected`].
    ///
    /// `compare_exchange` takes two [`Ordering`][ordering] arguments to
    /// describe the memory ordering of this operation.
    /// The first describes the required ordering if the operation succeeds
    /// while the second describes the required ordering when the operation
    /// fails.
    /// Using [`Acquire`][acquire] as success ordering makes the store part of
    /// this operation [`Relaxed`][relaxed], and using [`Release`][release]
    /// makes the successful load [`Relaxed`][relaxed].
    /// The failure ordering can only be [`SeqCst`][seq_cst],
    /// [`Acquire`][acquire] or [`Relaxed`][relaxed] and must be equivalent to
    /// or weaker than the success ordering.
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [relaxed]: core::sync::atomic::Ordering::Relaxed
    /// [acquire]: core::sync::atomic::Ordering::Acquire
    /// [release]: core::sync::atomic::Ordering::Release
    /// [seq_cst]: core::sync::atomic::Ordering::SeqCst
    #[inline]
    pub fn compare_exchange<C, S>(
        &self,
        current: C,
        new: S,
        success: Ordering,
        failure: Ordering,
    ) -> Result<C::Unlinked, CompareExchangeFailure<T, R, S, N>>
    where
        C: Compare<Item = T, MarkBits = N, Reclaimer = R>,
        S: Store<Item = T, MarkBits = N, Reclaimer = R>,
    {
        let current = MarkedPointer::into_marked_ptr(current);
        let new = MarkedPointer::into_marked_ptr(new);

        self.inner
            .compare_exchange(current, new, success, failure)
            .map(|ptr| unsafe { C::Unlinked::from_marked_ptr(ptr) })
            .map_err(|ptr| CompareExchangeFailure {
                loaded: unsafe { Option::from_marked_ptr(ptr) },
                input: unsafe { S::from_marked_ptr(new) },
                _marker: PhantomData,
            })
    }

    /// Stores a value (either null or valid) into the pointer if the current
    /// value is the same as `current`.
    ///
    /// Unlike [`compare_exchange`](Atomic::compare_exchange), this function is
    /// allowed to spuriously fail even when the comparision succeeds, which can
    /// result in more efficient code on some platforms.
    /// The return value is a result indicating whether the `new` value was
    /// written and containing the previous and now unlinked value.
    /// On success this value is guaranteed to be equal to `current` and can be
    /// safely reclaimed as long as the *uniqueness* invariant is maintained.
    /// On failure, a [struct](CompareExchangeFailure) is returned that contains
    /// both the actual value and the value that was previously attempted to be
    /// inserted (`new`).
    /// This is necessary, because it is possible to attempt insertion of
    /// move-only types such as [`Owned`] or [`Unlinked`], which would otherwise
    /// be irretrievably lost when the `compare_exchange` fails.
    /// The actually loaded value is [`Unprotected`].
    ///
    /// `compare_exchange` takes two [`Ordering`][ordering] arguments to
    /// describe the memory ordering of this operation.
    /// The first describes the required ordering if the operation succeeds
    /// while the second describes the required ordering when the operation
    /// fails.
    /// Using [`Acquire`][acquire] as success ordering makes the store part of
    /// this operation [`Relaxed`][relaxed], and using [`Release`][release]
    /// makes the successful load [`Relaxed`][relaxed].
    /// The failure ordering can only be [`SeqCst`][seq_cst],
    /// [`Acquire`][acquire] or [`Relaxed`][relaxed] and must be equivalent to
    /// or weaker than the success ordering.
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [relaxed]: core::sync::atomic::Ordering::Relaxed
    /// [acquire]: core::sync::atomic::Ordering::Acquire
    /// [release]: core::sync::atomic::Ordering::Release
    /// [seq_cst]: core::sync::atomic::Ordering::SeqCst
    #[inline]
    pub fn compare_exchange_weak<C, S>(
        &self,
        current: C,
        new: S,
        success: Ordering,
        failure: Ordering,
    ) -> Result<C::Unlinked, CompareExchangeFailure<T, R, S, N>>
    where
        C: Compare<Item = T, MarkBits = N, Reclaimer = R>,
        S: Store<Item = T, MarkBits = N, Reclaimer = R>,
    {
        let current = MarkedPointer::into_marked_ptr(current);
        let new = MarkedPointer::into_marked_ptr(new);

        self.inner
            .compare_exchange_weak(current, new, success, failure)
            .map(|ptr| unsafe { C::Unlinked::from_marked_ptr(ptr) })
            .map_err(|ptr| CompareExchangeFailure {
                loaded: unsafe { Option::from_marked_ptr(ptr) },
                input: unsafe { S::from_marked_ptr(new) },
                _marker: PhantomData,
            })
    }

    /// Takes the value out of the pointer as an optional [`Owned`], leaving a
    /// `null` pointer in its place.
    ///
    /// This is similar to [`Option::take`][Option::take] and is useful for
    /// manually dropping the value pointed-to by the [`Atomic`], since
    /// [`Owned`] values behave like `Box` when they are dropped.
    #[inline]
    pub fn take(&mut self) -> Option<Owned<T, R, N>> {
        // this is safe because the mutable reference ensures no concurrent access is possible
        MarkedNonNull::new(self.inner.swap(MarkedPtr::null(), Ordering::Relaxed))
            .map(|ptr| unsafe { Owned::from_marked_non_null(ptr) })
            .value()
    }
}

impl<T, N: Unsigned> Atomic<T, Leaking, N> {
    /// Loads an optional [`Shared`] reference from the `Atomic`.
    ///
    /// Since [`Leaking`] never frees memory of retired records, this is always
    /// safe even without any guards.
    ///
    /// # Panics
    ///
    /// Panics if `order` is [`Release`][release] or [`AcqRel`][acq_rel].
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [release]: core::sync::atomic::Ordering::Release
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
    #[inline]
    pub fn load_shared(&self, order: Ordering) -> Option<Shared<T, Leaking, N>> {
        self.load_marked_shared(order).value()
    }

    /// Loads a [`Shared`] reference wrapped in a [`Marked`] from the `Atomic`.
    ///
    /// Since [`Leaking`] never frees memory of retired records, this is always
    /// safe even without any guards.
    ///
    /// # Panics
    ///
    /// Panics if `order` is [`Release`][release] or [`AcqRel`][acq_rel].
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [release]: core::sync::atomic::Ordering::Release
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
    #[inline]
    pub fn load_marked_shared(&self, order: Ordering) -> Marked<Shared<T, Leaking, N>> {
        MarkedNonNull::new(self.inner.load(order))
            .map(|ptr| Shared { inner: ptr, _marker: PhantomData })
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl Default
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: Reclaim, N: Unsigned> Default for Atomic<T, R, N> {
    #[inline]
    fn default() -> Self {
        Self::null()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl From
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: Reclaim, N: Unsigned> From<T> for Atomic<T, R, N> {
    #[inline]
    fn from(val: T) -> Self {
        Self::new(val)
    }
}

impl<T, R: Reclaim, N: Unsigned> From<Owned<T, R, N>> for Atomic<T, R, N> {
    #[inline]
    fn from(owned: Owned<T, R, N>) -> Self {
        Self { inner: AtomicMarkedPtr::from(Owned::into_marked_ptr(owned)), _marker: PhantomData }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl Debug & Pointer
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: Reclaim, N: Unsigned> fmt::Debug for Atomic<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.inner.load(Ordering::SeqCst).decompose();
        f.debug_struct("Atomic").field("ptr", &ptr).field("tag", &tag).finish()
    }
}

impl<T, R: Reclaim, N: Unsigned> fmt::Pointer for Atomic<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.load(Ordering::SeqCst), f)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl Internal
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: Reclaim, N: Unsigned> Internal for Atomic<T, R, N> {}

////////////////////////////////////////////////////////////////////////////////////////////////////
// CompareExchangeFailure
////////////////////////////////////////////////////////////////////////////////////////////////////

/// The returned error type for a failed [`compare_exchange`](Atomic::compare_exchange) or
/// [`compare_exchange_weak`](Atomic::compare_exchange_weak) operation.
#[derive(Debug)]
pub struct CompareExchangeFailure<T, R, S, N>
where
    R: Reclaim,
    S: Store<Item = T, MarkBits = N, Reclaimer = R>,
    N: Unsigned,
{
    /// The actually loaded value
    pub loaded: Option<Unprotected<T, R, N>>,
    /// The value with which the failed swap was attempted
    pub input: S,
    // prevents construction outside of the current module
    _marker: PhantomData<R>,
}
