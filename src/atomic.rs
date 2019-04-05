use core::fmt;
use core::marker::PhantomData;
use core::sync::atomic::Ordering;

use typenum::Unsigned;

use crate::marked::{AtomicMarkedPtr, MarkedNonNull, MarkedPtr};
use crate::pointer::MarkedPointer;
use crate::{NotEqual, Owned, Protect, Reclaim, Shared, Unlinked, Unprotected};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Atomic
////////////////////////////////////////////////////////////////////////////////////////////////////

/// An atomic markable pointer type to an owned heap allocated value similar to `AtomicPtr`.
pub struct Atomic<T, N, R> {
    inner: AtomicMarkedPtr<T, N>,
    _marker: PhantomData<(T, R)>,
}

unsafe impl<T, N: Unsigned, R: Reclaim> Send for Atomic<T, N, R> where T: Send + Sync {}
unsafe impl<T, N: Unsigned, R: Reclaim> Sync for Atomic<T, N, R> where T: Send + Sync {}

impl<T, N, R> Atomic<T, N, R> {
    /// Creates a new `null` pointer.
    #[inline]
    pub const fn null() -> Self {
        Self {
            inner: AtomicMarkedPtr::null(),
            _marker: PhantomData,
        }
    }

    /// Gets a reference to the underlying raw atomic markable pointer.
    #[inline]
    pub const fn as_raw(&self) -> &AtomicMarkedPtr<T, N> {
        &self.inner
    }
}

impl<T, N: Unsigned, R: Reclaim> Atomic<T, N, R> {
    /// Creates a new [`Atomic`](struct.Atomic.html) by allocating specified `val` on the heap.
    #[inline]
    pub fn new(val: T) -> Self {
        Self::from(Owned::from(val))
    }

    /// Loads a raw marked value from the pointer.
    ///
    /// `load_raw` takes an `Ordering` argument, which describes the memory ordering
    /// of this operation.
    ///
    /// # Panics
    ///
    /// Panics if `order` is `Release` or `AcqRel`.
    #[inline]
    pub fn load_raw(&self, order: Ordering) -> MarkedPtr<T, N> {
        self.inner.load(order)
    }

    /// Loads a value from the pointer and stores it within `guard`.
    ///
    /// If the loaded value is non-null, the value is guaranteed to be protected
    /// from reclamation as long as it is stored within `guard`. This method
    /// relies on `Protected::acquire`, which is required to be lock-free, but
    /// not wait free.
    ///
    /// `load` takes an `Ordering` argument, which describes the memory ordering
    /// of this operation.
    ///
    /// # Panics
    ///
    /// Panics if `order` is `Release` or `AcqRel`.
    #[inline]
    pub fn load<'g>(
        &self,
        order: Ordering,
        guard: &'g mut impl Protect<Item = T, MarkBits = N, Reclaimer = R>,
    ) -> Option<Shared<'g, T, N, R>> {
        guard.acquire(&self, order)
    }

    /// Loads a value from the pointer and stores it within `guard`, but only if
    /// the loaded value equals `expected`.
    ///
    /// If the loaded value is non-null, the value is guaranteed to be protected
    /// from reclamation as long as it is stored within `guard`. This method
    /// relies on `Protected::acquire_if_equal`, which is required to be wait free.
    ///
    /// `load_if_equal` takes an `Ordering` argument, which describes the memory
    /// ordering of this operation.
    ///
    /// # Panics
    ///
    /// Panics if `order` is `Release` or `AcqRel`.
    #[inline]
    pub fn load_if_equal<'g>(
        &self,
        compare: MarkedPtr<T, N>,
        order: Ordering,
        guard: &'g mut impl Protect<Item = T, MarkBits = N, Reclaimer = R>,
    ) -> Result<Option<Shared<'g, T, N, R>>, NotEqual> {
        guard.acquire_if_equal(self, compare, order)
    }

    /// TODO: Doc...
    #[inline]
    pub fn load_unprotected(&self, order: Ordering) -> Option<Unprotected<T, N, R>> {
        MarkedNonNull::new(self.inner.load(order)).map(|ptr| Unprotected {
            inner: ptr,
            _marker: PhantomData,
        })
    }

    /// Stores either `null` or a valid address to an owned heap allocated value
    /// into the pointer.
    ///
    /// Note, that overwriting a non-null value through `store` will very likely
    /// lead to memory leaks, since instances of [`Atomic`](struct.Atomic.html)
    /// will most commonly be associated wit some kind of uniqueness invariants
    /// in order to be sound.
    ///
    /// `store` takes an `Ordering` argument, which describes the memory ordering
    /// orf this operation.
    #[inline]
    pub fn store(&self, ptr: impl Store<Item = T, MarkBits = N, Reclaimer = R>, order: Ordering) {
        self.inner.store(ptr.into_marked(), order);
    }

    /// TODO: Doc...
    #[inline]
    pub fn swap(
        &self,
        ptr: impl Store<Item = T, MarkBits = N, Reclaimer = R>,
        order: Ordering,
    ) -> Option<Unlinked<T, N, R>> {
        let res = self.inner.swap(ptr.into_marked(), order);
        // this is safe because the pointer is no longer accessible by other threads
        // (there can still be outstanding references that were loaded before the swap)
        unsafe { Unlinked::try_from_marked(res) }
    }

    /// TODO: Doc...
    #[inline]
    pub fn compare_exchange<C, S>(
        &self,
        current: C,
        new: S,
        success: Ordering,
        failure: Ordering,
    ) -> Result<C::Unlinked, CompareExchangeFailure<T, N, R, S>>
    where
        C: Compare<Item = T, MarkBits = N, Reclaimer = R>,
        S: Store<Item = T, MarkBits = N, Reclaimer = R>,
    {
        let current = current.into_marked();
        let new = new.into_marked();

        self.inner
            .compare_exchange(current, new, success, failure)
            .map(|ptr| unsafe { C::Unlinked::from_marked(ptr) })
            .map_err(|ptr| CompareExchangeFailure {
                loaded: ptr,
                input: unsafe { S::from_marked(new) },
                _marker: PhantomData,
            })
    }

    /// TODO: Doc...
    #[inline]
    pub fn compare_exchange_weak<C, S>(
        &self,
        current: C,
        new: S,
        success: Ordering,
        failure: Ordering,
    ) -> Result<C::Unlinked, CompareExchangeFailure<T, N, R, S>>
    where
        C: Compare<Item = T, MarkBits = N, Reclaimer = R>,
        S: Store<Item = T, MarkBits = N, Reclaimer = R>,
    {
        let current = current.into_marked();
        let new = new.into_marked();

        self.inner
            .compare_exchange_weak(current, new, success, failure)
            .map(|ptr| unsafe { C::Unlinked::from_marked(ptr) })
            .map_err(|ptr| CompareExchangeFailure {
                loaded: ptr,
                input: unsafe { S::from_marked(new) },
                _marker: PhantomData,
            })
    }

    /// TODO: Doc...
    #[inline]
    pub fn take(&mut self) -> Option<Owned<T, N, R>> {
        // this is safe because the mutable reference ensures no concurrent access is possible
        MarkedNonNull::new(self.inner.swap(MarkedPtr::null(), Ordering::Relaxed))
            .map(|ptr| unsafe { Owned::from_marked_non_null(ptr) })
    }
}

