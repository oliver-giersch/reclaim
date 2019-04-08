use core::fmt;
use core::marker::PhantomData;

use typenum::Unsigned;

use crate::marked::MarkedNonNull;
use crate::pointer::{Internal, MarkedPointer};
use crate::{Reclaim, Shared};

impl<'g, T, N, R> Internal for Shared<'g, T, N, R> {}
impl<'g, T, N, R> Internal for Option<Shared<'g, T, N, R>> {}

impl<'g, T, N, R: Reclaim> Clone for Shared<'g, T, N, R> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
            _marker: PhantomData,
        }
    }
}

impl<'g, T, N, R: Reclaim> Copy for Shared<'g, T, N, R> {}

impl<'g, T, N: Unsigned, R: Reclaim> MarkedPointer for Shared<'g, T, N, R> {
    impl_marked_pointer!();
}

impl<'g, T, N: Unsigned, R: Reclaim> MarkedPointer for Option<Shared<'g, T, N, R>> {
    impl_marked_pointer_option!();
}

impl<'g, T, N: Unsigned, R: Reclaim> Shared<'g, T, N, R> {
    impl_inherent!();

    /// TODO: Doc...
    #[inline]
    pub unsafe fn deref(self) -> &'g T {
        &*self.inner.decompose_ptr()
    }

    /// TODO: Doc...
    #[inline]
    pub unsafe fn decompose_ref(self) -> (&'g T, usize) {
        let (ptr, tag) = self.inner.decompose();
        (&*ptr.as_ptr(), tag)
    }
}

impl<'g, T, N: Unsigned, R: Reclaim> fmt::Debug for Shared<'g, T, N, R> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.inner.decompose();
        f.debug_struct("Shared")
            .field("ptr", &ptr)
            .field("tag", &tag)
            .finish()
    }
}

impl<'g, T, N: Unsigned, R: Reclaim> fmt::Pointer for Shared<'g, T, N, R> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}
