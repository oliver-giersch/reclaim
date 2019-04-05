use core::fmt;
use core::marker::PhantomData;

use typenum::Unsigned;

use crate::marked::MarkedNonNull;
use crate::pointer::{Internal, MarkedPointer};
use crate::{Reclaim, Unlinked};

impl<T, N, R> Internal for Unlinked<T, N, R> {}
impl<T, N, R> Internal for Option<Unlinked<T, N, R>> {}

impl<T, N: Unsigned, R: Reclaim> MarkedPointer for Unlinked<T, N, R> {
    impl_marked_pointer!();
}

impl<T, N: Unsigned, R: Reclaim> MarkedPointer for Option<Unlinked<T, N, R>> {
    impl_marked_pointer_option!();
}

impl<T, N: Unsigned, R: Reclaim> Unlinked<T, N, R> {
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

impl<T, N: Unsigned, R: Reclaim> fmt::Debug for Unlinked<T, N, R> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.inner.decompose();
        f.debug_struct("Shared")
            .field("ptr", &ptr)
            .field("tag", &tag)
            .finish()
    }
}

impl<T, N: Unsigned, R: Reclaim> fmt::Pointer for Unlinked<T, N, R> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}
