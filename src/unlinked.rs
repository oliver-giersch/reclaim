use core::fmt;
use core::marker::PhantomData;

use typenum::Unsigned;

use crate::pointer::{Internal, Marked, MarkedNonNull, MarkedPointer, NonNullable};
use crate::{LocalReclaim, Reclaim, Unlinked};

////////////////////////////////////////////////////////////////////////////////////////////////////
// MarkedPointer
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> MarkedPointer for Unlinked<T, R, N> {
    impl_trait!();
}

impl<T, R: LocalReclaim, N: Unsigned> MarkedPointer for Option<Unlinked<T, R, N>> {
    impl_trait_option!();
}

impl<T, R: LocalReclaim, N: Unsigned> MarkedPointer for Marked<Unlinked<T, R, N>> {
    impl_trait_marked!(Unlinked);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// inherent
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> Unlinked<T, R, N> {
    impl_inherent!();

    /// TODO: Doc...
    #[inline]
    pub unsafe fn deref(&self) -> &T {
        self.inner.as_ref()
    }

    /// TODO: Doc...
    #[inline]
    pub unsafe fn decompose_ref(&self) -> (&T, usize) {
        self.inner.decompose_ref()
    }

    /// TODO: Doc...
    #[inline]
    pub unsafe fn retire_local(self, local: &R::Local)
    where
        T: 'static,
    {
        R::retire_local(local, self)
    }

    /// TODO: Doc...
    #[inline]
    pub unsafe fn retire_local_unchecked(self, local: &R::Local) {
        R::retire_local_unchecked(local, self)
    }
}

impl<T, R: Reclaim, N: Unsigned> Unlinked<T, R, N> {
    /// TODO: Doc...
    #[inline]
    pub unsafe fn retire(self)
    where
        T: 'static,
    {
        R::retire(self)
    }

    /// TODO: Doc...
    #[inline]
    pub unsafe fn retire_unchecked(self) {
        R::retire_unchecked(self)
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
// Internal
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R, N> Internal for Unlinked<T, R, N> {}
impl<T, R, N> Internal for Option<Unlinked<T, R, N>> {}
impl<T, R, N> Internal for Marked<Unlinked<T, R, N>> {}

////////////////////////////////////////////////////////////////////////////////////////////////////
// NonNullable
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R, N> NonNullable for Unlinked<T, R, N> {}
