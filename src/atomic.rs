use core::fmt;
use core::marker::PhantomData;
use core::sync::atomic::Ordering;

use typenum::Unsigned;

use crate::leak::Leaking;
use crate::pointer::{AtomicMarkedPtr, Marked, MarkedNonNull, MarkedPointer, MarkedPtr};
use crate::{LocalReclaim, NotEqualError, Owned, Protect, Shared, Unlinked, Unprotected};

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
/// meaning it does not automatically take care of memory deallocation when it
/// goes out of scope.
/// Use the [`take`][Atomic::take] method to extract an (optional) [`Owned`]
/// value, which *does* correctly deallocate memory when it goes out of scope.
pub struct Atomic<T, R, N> {
    inner: AtomicMarkedPtr<T, N>,
    _marker: PhantomData<(T, R)>,
}

unsafe impl<T, R: LocalReclaim, N: Unsigned> Send for Atomic<T, R, N> where T: Send + Sync {}
unsafe impl<T, R: LocalReclaim, N: Unsigned> Sync for Atomic<T, R, N> where T: Send + Sync {}

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

impl<T, R: LocalReclaim, N: Unsigned> Atomic<T, R, N> {
    /// Creates a new [`Atomic`] by allocating specified `val` on the heap.
    #[inline]
    pub fn new(val: T) -> Self {
        Self::from(Owned::from(val))
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
    #[inline]
    pub fn load_raw(&self, order: Ordering) -> MarkedPtr<T, N> {
        self.inner.load(order)
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
    /// need to be dereferenced, but is only used to reinsert it in a different
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

    /// Loads an optional [`Unprotected`] reference from the `Atomic`.
    ///
    /// The returned reference is explicitly **not** protected from reclamation,
    /// meaning another thread could free the value's memory at any time.
    ///
    /// This method is similar to [`load_raw`][Atomic::load_raw], but the
    /// resulting [`Unprotected`] type has stronger guarantees than a raw
    /// [`MarkedPtr`].
    /// It can be useful to load an unprotected pointer if that pointer does not
    /// need to be dereferenced, but is only used to reinsert it in a different
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

    /// Loads a value from the pointer and stores it within `guard`.
    ///
    /// If the loaded value is non-null, the value is guaranteed to be protected
    /// from reclamation for as long as it is stored within `guard`. This method
    /// internally relies on [`acquire`](crate::Protect::acquire).
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
        guard: &'g mut impl Protect<Item = T, MarkBits = N, Reclaimer = R>,
    ) -> Option<Shared<'g, T, R, N>> {
        guard.acquire(&self, order).value()
    }

