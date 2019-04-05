use core::fmt;
use core::marker::PhantomData;

use typenum::Unsigned;

use crate::marked::MarkedNonNull;
use crate::pointer::{Internal, MarkedPointer};
use crate::{Reclaim, Unprotected};

impl<T, N, R> Internal for Unprotected<T, N, R> {}
impl<T, N, R> Internal for Option<Unprotected<T, N, R>> {}

impl<T, N, R: Reclaim> Clone for Unprotected<T, N, R> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
            _marker: PhantomData,
        }
    }
}

impl<T, N, R: Reclaim> Copy for Unprotected<T, N, R> {}

impl<T, N: Unsigned, R: Reclaim> MarkedPointer for Unprotected<T, N, R> {
    impl_marked_pointer!();
}

impl<T, N: Unsigned, R: Reclaim> MarkedPointer for Option<Unprotected<T, N, R>> {
    impl_marked_pointer_option!();
}

impl<T, N: Unsigned, R: Reclaim> Unprotected<T, N, R> {
    impl_inherent!();

    /// TODO: Doc...
    ///
    /// # Safety
    ///
    /// This is generally unsound to call. Only when the caller is able to ensure no memory
    /// reclamation is happening concurrently can it be considered to be safe to dereference an
    /// unprotected pointer loaded from a concurrent data structure. This is e.g. the case when
    /// there are mutable references involved (e.g. during `drop`).
    #[inline]
    pub unsafe fn deref_unprotected<'a>(self) -> &'a T {
        self.inner.as_ref()
    }
}

impl<T, N: Unsigned, R: Reclaim> fmt::Debug for Unprotected<T, N, R> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.inner.decompose();
        f.debug_struct("Shared")
            .field("ptr", &ptr)
            .field("tag", &tag)
            .finish()
    }
}

impl<T, N: Unsigned, R: Reclaim> fmt::Pointer for Unprotected<T, N, R> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}
