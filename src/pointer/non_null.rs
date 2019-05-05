use core::cmp;
use core::convert::TryFrom;
use core::fmt;
use core::marker::PhantomData;
use core::ptr::NonNull;

use typenum::{IsGreaterOrEqual, True, Unsigned};

use crate::pointer::{
    self, InvalidNullError,
    Marked::{self, Null, OnlyTag, Ptr},
    MarkedNonNull, MarkedPtr, NonNullable,
};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Copy & Clone
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, N> Clone for MarkedNonNull<T, N> {
    #[inline]
    fn clone(&self) -> Self {
        Self::from(self.inner)
    }
}

impl<T, N> Copy for MarkedNonNull<T, N> {}

////////////////////////////////////////////////////////////////////////////////////////////////////
// inherent
////////////////////////////////////////////////////////////////////////////////////////////////////

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
            (raw, _) if !raw.is_null() => unsafe { Ptr(Self::new_unchecked(ptr)) },
            (_, 0) => Null,
            (_, tag) => OnlyTag(tag),
        }
    }

    /// Creates a new `MarkedNonNull` that is dangling, but well-aligned.
    ///
    /// This is useful for initializing types which lazily allocate, like
    /// `Vec::new` does.
    ///
    /// Note that the pointer value may potentially represent a valid pointer to
    /// a `T`, which means this must not be used as a "not yet initialized"
    /// sentinel value. Types that lazily allocate must track initialization by
    /// some other means.
    #[inline]
    pub fn dangling() -> Self {
        Self { inner: NonNull::dangling(), _marker: PhantomData }
    }

    /// Converts the pointer to the equivalent [`MarkedPtr`][crate::pointer::MarkedPtr].
    #[inline]
    pub fn into_marked_ptr(self) -> MarkedPtr<T, N> {
        MarkedPtr::new(self.inner.as_ptr())
    }

    /// Composes a new marked non-null pointer from a non-null pointer and a tag
    /// value.
    #[inline]
    pub fn compose(ptr: NonNull<T>, tag: usize) -> Self {
        debug_assert_eq!(0, ptr.as_ptr() as usize & Self::MARK_MASK, "`ptr` is not well aligned");
        unsafe { Self::from(NonNull::new_unchecked(pointer::compose::<_, N>(ptr.as_ptr(), tag))) }
    }

    /// Decomposes the marked pointer, returning the separated raw
    /// [`NonNull`][core::ptr::NonNull] pointer and its tag.
    #[inline]
    pub fn decompose(self) -> (NonNull<T>, usize) {
        let (ptr, tag) = pointer::decompose(self.inner.as_ptr() as usize, Self::MARK_BITS);
        (unsafe { NonNull::new_unchecked(ptr) }, tag)
    }

    /// Decomposes the marked pointer, returning only the separated raw pointer.
    #[inline]
    pub fn decompose_ptr(self) -> *mut T {
        pointer::decompose_ptr(self.inner.as_ptr() as usize, Self::MARK_BITS)
    }

    /// Decomposes the marked pointer, returning only the separated raw
    /// [`NonNull`][core::ptr::NonNull] pointer.
    #[inline]
    pub fn decompose_non_null(self) -> NonNull<T> {
        unsafe {
            NonNull::new_unchecked(pointer::decompose_ptr(
                self.inner.as_ptr() as usize,
                Self::MARK_BITS,
            ))
        }
    }

    /// Decomposes the marked pointer, returning only the separated tag.
    #[inline]
    pub fn decompose_tag(self) -> usize {
        pointer::decompose_tag::<T>(self.inner.as_ptr() as usize, Self::MARK_BITS)
    }

    /// Decomposes the marked pointer, dereferences the the raw pointer and returns both the
    /// reference and the separated tag.
    ///
    /// The resulting lifetime is bound to self so this behaves "as if"
    /// it were actually an instance of T that is getting borrowed. If a longer
    /// (unbound) lifetime is needed, use e.g. `&*my_ptr.decompose_ptr()`.
    #[inline]
    pub unsafe fn decompose_ref(&self) -> (&T, usize) {
        let (ptr, tag) = self.decompose();
        (&*ptr.as_ptr(), tag)
    }

    /// Decomposes the marked pointer, mutably dereferences the the raw pointer and returns both the
    /// mutable reference and the separated tag.
    ///
    /// The resulting lifetime is bound to self so this behaves "as if"
    /// it were actually an instance of T that is getting borrowed. If a longer
    /// (unbound) lifetime is needed, use e.g. `&mut *my_ptr.decompose_ptr()`.
    #[inline]
    pub unsafe fn decompose_mut(&mut self) -> (&mut T, usize) {
        let (ptr, tag) = self.decompose();
        (&mut *ptr.as_ptr(), tag)
    }

    /// Decomposes the marked pointer, returning only the dereferenced raw pointer.
    ///
    /// The resulting lifetime is bound to self so this behaves "as if"
    /// it were actually an instance of T that is getting borrowed. If a longer
    /// (unbound) lifetime is needed, use e.g. `&*my_ptr.decompose_ptr()`.
    #[inline]
    pub unsafe fn as_ref(&self) -> &T {
        &*self.decompose_non_null().as_ptr()
    }

    /// Decomposes the marked pointer, returning only the mutably dereferenced raw pointer.
    ///
    /// The resulting lifetime is bound to self so this behaves "as if"
    /// it were actually an instance of T that is getting borrowed. If a longer
    /// (unbound) lifetime is needed, use e.g. `&mut *my_ptr.decompose_ptr()`.
    #[inline]
    pub unsafe fn as_mut(&mut self) -> &mut T {
        &mut *self.decompose_non_null().as_ptr()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Debug & Pointer (fmt)
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, N: Unsigned> fmt::Debug for MarkedNonNull<T, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.decompose();
        f.debug_struct("MarkedNonNull").field("ptr", &ptr).field("tag", &tag).finish()
    }
}

