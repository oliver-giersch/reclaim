use core::fmt;
use core::marker::PhantomData;
use core::ops::Deref;

use typenum::Unsigned;

use crate::pointer::{Internal, Marked, MarkedNonNull, MarkedPointer, NonNullable};
use crate::{LocalReclaim, Reclaim, Unlinked, Unprotected};

////////////////////////////////////////////////////////////////////////////////////////////////////
// MarkedPointer
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> MarkedPointer for Unlinked<T, R, N> {
    impl_trait!(unlinked);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// inherent
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> Unlinked<T, R, N> {
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

    /// Retires a record by calling [`retire_local`][retire] on the generic
    /// reclamation parameter `R`.
    ///
    /// # Safety
    ///
    /// The same caveats as with [`LocalReclaim::retire_local`][retire] apply.
    ///
    /// [retire]: crate::LocalReclaim::retire_local
    #[inline]
    pub unsafe fn retire_local(unlinked: Self, local: &R::Local)
    where
        T: 'static,
    {
        R::retire_local(local, unlinked)
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
    #[inline]
    pub unsafe fn retire_local_unchecked(unlinked: Self, local: &R::Local) {
        R::retire_local_unchecked(local, unlinked)
    }
}

impl<T, R: Reclaim, N: Unsigned> Unlinked<T, R, N> {
    /// Retires a record by calling [`retire`][retire] on the generic
    /// reclamation parameter `R`.
    ///
    /// # Safety
    ///
    /// The same caveats as with [`Reclaim::retire`][retire] apply.
    ///
    /// [retire]: crate::Reclaim::retire
    #[inline]
    pub unsafe fn retire(unlinked: Self)
    where
        T: 'static,
    {
        R::retire(unlinked)
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
    #[inline]
    pub unsafe fn retire_unchecked(unlinked: Self) {
        R::retire_unchecked(unlinked)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// AsRef
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> AsRef<T> for Unlinked<T, R, N> {
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { self.inner.as_ref() }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Deref
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> Deref for Unlinked<T, R, N> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.inner.as_ref() }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Debug & Pointer
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> fmt::Debug for Unlinked<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.inner.decompose();
        f.debug_struct("Shared").field("ptr", &ptr).field("tag", &tag).finish()
    }
}

impl<T, R: LocalReclaim, N: Unsigned> fmt::Pointer for Unlinked<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// NonNullable
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
// Internal
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R, N> Internal for Unlinked<T, R, N> {}