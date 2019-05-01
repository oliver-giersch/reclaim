use core::fmt;
use core::marker::PhantomData;

use typenum::Unsigned;

use crate::marked::MarkedNonNull;
use crate::pointer_old::{Internal, MarkedPointer};
use crate::{LocalReclaim, Unprotected};

impl<T, R, N> Internal for Unprotected<T, R, N> {}
impl<T, R, N> Internal for Option<Unprotected<T, R, N>> {}

impl<T, R, N> Clone for Unprotected<T, R, N> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
            _marker: PhantomData,
        }
    }
}

impl<T, R: LocalReclaim, N> Copy for Unprotected<T, R, N> {}

impl<T, R: LocalReclaim, N: Unsigned> MarkedPointer for Unprotected<T, R, N> {
    impl_marked_pointer!();
}

impl<T, R: LocalReclaim, N: Unsigned> MarkedPointer for Option<Unprotected<T, R, N>> {
    impl_marked_pointer_option!();
}

impl<T, R: LocalReclaim, N: Unsigned> Unprotected<T, R, N> {
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
        &*self.inner.decompose_ptr()
    }
}

impl<T, R: LocalReclaim, N: Unsigned> fmt::Debug for Unprotected<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.inner.decompose();
        f.debug_struct("Shared")
            .field("ptr", &ptr)
            .field("tag", &tag)
            .finish()
    }
}

impl<T, R: LocalReclaim, N: Unsigned> fmt::Pointer for Unprotected<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}