impl<T, N: Unsigned, R: Reclaim> Default for Atomic<T, N, R> {
    #[inline]
    fn default() -> Self {
        Self::null()
    }
}

impl<T, N: Unsigned, R: Reclaim> From<T> for Atomic<T, N, R> {
    #[inline]
    fn from(val: T) -> Self {
        Self::new(val)
    }
}

impl<T, N: Unsigned, R: Reclaim> From<Owned<T, N, R>> for Atomic<T, N, R> {
    #[inline]
    fn from(owned: Owned<T, N, R>) -> Self {
        Self {
            inner: AtomicMarkedPtr::from(Owned::into_marked(owned)),
            _marker: PhantomData,
        }
    }
}

impl<T, N: Unsigned, R: Reclaim> fmt::Debug for Atomic<T, N, R> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.inner.load(Ordering::SeqCst).decompose();
        f.debug_struct("Atomic")
            .field("ptr", &ptr)
            .field("tag", &tag)
            .finish()
    }
}

impl<T, N: Unsigned, R: Reclaim> fmt::Pointer for Atomic<T, N, R> {
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
pub struct CompareExchangeFailure<T, N, R, S>
where
    N: Unsigned,
    R: Reclaim,
    S: Store<Item = T, MarkBits = N, Reclaimer = R>,
{
    /// The actually loaded value
    pub loaded: MarkedPtr<T, N>,
    /// The value for which the failed swap was attempted
    pub input: S,
    // prevents construction outside of the current module
    _marker: PhantomData<R>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Store (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Trait for pointer types that can be stored in an `Atomic`.
pub trait Store: MarkedPointer + Sized {
    type Reclaimer: Reclaim;
}

impl<T, N: Unsigned, R: Reclaim> Store for Owned<T, N, R> {
    type Reclaimer = R;
}

impl<T, N: Unsigned, R: Reclaim> Store for Option<Owned<T, N, R>> {
    type Reclaimer = R;
}

impl<'g, T, N: Unsigned, R: Reclaim> Store for Shared<'g, T, N, R> {
    type Reclaimer = R;
}

impl<'g, T, N: Unsigned, R: Reclaim> Store for Option<Shared<'g, T, N, R>> {
    type Reclaimer = R;
}

impl<T, N: Unsigned, R: Reclaim> Store for Unlinked<T, N, R> {
    type Reclaimer = R;
}

impl<T, N: Unsigned, R: Reclaim> Store for Option<Unlinked<T, N, R>> {
    type Reclaimer = R;
}

impl<T, N: Unsigned, R: Reclaim> Store for Unprotected<T, N, R> {
    type Reclaimer = R;
}

impl<T, N: Unsigned, R: Reclaim> Store for Option<Unprotected<T, N, R>> {
    type Reclaimer = R;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Compare (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait Compare: MarkedPointer + Sized {
    type Reclaimer: Reclaim;
    type Unlinked: MarkedPointer<Item = Self::Item, MarkBits = Self::MarkBits>;
}

impl<'g, T, N: Unsigned, R: Reclaim> Compare for Shared<'g, T, N, R> {
    type Reclaimer = R;
    type Unlinked = Unlinked<T, N, R>;
}

impl<'g, T, N: Unsigned, R: Reclaim> Compare for Option<Shared<'g, T, N, R>> {
    type Reclaimer = R;
    type Unlinked = Option<Unlinked<T, N, R>>;
}

impl<T, N: Unsigned, R: Reclaim> Compare for Unprotected<T, N, R> {
    type Reclaimer = R;
    type Unlinked = Unlinked<T, N, R>;
}

impl<T, N: Unsigned, R: Reclaim> Compare for Option<Unprotected<T, N, R>> {
    type Reclaimer = R;
    type Unlinked = Option<Unlinked<T, N, R>>;
}