impl<T, N: Unsigned> fmt::Pointer for MarkedNonNull<T, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.decompose_non_null(), f)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// From & TryFrom
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, N> From<NonNull<T>> for MarkedNonNull<T, N> {
    #[inline]
    fn from(ptr: NonNull<T>) -> Self {
        Self { inner: ptr, _marker: PhantomData }
    }
}

impl<'a, T, N: Unsigned> From<&'a T> for MarkedNonNull<T, N> {
    #[inline]
    fn from(reference: &'a T) -> Self {
        Self::from(NonNull::from(reference))
    }
}

impl<'a, T, N: Unsigned> From<&'a mut T> for MarkedNonNull<T, N> {
    #[inline]
    fn from(reference: &'a mut T) -> Self {
        Self::from(NonNull::from(reference))
    }
}

impl<T, N: Unsigned> TryFrom<MarkedPtr<T, N>> for MarkedNonNull<T, N> {
    type Error = InvalidNullError;

    #[inline]
    fn try_from(ptr: MarkedPtr<T, N>) -> Result<Self, Self::Error> {
        match ptr.decompose() {
            (raw, _) if raw.is_null() => Err(InvalidNullError),
            _ => unsafe { Ok(MarkedNonNull::new_unchecked(ptr)) },
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// comparison
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, N> PartialEq for MarkedNonNull<T, N> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T, N> PartialOrd for MarkedNonNull<T, N> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl<T, N> PartialEq<MarkedPtr<T, N>> for MarkedNonNull<T, N> {
    #[inline]
    fn eq(&self, other: &MarkedPtr<T, N>) -> bool {
        self.inner.as_ptr() == other.inner
    }
}

impl<T, N> PartialOrd<MarkedPtr<T, N>> for MarkedNonNull<T, N> {
    #[inline]
    fn partial_cmp(&self, other: &MarkedPtr<T, N>) -> Option<cmp::Ordering> {
        self.inner.as_ptr().partial_cmp(&other.inner)
    }
}

impl<T, N> NonNullable for MarkedNonNull<T, N> {}

#[cfg(test)]
mod tests {
    use std::ptr;

    use typenum::U2;

    use crate::align::Aligned4;

    type MarkedPtr<T> = crate::pointer::MarkedPtr<T, U2>;
    type MarkedNonNull<T> = crate::pointer::MarkedNonNull<T, U2>;

    #[test]
    fn new() {
        let reference = &mut Aligned4(1);
        let unmarked = MarkedPtr::new(reference);

        let marked = MarkedNonNull::new(unmarked);
        assert_eq!(unsafe { marked.unwrap_ptr().decompose_ref() }, (&Aligned4(1), 0));

        let marked = MarkedNonNull::new(MarkedPtr::compose(reference, 0b11));
        assert_eq!(unsafe { marked.unwrap_ptr().decompose_ref() }, (&Aligned4(1), 0b11));

        let null: *mut Aligned4<i32> = ptr::null_mut();
        let marked = MarkedNonNull::new(MarkedPtr::compose(null, 0b11));
        assert_eq!(marked.unwrap_tag(), 0b11);

        let marked = MarkedNonNull::new(MarkedPtr::compose(null, 0));
        assert!(marked.is_null());
    }
}
