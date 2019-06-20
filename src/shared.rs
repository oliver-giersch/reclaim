//! Inherent implementation and trait implementations for the [`Shared`] type.

use core::fmt;
use core::marker::PhantomData;
use core::ops::Deref;

use typenum::Unsigned;

use crate::internal::Internal;
use crate::pointer::{Marked, MarkedNonNull, MarkedPointer, NonNullable};
use crate::{Reclaim, Shared, Unprotected};

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl Copy & Clone
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'g, T, R, N> Clone for Shared<'g, T, N, R> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner, _marker: PhantomData }
    }
}

impl<'g, T, R, N> Copy for Shared<'g, T, R, N> {}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl MarkedPointer
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'g, T, R: Reclaim, N: Unsigned> MarkedPointer for Shared<'g, T, R, N> {
    impl_trait!(shared);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl inherent
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'g, T, R: Reclaim, N: Unsigned> Shared<'g, T, R, N> {
    impl_inherent!(shared);

    /// Decomposes the marked reference, returning the reference itself and the
    /// separated tag.
    #[inline]
    pub fn decompose_ref(shared: Self) -> (&'g T, usize) {
        let (ptr, tag) = shared.inner.decompose();
        unsafe { (&*ptr.as_ptr(), tag) }
    }

    /// Consumes and decomposes the marked reference, returning only the
    /// reference itself.
    ///
    /// # Example
    ///
    /// Derefencing a [`Shared`] through the [`Deref`] implementation ties the
    /// returned reference to the shorter lifetime of the `shared` itself.
    /// Use this function to get a reference with the full lifetime `'g`.
    ///
    /// ```
    /// use core::sync::atomic::Ordering::Relaxed;
    ///
    /// use reclaim::prelude::*;
    /// use reclaim::typenum::U0;
    /// use reclaim::leak::Shared;
    ///
    /// type Atomic<T> = reclaim::leak::Atomic<T, U0>;
    /// type Guard = reclaim::leak::Guard;
    ///
    /// let atomic = Atomic::new("string");
    ///
    /// let mut guard = Guard::new();
    /// let shared = atomic.load(Relaxed, &mut guard);
    ///
    /// let reference = Shared::into_ref(shared.unwrap());
    /// assert_eq!(reference, &"string");
    /// ```
    #[inline]
    pub fn into_ref(shared: Self) -> &'g T {
        unsafe { &*shared.inner.decompose_ptr() }
    }

    /// Converts the `Shared` reference into an [`Unprotected`].
    pub fn into_unprotected(shared: Self) -> Unprotected<T, R, N> {
        Unprotected { inner: shared.inner, _marker: PhantomData }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl AsRef
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'g, T, R: Reclaim, N: Unsigned> AsRef<T> for Shared<'g, T, R, N> {
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { self.inner.as_ref() }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl Deref
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'g, T, R: Reclaim, N: Unsigned> Deref for Shared<'g, T, R, N> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.inner.as_ref() }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl Debug & Pointer
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'g, T, R: Reclaim, N: Unsigned> fmt::Debug for Shared<'g, T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.inner.decompose();
        f.debug_struct("Shared").field("ptr", &ptr).field("tag", &tag).finish()
    }
}

impl<'g, T, R: Reclaim, N: Unsigned> fmt::Pointer for Shared<'g, T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl NonNullable
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'g, T, R, N: Unsigned> NonNullable for Shared<'g, T, R, N> {
    type Item = T;
    type MarkBits = N;

    #[inline]
    fn into_marked_non_null(self) -> MarkedNonNull<Self::Item, Self::MarkBits> {
        self.inner
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl Internal
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'g, T, R, N> Internal for Shared<'g, T, R, N> {}
