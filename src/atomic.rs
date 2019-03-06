use core::marker::PhantomData;
use core::mem;
use core::sync::atomic::Ordering;

use typenum::Unsigned;

use crate::marked::{AtomicMarkedPtr, MarkedPtr};
use crate::owned::Owned;
use crate::pointer::MarkedPointer;
use crate::MarkedNonNull;
use crate::{NotEqual, Protected, Reclaim, Shared, Unlinked};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Atomic
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Doc...
pub struct Atomic<T, N: Unsigned, R: Reclaim> {
    inner: AtomicMarkedPtr<T, N>,
    _marker: PhantomData<(T, R)>,
}

unsafe impl<T, N: Unsigned, R: Reclaim> Send for Atomic<T, N, R> where T: Send + Sync {}
unsafe impl<T, N: Unsigned, R: Reclaim> Sync for Atomic<T, N, R> where T: Send + Sync {}

impl<T, N: Unsigned, R: Reclaim> Atomic<T, N, R> {
    /// TODO: Doc...
    pub const fn null() -> Self {
        Self {
            inner: AtomicMarkedPtr::null(),
            _marker: PhantomData,
        }
    }

    /// TODO: Doc...
    pub fn as_raw(&self) -> &AtomicMarkedPtr<T, N> {
        &self.inner
    }

    /// TODO: Doc...
    pub unsafe fn load_unprotected<'a>(&self, order: Ordering) -> Option<Shared<'a, T, N, R>> {
        MarkedNonNull::new(self.inner.load(order)).map(|ptr| Shared::from_marked_non_null(ptr))
    }

    /// TODO: Doc...
    pub fn load<'g>(
        &self,
        order: Ordering,
        guard: &'g mut impl Protected<Item = T, MarkBits = N, Reclaimer = R>,
    ) -> Option<Shared<'g, T, N, R>> {
        guard.acquire(&self, order)
    }

    /// TODO: Doc...
    pub fn load_if_equal<
        'g,
        Guard: Protected<Item = T, MarkBits = N, Reclaimer = R>,
    >(
        &self,
        compare: MarkedPtr<T, N>,
        order: Ordering,
        guard: &'g mut Guard,
    ) -> Result<Option<Shared<'g, T, N, Guard::Reclaimer>>, NotEqual> {
        guard.acquire_if_equal(self, compare, order)
    }

    /// TODO: Doc...
    pub fn load_raw(&self, order: Ordering) -> MarkedPtr<T, N> {
        self.inner.load(order)
    }

    /// TODO: Doc...
    pub fn store(&self, ptr: impl Store<Item = T, MarkBits = N, Reclaimer = R>, order: Ordering) {
        self.inner.store(ptr.into_marked(), order);
    }

    /// TODO: Doc...
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
}

impl<T, N: Unsigned, R: Reclaim> Drop for Atomic<T, N, R> {
    fn drop(&mut self) {
        let ptr = self.inner.load(Ordering::Relaxed);
        if !ptr.is_null() {
            mem::drop(unsafe { Owned::<T, N, R>::from_marked(ptr) });
        }
    }
}

impl<T, N: Unsigned, R: Reclaim> From<Owned<T, N, R>> for Atomic<T, N, R> {
    fn from(owned: Owned<T, N, R>) -> Self {
        Self {
            inner: AtomicMarkedPtr::from(Owned::into_marked(owned)),
            _marker: PhantomData,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// CompareExchangeFailure
////////////////////////////////////////////////////////////////////////////////////////////////////

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
pub trait Store: MarkedPointer + Sized + Internal {
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

////////////////////////////////////////////////////////////////////////////////////////////////////
// Compare (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait Compare: MarkedPointer + Sized + Internal {
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

////////////////////////////////////////////////////////////////////////////////////////////////////
// InternalOnly (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Marker trait that is not exported from this crate
pub trait Internal {}

impl<T, N: Unsigned, R: Reclaim> Internal for Owned<T, N, R> {}
impl<T, N: Unsigned, R: Reclaim> Internal for Option<Owned<T, N, R>> {}
impl<'g, T, N: Unsigned, R: Reclaim> Internal for Shared<'g, T, N, R> {}
impl<'g, T, N: Unsigned, R: Reclaim> Internal for Option<Shared<'g, T, N, R>> {}
impl<T, N: Unsigned, R: Reclaim> Internal for Unlinked<T, N, R> {}
impl<T, N: Unsigned, R: Reclaim> Internal for Option<Unlinked<T, N, R>> {}