    /// Loads a value from the pointer and stores it within `guard`.
    /// The protected [`Shared`] value is wrapped in a [`Marked].
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
        guard: &'g mut impl Protect<Item = T, MarkBits = N, Reclaimer = R>,
    ) -> Marked<Shared<'g, T, R, N>> {
        guard.acquire(&self, order)
    }

    /// Loads a value from the pointer and stores it within `guard`, but only if
    /// the loaded value equals `expected`.
    ///
    /// If the loaded value is non-null, the value is guaranteed to be protected
    /// from reclamation for as long as it is stored within `guard`. This method
    /// internally calls [`acquire_if_equal`][Protect::acquire_if_equal].
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
        guard: &'g mut impl Protect<Item = T, MarkBits = N, Reclaimer = R>,
    ) -> Result<Option<Shared<'g, T, R, N>>, NotEqualError> {
        guard.acquire_if_equal(self, expected, order).map(|marked| marked.value())
    }

    /// Loads a value wrapped in a [`Marked] from the pointer and stores it
    /// within `guard`, but only if the loaded value equals `expected`.
    ///
    /// If the loaded value is non-null, the value is guaranteed to be protected
    /// from reclamation for as long as it is stored within `guard`. This method
    /// internally calls [`acquire_if_equal`][Protect::acquire_if_equal].
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
    pub fn load_marked_if_equal<'g>(
        &self,
        expected: MarkedPtr<T, N>,
        order: Ordering,
        guard: &'g mut impl Protect<Item = T, MarkBits = N, Reclaimer = R>,
    ) -> Result<Marked<Shared<'g, T, R, N>>, NotEqualError> {
        guard.acquire_if_equal(self, expected, order)
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
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Default
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> Default for Atomic<T, R, N> {
    #[inline]
    fn default() -> Self {
        Self::null()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// From
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> From<T> for Atomic<T, R, N> {
    #[inline]
    fn from(val: T) -> Self {
        Self::new(val)
    }
}

impl<T, R: LocalReclaim, N: Unsigned> From<Owned<T, R, N>> for Atomic<T, R, N> {
    #[inline]
    fn from(owned: Owned<T, R, N>) -> Self {
        Self { inner: AtomicMarkedPtr::from(Owned::into_marked_ptr(owned)), _marker: PhantomData }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Debug & Pointer
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> fmt::Debug for Atomic<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.inner.load(Ordering::SeqCst).decompose();
        f.debug_struct("Atomic").field("ptr", &ptr).field("tag", &tag).finish()
    }
}

impl<T, R: LocalReclaim, N: Unsigned> fmt::Pointer for Atomic<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.load(Ordering::SeqCst), f)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// CompareExchangeFailure
////////////////////////////////////////////////////////////////////////////////////////////////////

/// The returned error type for a failed [`compare_exchange`](Atomic::compare_exchange) or
/// [`compare_exchange_weak`](Atomic::compare_exchange_weak) operation.
#[derive(Debug)]
pub struct CompareExchangeFailure<T, R, S, N>
where
    R: LocalReclaim,
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

////////////////////////////////////////////////////////////////////////////////////////////////////
// Store (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Trait for pointer types that can be stored in an `Atomic`.
pub trait Store: MarkedPointer + Sized {
    type Reclaimer: LocalReclaim;
}

impl<T, R: LocalReclaim, N: Unsigned> Store for Owned<T, R, N> {
    type Reclaimer = R;
}

impl<T, R: LocalReclaim, N: Unsigned> Store for Option<Owned<T, R, N>> {
    type Reclaimer = R;
}

impl<T, R: LocalReclaim, N: Unsigned> Store for Marked<Owned<T, R, N>> {
    type Reclaimer = R;
}

impl<'g, T, R: LocalReclaim, N: Unsigned> Store for Shared<'g, T, R, N> {
    type Reclaimer = R;
}

impl<'g, T, R: LocalReclaim, N: Unsigned> Store for Option<Shared<'g, T, R, N>> {
    type Reclaimer = R;
}

impl<'g, T, R: LocalReclaim, N: Unsigned> Store for Marked<Shared<'g, T, R, N>> {
    type Reclaimer = R;
}

impl<T, R: LocalReclaim, N: Unsigned> Store for Unlinked<T, R, N> {
    type Reclaimer = R;
}

impl<T, R: LocalReclaim, N: Unsigned> Store for Option<Unlinked<T, R, N>> {
    type Reclaimer = R;
}

impl<T, R: LocalReclaim, N: Unsigned> Store for Marked<Unlinked<T, R, N>> {
    type Reclaimer = R;
}

impl<T, R: LocalReclaim, N: Unsigned> Store for Unprotected<T, R, N> {
    type Reclaimer = R;
}

impl<T, R: LocalReclaim, N: Unsigned> Store for Option<Unprotected<T, R, N>> {
    type Reclaimer = R;
}

impl<T, R: LocalReclaim, N: Unsigned> Store for Marked<Unprotected<T, R, N>> {
    type Reclaimer = R;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Compare (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait Compare: MarkedPointer + Sized {
    type Reclaimer: LocalReclaim;
    type Unlinked: MarkedPointer<Item = Self::Item, MarkBits = Self::MarkBits>;
}

impl<'g, T, R: LocalReclaim, N: Unsigned> Compare for Shared<'g, T, R, N> {
    type Reclaimer = R;
    type Unlinked = Unlinked<T, R, N>;
}

impl<'g, T, R: LocalReclaim, N: Unsigned> Compare for Option<Shared<'g, T, R, N>> {
    type Reclaimer = R;
    type Unlinked = Option<Unlinked<T, R, N>>;
}

impl<'g, T, R: LocalReclaim, N: Unsigned> Compare for Marked<Shared<'g, T, R, N>> {
    type Reclaimer = R;
    type Unlinked = Marked<Unlinked<T, R, N>>;
}

impl<T, R: LocalReclaim, N: Unsigned> Compare for Unprotected<T, R, N> {
    type Reclaimer = R;
    type Unlinked = Unlinked<T, R, N>;
}

impl<T, R: LocalReclaim, N: Unsigned> Compare for Option<Unprotected<T, R, N>> {
    type Reclaimer = R;
    type Unlinked = Option<Unlinked<T, R, N>>;
}

impl<T, R: LocalReclaim, N: Unsigned> Compare for Marked<Unprotected<T, R, N>> {
    type Reclaimer = R;
    type Unlinked = Marked<Unlinked<T, R, N>>;
}
