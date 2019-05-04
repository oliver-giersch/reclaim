use core::fmt;
use core::marker::PhantomData;

use typenum::Unsigned;

use crate::pointer::{Internal, Marked, MarkedNonNull, MarkedPointer, NonNullable};
use crate::{LocalReclaim, Shared};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Copy & Clone
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'g, T, R, N> Clone for Shared<'g, T, N, R> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: self.inner, _marker: PhantomData }
    }
}

impl<'g, T, R, N> Copy for Shared<'g, T, R, N> {}

////////////////////////////////////////////////////////////////////////////////////////////////////
// MarkedPointer
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'g, T, R: LocalReclaim, N: Unsigned> MarkedPointer for Shared<'g, T, R, N> {
    impl_trait!();
}

impl<'g, T, R: LocalReclaim, N: Unsigned> MarkedPointer for Option<Shared<'g, T, R, N>> {
    impl_trait_option!();
}

impl<'g, T, R: LocalReclaim, N: Unsigned> MarkedPointer for Marked<Shared<'g, T, R, N>> {
    impl_trait_marked!(Shared);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// inherent
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'g, T, R: LocalReclaim, N: Unsigned> Shared<'g, T, R, N> {
    impl_inherent!();

    /// TODO: Doc...
    ///
    /// # Safety
    ///
    /// ...
    #[inline]
    pub unsafe fn deref(self) -> &'g T {
        &*self.inner.decompose_ptr()
    }

    /// TODO: Doc...
    ///
    /// # Safety
    ///
    /// ...
    #[inline]
    pub unsafe fn decompose_ref(self) -> (&'g T, usize) {
        let (ptr, tag) = self.inner.decompose();
        (&*ptr.as_ptr(), tag)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Debug & Pointer
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'g, T, R: LocalReclaim, N: Unsigned> fmt::Debug for Shared<'g, T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.inner.decompose();
        f.debug_struct("Shared").field("ptr", &ptr).field("tag", &tag).finish()
    }
}

impl<'g, T, R: LocalReclaim, N: Unsigned> fmt::Pointer for Shared<'g, T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Internal
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'g, T, R, N> Internal for Shared<'g, T, R, N> {}
impl<'g, T, R, N> Internal for Option<Shared<'g, T, R, N>> {}
impl<'g, T, R, N> Internal for Marked<Shared<'g, T, R, N>> {}

////////////////////////////////////////////////////////////////////////////////////////////////////
// NonNullable
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'g, T, R, N> NonNullable for Shared<'g, T, R, N> {}
