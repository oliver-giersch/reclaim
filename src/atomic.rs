use core::marker::PhantomData;
use core::mem;
use core::sync::atomic::Ordering;

#[cfg(feature = "global_alloc")]
use std::alloc::Global;

use crate::marked::{AtomicMarkedPtr, MarkedPtr};
use crate::owned::Owned;
use crate::pointer::Pointer;
use crate::{NotEqual, Protected, Reclaim, Shared, StatelessAlloc, Unlinked};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Atomic
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Doc...
#[cfg(feature = "global_alloc")]
pub struct Atomic<T, R: Reclaim<A>, A: StatelessAlloc = Global> {
    inner: AtomicMarkedPtr<T>,
    _marker: PhantomData<(T, R, A)>,
}

/// TODO: Doc...
#[cfg(not(feature = "global_alloc"))]
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
        let raw = self.inner.load(order);
        if raw.is_null() {
            None
        } else {
            Some(Shared::from_raw(raw.into_inner()))
        }
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

    // allow for Owned, Unlinked, Leaked, Shared or Options<_> thereof
    /// TODO: Doc...
    pub fn store(&self, ptr: impl Store<A, Item = T, Reclaimer = R>, order: Ordering) {
        let ptr = ptr.into_raw();
        self.inner.store(MarkedPtr::from(ptr), order);
    }

    /// TODO: Doc...
    pub fn swap(
        &self,
        ptr: impl Store<A, Item = T, Reclaimer = R>,
        order: Ordering,
    ) -> Option<Unlinked<T, R, A>> {
        let marked = MarkedPtr::from(ptr.into_raw());
        // this is safe because `Option<Unlinked>` has same representation as a raw pointer
        unsafe { mem::transmute(self.inner.swap(marked, order).into_inner()) }
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
        let current = MarkedPtr::from(current.into_raw());
        let new = MarkedPtr::from(new.into_raw());

        self.inner
            .compare_exchange(current, new, success, failure)
            .map(|ptr| unsafe { C::Unlinked::from_raw(ptr.into_inner()) })
            .map_err(|ptr| CompareExchangeFailure {
                loaded: ptr,
                input: unsafe { S::from_raw(new.into_inner()) },
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
        let current = MarkedPtr::from(current.into_raw());
        let new = MarkedPtr::from(new.into_raw());

        self.inner
            .compare_exchange_weak(current, new, success, failure)
            .map(|ptr| unsafe { C::Unlinked::from_raw(ptr.into_inner()) })
            .map_err(|ptr| CompareExchangeFailure {
                loaded: ptr,
                input: unsafe { S::from_raw(new.into_inner()) },
                _marker: PhantomData,
            })
    }
}

impl<T, R: Reclaim<A>, A: StatelessAlloc> Drop for Atomic<T, R, A> {
    fn drop(&mut self) {
        let ptr = self.inner.load(Ordering::SeqCst).decompose_ptr();
        if !ptr.is_null() {
            // the tag doesn't matter anymore at this point
            mem::drop(unsafe { Owned::<T, R, A>::from_raw(ptr) });
        }
    }
}

impl<T, R: Reclaim<A>, A: StatelessAlloc> From<Owned<T, R, A>> for Atomic<T, R, A> {
    fn from(owned: Owned<T, R, A>) -> Self {
        Self {
            inner: AtomicMarkedPtr::from(Owned::into_raw(owned)),
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
pub trait Store<A: StatelessAlloc>: Pointer + Sized + InternalOnly {
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

pub trait Compare<A: StatelessAlloc>: Pointer + Sized + InternalOnly {
    type Reclaimer: Reclaim<A>;
    // FIXME: maybe add extra trait for Unlinked and Option<Unlinked>
    type Unlinked: Pointer<Item = Self::Item>;
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
pub trait InternalOnly {}

impl<T, R: Reclaim<A>, A: StatelessAlloc> InternalOnly for Owned<T, R, A> {}
impl<T, R: Reclaim<A>, A: StatelessAlloc> InternalOnly for Option<Owned<T, R, A>> {}
impl<'g, T, R: Reclaim<A>, A: StatelessAlloc> InternalOnly for Shared<'g, T, R, A> {}
impl<'g, T, R: Reclaim<A>, A: StatelessAlloc> InternalOnly for Option<Shared<'g, T, R, A>> {}
impl<T, R: Reclaim<A>, A: StatelessAlloc> InternalOnly for Unlinked<T, R, A> {}
impl<T, R: Reclaim<A>, A: StatelessAlloc> InternalOnly for Option<Unlinked<T, R, A>> {}