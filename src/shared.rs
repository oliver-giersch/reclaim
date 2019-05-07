use core::fmt;
use core::marker::PhantomData;
use core::ops::Deref;

use typenum::Unsigned;

use crate::pointer::{Internal, Marked, MarkedNonNull, MarkedPointer, NonNullable};
use crate::{LocalReclaim, Record, Shared};

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
    impl_trait_option!(Shared<'g, T, R, N>);
}

impl<'g, T, R: LocalReclaim, N: Unsigned> MarkedPointer for Marked<Shared<'g, T, R, N>> {
    impl_trait_marked!(Shared<'g, T, R, N>);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// inherent
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'g, T, R: LocalReclaim, N: Unsigned> Shared<'g, T, R, N> {
    impl_inherent!();

    /// Returns a reference to the header type that is automatically
    /// allocated alongside every new record.
    #[inline]
    pub fn header(self) -> &'g R::RecordHeader {
        unsafe { Record::<T, R>::get_header(self.inner.decompose_non_null()) }
    }

    /// Decomposes the marked reference, returning the reference itself and the
    /// separated tag.
    #[inline]
    pub fn decompose_ref(self) -> (&'g T, usize) {
        let (ptr, tag) = self.inner.decompose();
        unsafe { (&*ptr.as_ptr(), tag) }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// AsRef
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'g, T, R: LocalReclaim, N: Unsigned> AsRef<T> for Shared<'g, T, R, N> {
    #[inline]
    fn as_ref(&self) -> &T {
        unsafe { self.inner.as_ref() }
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
// Deref
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'g, T, R: LocalReclaim, N: Unsigned> Deref for Shared<'g, T, R, N> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.inner.as_ref() }
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
