use core::fmt;
use core::marker::PhantomData;

use typenum::Unsigned;

use crate::internal::Internal;
use crate::pointer::{Marked, MarkedNonNull, MarkedPointer, NonNullable};
use crate::{Reclaim, Shared, Unprotected};

/********** impl Clone ****************************************************************************/

impl<T, R, N> Clone for Unprotected<T, R, N> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner, _marker: PhantomData }
    }
}

/********** impl Copy *****************************************************************************/

impl<T, R: Reclaim, N> Copy for Unprotected<T, R, N> {}

/********** impl MarkedPointer ********************************************************************/

impl<T, R: Reclaim, N: Unsigned> MarkedPointer for Unprotected<T, R, N> {
    impl_trait!(unprotected);
}

/********** impl inherent *************************************************************************/

impl<T, R: Reclaim, N: Unsigned> Unprotected<T, R, N> {
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
    ///
    /// # Example
    ///
    /// ...
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

    /// Consumes the `unprotected` and converts it to a [`Shared`] reference
    /// with arbitrary lifetime.
    ///
    /// # Safety
    ///
    /// The returned reference is not in fact protected and could be reclaimed
    /// by other threads, so the caller has to ensure no concurrent reclamation
    /// is possible.
    #[inline]
    pub unsafe fn into_shared<'a>(unprotected: Self) -> Shared<'a, T, R, N> {
        Shared::from_marked_non_null(unprotected.inner)
    }

    /// Casts the [`Unprotected`] to a reference to a different type.
    #[inline]
    pub fn cast<U>(unprotected: Self) -> Unprotected<U, R, N> {
        Unprotected { inner: unprotected.inner.cast(), _marker: PhantomData }
    }
}

/********** impl Debug ****************************************************************************/

impl<T, R: Reclaim, N: Unsigned> fmt::Debug for Unprotected<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.inner.decompose();
        f.debug_struct("Shared").field("ptr", &ptr).field("tag", &tag).finish()
    }
}

/********** impl Pointer **************************************************************************/

impl<T, R: Reclaim, N: Unsigned> fmt::Pointer for Unprotected<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}

/********** impl NonNullable **********************************************************************/

impl<T, R, N: Unsigned> NonNullable for Unprotected<T, R, N> {
    type Item = T;
    type MarkBits = N;

    #[inline]
    fn into_marked_non_null(self) -> MarkedNonNull<Self::Item, Self::MarkBits> {
        self.inner
    }
}

/********** impl Internal *************************************************************************/

impl<T, R, N> Internal for Unprotected<T, R, N> {}
