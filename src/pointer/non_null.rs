use core::ptr::NonNull;

use typenum::{IsGreaterOrEqual, True, Unsigned};

use crate::pointer::{
    self,
    Marked::{self, Null, OnlyTag, Value},
    MarkedNonNull, MarkedPtr, NonNullable,
};

////////////////////////////////////////////////////////////////////////////////
// Copy & Clone
////////////////////////////////////////////////////////////////////////////////

impl<T, N> Clone for MarkedNonNull<T, N> {
    #[inline]
    fn clone(&self) -> Self {
        Self::from(self.inner)
    }
}

impl<T, N> Copy for MarkedNonNull<T, N> {}

////////////////////////////////////////////////////////////////////////////////
// inherent
////////////////////////////////////////////////////////////////////////////////

impl<T, N: Unsigned> MarkedNonNull<T, N> {
    /// The number of available mark bits for this type.
    pub const MARK_BITS: usize = N::USIZE;
    /// The bitmask for the lower markable bits.
    pub const MARK_MASK: usize = pointer::mark_mask::<T>(Self::MARK_BITS);
    /// The bitmask for the (higher) pointer bits.
    pub const POINTER_MASK: usize = !Self::MARK_MASK;

    /// Converts a marked non-null pointer with `M` potential mark bits to the
    /// **same** marked pointer with `N` potential mark bits, requires that
    /// `N >= M`.
    #[inline]
    pub fn convert<M: Unsigned>(other: MarkedNonNull<T, M>) -> Self
    where
        N: IsGreaterOrEqual<M, Output = True>,
    {
        Self::from(other.inner)
    }

    /// Creates a new `MarkedNonNull` from a marked pointer without checking
    /// for `null`.
    ///
    /// # Safety
    ///
    /// `ptr` may be marked, but must be be neither an unmarked nor a marked
    /// null pointer.
    #[inline]
    pub unsafe fn new_unchecked(ptr: MarkedPtr<T, N>) -> Self {
        Self::from(NonNull::new_unchecked(ptr.inner))
    }

    /// Creates a new `MarkedNonNull` wrapped in a [`Marked`][crate::pointer::Marked]
    /// if `ptr` is non-null.
    pub fn new(ptr: MarkedPtr<T, N>) -> Marked<Self> {
        match ptr.decompose() {
            (raw, 0) if raw.is_null() => Null,
            (raw, tag) if raw.is_null() => OnlyTag(tag),
            _ => unsafe { Value(Self::new_unchecked(ptr)) },
        }
    }
}

impl<T, N> NonNullable for MarkedNonNull<T, N> {}
