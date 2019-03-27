use core::fmt;
use core::marker::PhantomData;
use core::sync::atomic::Ordering;

use typenum::Unsigned;

use crate::marked::{AtomicMarkedPtr, MarkedNonNull, MarkedPtr};
use crate::owned::Owned;
use crate::pointer::MarkedPointer;
use crate::{NotEqual, Protect, Reclaim, Shared, Unlinked, Unprotected};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Atomic
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Doc...
pub struct Atomic<T, N: Unsigned, R> {
    inner: AtomicMarkedPtr<T, N>,
    _marker: PhantomData<(T, R)>,
}

unsafe impl<T, N: Unsigned, R: Reclaim> Send for Atomic<T, N, R> where T: Send + Sync {}
unsafe impl<T, N: Unsigned, R: Reclaim> Sync for Atomic<T, N, R> where T: Send + Sync {}

impl<T, N: Unsigned, R> Atomic<T, N, R> {
    /// TODO: Doc...
    #[inline]
    pub const fn null() -> Self {
        Self {
            inner: AtomicMarkedPtr::null(),
            _marker: PhantomData,
        }
    }

    /// TODO: Doc...
    #[inline]
    pub const fn as_raw(&self) -> &AtomicMarkedPtr<T, N> {
        &self.inner
    }
}

impl<T, N: Unsigned, R: Reclaim> Atomic<T, N, R> {
    #[inline]
    pub fn new(val: T) -> Self {
        Self::from(Owned::from(val))
    }

    /// TODO: Doc...
    #[inline]
    pub fn load_raw(&self, order: Ordering) -> MarkedPtr<T, N> {
        self.inner.load(order)
    }

    /// TODO: Doc...
    #[inline]
    pub fn load<'g>(
        &self,
        order: Ordering,
        guard: &'g mut impl Protect<Item = T, MarkBits = N, Reclaimer = R>,
    ) -> Option<Shared<'g, T, N, R>> {
        guard.acquire(&self, order)
    }

    /// TODO: Doc...
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
    pub fn load_unprotected<'a>(&self, order: Ordering) -> Option<Unprotected<T, N, R>> {
        MarkedNonNull::new(self.inner.load(order)).map(|ptr| Unprotected {
            inner: ptr,
            _marker: PhantomData,
        })
    }

    /// TODO: Doc...
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
        unsafe { Unlinked::from_marked(res) }
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
        // TODO: this is safe because
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

#[derive(Debug)]
pub struct CompareExchangeFailure<T, N, R, S>
where
    N: Unsigned,
    R: Reclaim,
    S: Store<Item = T, MarkBits = N, Reclaimer = R>,
{
    pub loaded: MarkedPtr<T, N>,
    pub input: S,
    // prevents construction outside of the current module
    _marker: PhantomData<(R)>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Store (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Trait for pointer types that can be stored in an `AtomicOwned`.
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
