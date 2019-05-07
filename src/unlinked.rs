use core::fmt;
use core::marker::PhantomData;
use core::ops::Deref;

use typenum::Unsigned;

use crate::pointer::{Internal, Marked, MarkedNonNull, MarkedPointer, NonNullable};
use crate::{LocalReclaim, Reclaim, Record, Unlinked};

////////////////////////////////////////////////////////////////////////////////////////////////////
// MarkedPointer
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> MarkedPointer for Unlinked<T, R, N> {
    impl_trait!();
}

impl<T, R: LocalReclaim, N: Unsigned> MarkedPointer for Option<Unlinked<T, R, N>> {
    impl_trait_option!(Unlinked);
}

impl<T, R: LocalReclaim, N: Unsigned> MarkedPointer for Marked<Unlinked<T, R, N>> {
    impl_trait_marked!(Unlinked);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// inherent
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> Unlinked<T, R, N> {
    impl_inherent!();

    /// Returns a reference to the header type that is automatically
    /// allocated alongside every new record.
    #[inline]
    pub fn header(&self) -> &R::RecordHeader {
        unsafe { Record::<T, R>::get_header(self.inner.decompose_non_null()) }
    }

    /// Decomposes the marked reference, returning the reference itself and the
    /// separated tag.
    #[inline]
    pub fn decompose_ref(&self) -> (&T, usize) {
        unsafe { self.inner.decompose_ref() }
    }

    /// Retires a record by calling [`retire_local`][retire] on the generic reclamation parameter `R`.
    ///
    /// # Safety
    ///
    /// The same caveats as with [`LocalReclaim::retire_local`][retire] apply.
    ///
    /// [retire]: crate::LocalReclaim::retire_local
    #[inline]
    pub unsafe fn retire_local(self, local: &R::Local)
    where
        T: 'static,
    {
        R::retire_local(local, self)
    }

    /// Retires a record by calling [`retire_local_unchecked`][retire_unchecked] on the generic
    /// reclamation parameter `R`.
    ///
    /// # Safety
    ///
    /// The same caveats as with [`LocalReclaim::retire_local_unchecked`][retire_unchecked] apply.
    ///
    /// [retire_unchecked]: crate::LocalReclaim::retire_local_unchecked
    #[inline]
    pub unsafe fn retire_local_unchecked(self, local: &R::Local) {
        R::retire_local_unchecked(local, self)
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
    /// The same caveats as with [`Reclaim::retire_unchecked`][retire_unchecked] apply.
    ///
    /// [retire_unchecked]: crate::Reclaim::retire_unchecked
    #[inline]
    pub unsafe fn retire_unchecked(self) {
        R::retire_unchecked(self)
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
// Internal
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R, N> Internal for Unlinked<T, R, N> {}
impl<T, R, N> Internal for Option<Unlinked<T, R, N>> {}
impl<T, R, N> Internal for Marked<Unlinked<T, R, N>> {}

////////////////////////////////////////////////////////////////////////////////////////////////////
// NonNullable
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R, N> NonNullable for Unlinked<T, R, N> {}
