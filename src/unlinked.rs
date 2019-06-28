//! Inherent implementation and trait implementations for the [`Unlinked`] type.

use core::fmt;
use core::marker::PhantomData;
use core::ops::Deref;

use typenum::Unsigned;

use crate::internal::Internal;
use crate::pointer::{Marked, MarkedNonNull, MarkedPointer, NonNullable};
use crate::{GlobalReclaim, Reclaim, Unlinked, Unprotected};

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl MarkedPointer
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: Reclaim, N: Unsigned> MarkedPointer for Unlinked<T, R, N> {
    impl_trait!(unlinked);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl inherent
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: Reclaim, N: Unsigned> Unlinked<T, R, N> {
    impl_inherent!(unlinked);

    /// Decomposes the marked reference, returning the reference itself and the
    /// separated tag.
    #[inline]
    pub fn decompose_ref(unlinked: &Self) -> (&T, usize) {
        unsafe { unlinked.inner.decompose_ref() }
    }

    /// Converts the `Unlinked` reference into an [`Unprotected`].
    pub fn into_unprotected(shared: Self) -> Unprotected<T, R, N> {
        Unprotected { inner: shared.inner, _marker: PhantomData }
    }

    /// Casts the [`Unlinked`] to a reference to a different type.
    ///
    /// # Safety
    ///
    /// The caller has to ensure the cast is valid.
    #[inline]
    pub unsafe fn cast<U>(unlinked: Self) -> Unlinked<U, R, N> {
        Unlinked { inner: unlinked.inner.cast(), _marker: PhantomData }
    }

    /// Retires a record by calling [`retire_local`][retire] on the generic
    /// reclamation parameter `R`.
    ///
    /// # Safety
    ///
    /// The same caveats as with [`LocalReclaim::retire_local`][retire] apply.
    ///
    /// [retire]: crate::LocalReclaim::retire_local
    ///
    /// # Note
    ///
    /// This method takes `self` as receiver, which means it may conflict with
    /// methods of `T`, since `Unlinked` implements [`Deref`].
    /// This is a deliberate trade-off in favor of better ergonomics around
    /// retiring records.
    #[inline]
    pub unsafe fn retire_local(self, local: &R::Local)
    where
        T: 'static,
    {
        R::retire_local(local, self)
    }

    /// Retires a record by calling [`retire_local_unchecked`][retire_unchecked]
    /// on the generic reclamation parameter `R`.
    ///
    /// # Safety
    ///
    /// The same caveats as with [`LocalReclaim::retire_local_unchecked`][retire_unchecked]
    /// apply.
    ///
    /// [retire_unchecked]: crate::LocalReclaim::retire_local_unchecked
    ///
    /// # Note
    ///
    /// This method takes `self` as receiver, which means it may conflict with
    /// methods of `T`, since `Unlinked` implements [`Deref`].
    /// This is a deliberate trade-off in favor of better ergonomics around
    /// retiring records.
    #[inline]
    pub unsafe fn retire_local_unchecked(self, local: &R::Local) {
        R::retire_local_unchecked(local, self)
    }
}

impl<T, R: GlobalReclaim, N: Unsigned> Unlinked<T, R, N> {
    /// Retires a record by calling [`retire`][retire] on the generic
    /// reclamation parameter `R`.
    ///
    /// # Safety
    ///
    /// The same caveats as with [`Reclaim::retire`][retire] apply.
    ///
    /// [retire]: crate::Reclaim::retire
    ///
    /// # Note
    ///
    /// This method takes `self` as receiver, which means it may conflict with
    /// methods of `T`, since `Unlinked` implements [`Deref`].
    /// This is a deliberate trade-off in favor of better ergonomics around
    /// retiring records.
    #[inline]
    pub unsafe fn retire(self)
    where
        T: 'static,
    {
        R::retire(self)
    }

    /// Retires a record by calling [`retire_unchecked`][retire_unchecked] on
    /// the generic reclamation parameter `R`.
    ///
    /// # Safety
    ///
    /// The same caveats as with [`Reclaim::retire_unchecked`][retire_unchecked]
    /// apply.
    ///
    /// [retire_unchecked]: crate::Reclaim::retire_unchecked
    ///
    /// # Note
    ///
    /// This method takes `self` as receiver, which means it may conflict with
    /// methods of `T`, since `Unlinked` implements [`Deref`].
    /// This is a deliberate trade-off in favor of better ergonomics around
    /// retiring records.
    #[inline]
    pub unsafe fn retire_unchecked(self) {
        R::retire_unchecked(self)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl AsRef
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: Reclaim, N: Unsigned> AsRef<T> for Unlinked<T, R, N> {
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { self.inner.as_ref() }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl Deref
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: Reclaim, N: Unsigned> Deref for Unlinked<T, R, N> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.inner.as_ref() }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl Debug & Pointer
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: Reclaim, N: Unsigned> fmt::Debug for Unlinked<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.inner.decompose();
        f.debug_struct("Shared").field("ptr", &ptr).field("tag", &tag).finish()
    }
}

impl<T, R: Reclaim, N: Unsigned> fmt::Pointer for Unlinked<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl NonNullable
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R, N: Unsigned> NonNullable for Unlinked<T, R, N> {
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

impl<T, R, N> Internal for Unlinked<T, R, N> {}
