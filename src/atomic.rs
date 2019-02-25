use core::marker::PhantomData;
use core::mem;
use core::sync::atomic::Ordering;

use crate::marked::{AtomicMarkedPtr, MarkedPtr};
use crate::owned::Owned;
use crate::pointer::MarkedPointer;
use crate::{NotEqual, Protected, Reclaim, Shared, StatelessAlloc, Unlinked};
use crate::MarkedNonNull;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Atomic
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Doc...
pub struct Atomic<T, R: Reclaim<A>, A: StatelessAlloc> {
    inner: AtomicMarkedPtr<T>,
    _marker: PhantomData<(T, R, A)>,
}

unsafe impl<T, R: Reclaim<A>, A: StatelessAlloc> Send for Atomic<T, R, A> where T: Send + Sync {}
unsafe impl<T, R: Reclaim<A>, A: StatelessAlloc> Sync for Atomic<T, R, A> where T: Send + Sync {}

impl<T, R: Reclaim<A>, A: StatelessAlloc> Atomic<T, R, A> {
    /// TODO: Doc...
    pub fn null() -> Self {
        Self {
            inner: AtomicMarkedPtr::null(),
            _marker: PhantomData,
        }
    }

    /// TODO: Doc...
    pub fn as_raw(&self) -> &AtomicMarkedPtr<T> {
        &self.inner
    }

    /// TODO: Doc...
    pub unsafe fn load_unprotected<'a>(&self, order: Ordering) -> Option<Shared<'a, T, R, A>> {
        MarkedNonNull::new(self.inner.load(order))
            .map(|ptr| Shared::from_marked_non_null(ptr))
    }

    /// TODO: Doc...
    pub fn load<'g>(
        &self,
        order: Ordering,
        guard: &'g mut impl Protected<A, Item = T, Reclaimer = R>,
    ) -> Option<Shared<'g, T, R, A>> {
        guard.acquire(&self, order)
    }

    /// TODO: Doc...
    pub fn load_if_equal<'g, Guard: Protected<A, Item = T, Reclaimer = R>>(
        &self,
        compare: MarkedPtr<T>,
        order: Ordering,
        guard: &'g mut Guard,
    ) -> Result<Option<Shared<'g, T, Guard::Reclaimer, A>>, NotEqual> {
        guard.acquire_if_equal(self, compare, order)
    }

    pub fn load_raw(&self, order: Ordering) -> MarkedPtr<T> {
        self.inner.load(order)
    }

    // allow for Owned, Unlinked, Leaked, Shared or Options<_> thereof
    /// TODO: Doc...
    pub fn store(&self, ptr: impl Store<A, Item = T, Reclaimer = R>, order: Ordering) {
        self.inner.store(ptr.into_marked(), order);
    }

    /// TODO: Doc...
    pub fn swap(
        &self,
        ptr: impl Store<A, Item = T, Reclaimer = R>,
        order: Ordering,
    ) -> Option<Unlinked<T, R, A>> {
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
    ) -> Result<C::Unlinked, CompareExchangeFailure<T, S, R, A>>
    where
        C: Compare<A, Item = T, Reclaimer = R>,
        S: Store<A, Item = T, Reclaimer = R>,
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
    ) -> Result<C::Unlinked, CompareExchangeFailure<T, S, R, A>>
    where
        C: Compare<A, Item = T, Reclaimer = R>,
        S: Store<A, Item = T, Reclaimer = R>,
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

impl<T, R: Reclaim<A>, A: StatelessAlloc> Drop for Atomic<T, R, A> {
    fn drop(&mut self) {
        let ptr = self.inner.load(Ordering::Relaxed);
        if !ptr.is_null() {
            mem::drop(unsafe { Owned::<T, R, A>::from_marked(ptr) });
        }
    }
}

impl<T, R: Reclaim<A>, A: StatelessAlloc> From<Owned<T, R, A>> for Atomic<T, R, A> {
    fn from(owned: Owned<T, R, A>) -> Self {
        Self {
            inner: AtomicMarkedPtr::from(Owned::into_marked(owned)),
            _marker: PhantomData,
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// CompareExchangeFailure
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct CompareExchangeFailure<T, S, R, A>
where
    S: Store<A, Item = T, Reclaimer = R>,
    R: Reclaim<A>,
    A: StatelessAlloc,
{
    pub loaded: MarkedPtr<T>,
    pub input: S,
    // prevents construction outside of the current module
    _marker: PhantomData<(R, A)>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Store (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Trait for pointer types that can be stored in an `AtomicOwned`.
pub trait Store<A: StatelessAlloc>: MarkedPointer + Sized + Internal {
    type Reclaimer: Reclaim<A>;
}

impl<T, R: Reclaim<A>, A: StatelessAlloc> Store<A> for Owned<T, R, A> {
    type Reclaimer = R;
}

impl<T, R: Reclaim<A>, A: StatelessAlloc> Store<A> for Option<Owned<T, R, A>> {
    type Reclaimer = R;
}

impl<'g, T, R: Reclaim<A>, A: StatelessAlloc> Store<A> for Shared<'g, T, R, A> {
    type Reclaimer = R;
}

impl<'g, T, R: Reclaim<A>, A: StatelessAlloc> Store<A> for Option<Shared<'g, T, R, A>> {
    type Reclaimer = R;
}

impl<T, R: Reclaim<A>, A: StatelessAlloc> Store<A> for Unlinked<T, R, A> {
    type Reclaimer = R;
}

impl<T, R: Reclaim<A>, A: StatelessAlloc> Store<A> for Option<Unlinked<T, R, A>> {
    type Reclaimer = R;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Compare (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

pub trait Compare<A: StatelessAlloc>: MarkedPointer + Sized + Internal {
    type Reclaimer: Reclaim<A>;
    // FIXME: maybe add extra trait for Unlinked and Option<Unlinked>
    type Unlinked: MarkedPointer<Item = Self::Item>;
}

impl<'g, T, R: Reclaim<A>, A: StatelessAlloc> Compare<A> for Shared<'g, T, R, A> {
    type Reclaimer = R;
    type Unlinked = Unlinked<T, R, A>;
}

impl<'g, T, R: Reclaim<A>, A: StatelessAlloc> Compare<A> for Option<Shared<'g, T, R, A>> {
    type Reclaimer = R;
    type Unlinked = Option<Unlinked<T, R, A>>;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// InternalOnly (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Marker trait that is not exported from this crate
pub trait Internal {}

impl<T, R: Reclaim<A>, A: StatelessAlloc> Internal for Owned<T, R, A> {}
impl<T, R: Reclaim<A>, A: StatelessAlloc> Internal for Option<Owned<T, R, A>> {}
impl<'g, T, R: Reclaim<A>, A: StatelessAlloc> Internal for Shared<'g, T, R, A> {}
impl<'g, T, R: Reclaim<A>, A: StatelessAlloc> Internal for Option<Shared<'g, T, R, A>> {}
impl<T, R: Reclaim<A>, A: StatelessAlloc> Internal for Unlinked<T, R, A> {}
impl<T, R: Reclaim<A>, A: StatelessAlloc> Internal for Option<Unlinked<T, R, A>> {}