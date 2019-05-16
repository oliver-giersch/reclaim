use core::fmt;
use core::marker::PhantomData;

use typenum::Unsigned;

use crate::pointer::{Internal, Marked, MarkedNonNull, MarkedPointer, NonNullable};
use crate::{LocalReclaim, Unprotected};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Copy & Clone
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R, N> Clone for Unprotected<T, R, N> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner, _marker: PhantomData }
    }
}

impl<T, R: LocalReclaim, N> Copy for Unprotected<T, R, N> {}

////////////////////////////////////////////////////////////////////////////////////////////////////
// MarkedPointer
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> MarkedPointer for Unprotected<T, R, N> {
    impl_trait!(unprotected);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// inherent
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> Unprotected<T, R, N> {
    impl_inherent!(unprotected);

    /// Dereferences the `Unprotected`, returning the resulting reference which
    /// is not bound to the lifetime of `self`.
    ///
    /// # Safety
    ///
    /// Since the pointed-to value is not protected from reclamation it could
    /// be freed at any point (even before calling this method). Hence, this
    /// method is only safe to call if the caller can guarantee that no
    /// reclamation can occur, e.g. when records are never retired at all.
    #[inline]
    pub unsafe fn deref_unprotected<'a>(self) -> &'a T {
        self.inner.as_ref_unbounded()
    }

    /// Decomposes the `Unprotected`, returning the reference (which is not
    /// bound to the lifetime of `self`) itself and the separated tag.
    ///
    /// # Safety
    ///
    /// Since the pointed-to value is not protected from reclamation it could
    /// be freed at any point (even before calling this method). Hence, this
    /// method is only safe to call if the caller can guarantee that no
    /// reclamation can occur, e.g. when records are never retired at all.
    #[inline]
    pub unsafe fn decompose_ref_unprotected<'a>(self) -> (&'a T, usize) {
        self.inner.decompose_ref_unbounded()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Debug & Pointer
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> fmt::Debug for Unprotected<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.inner.decompose();
        f.debug_struct("Shared").field("ptr", &ptr).field("tag", &tag).finish()
    }
}

impl<T, R: LocalReclaim, N: Unsigned> fmt::Pointer for Unprotected<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// NonNullable
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R, N: Unsigned> NonNullable for Unprotected<T, R, N> {
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

impl<T, R, N> Internal for Unprotected<T, R, N> {}