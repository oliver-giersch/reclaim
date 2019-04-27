use core::cmp::{self, PartialEq, PartialOrd};
use core::fmt;
use core::marker::PhantomData;
use core::ptr::{self, NonNull};

use typenum::{IsGreaterOrEqual, True, Unsigned};

use crate::marked::{self, MarkedNonNull, MarkedPtr};

impl<T, N> Clone for MarkedNonNull<T, N> {
    #[inline]
    fn clone(&self) -> Self {
        Self::from(self.inner)
    }
}

impl<T, N> Copy for MarkedNonNull<T, N> {}

impl<T, N: Unsigned> MarkedNonNull<T, N> {
    /// The number of available mark bits for this type.
    pub const MARK_BITS: usize = N::USIZE;
    /// The bitmask for the lower markable bits.
    pub const MARK_MASK: usize = marked::mark_mask::<T>(Self::MARK_BITS);
    /// The bitmask for the (higher) pointer bits.
    pub const POINTER_MASK: usize = !Self::MARK_MASK;

    /// TODO: Doc...
    #[inline]
    pub fn convert<M: Unsigned>(other: MarkedNonNull<T, M>) -> Self
    where
        N: IsGreaterOrEqual<M, Output = True>,
    {
        Self::from(other.inner)
    }

    /// Creates a new `MarkedNonNull` from e.g. a raw pointer or a marked pointer .
    ///
    /// # Safety
    ///
    /// `ptr` may be marked, but must be non-null.
    #[inline]
    pub unsafe fn new_unchecked(ptr: MarkedPtr<T, N>) -> Self {
        Self::from(NonNull::new_unchecked(ptr.inner))
    }

    /// Creates a new `MarkedNonNull` if `ptr` is non-null.
    ///
    /// `ptr` may be marked, but the tag will be discarded, when `ptr` is null and only `None` is
    /// returned.
    #[inline]
    pub fn new(ptr: MarkedPtr<T, N>) -> Option<Self> {
        match ptr.into_usize() {
            0 => None,
            _ => Some(unsafe { Self::new_unchecked(ptr) }),
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
        Self {
            inner: NonNull::dangling(),
            _marker: PhantomData,
        }
    }

    /// TODO: Doc...
    #[inline]
    pub fn into_marked(self) -> MarkedPtr<T, N> {
        MarkedPtr::new(self.inner.as_ptr())
    }

    /// Composes a new marked non-null pointer from a non-null pointer and a tag value.
    #[inline]
    pub fn compose(ptr: NonNull<T>, tag: usize) -> Self {
        debug_assert_eq!(
            ptr.as_ptr() as usize & Self::MARK_MASK,
            0,
            "pointer must be properly aligned"
        );
        unsafe {
            Self::from(NonNull::new_unchecked(marked::compose::<_, N>(
                ptr.as_ptr(),
                tag,
            )))
        }
    }

    /// TODO: Doc...
    #[inline]
    pub fn decompose(self) -> (NonNull<T>, usize) {
        let (ptr, tag) = marked::decompose(self.inner.as_ptr() as usize, Self::MARK_BITS);
        (unsafe { NonNull::new_unchecked(ptr) }, tag)
    }

    /// TODO: Doc...
    #[inline]
    pub fn decompose_ptr(self) -> *mut T {
        marked::decompose_ptr(self.inner.as_ptr() as usize, Self::MARK_BITS)
    }

    /// TODO: Doc...
    #[inline]
    pub fn decompose_non_null(self) -> NonNull<T> {
        unsafe {
            NonNull::new_unchecked(marked::decompose_ptr(
                self.inner.as_ptr() as usize,
                Self::MARK_BITS,
            ))
        }
    }

    /// TODO: Doc...
    #[inline]
    pub fn decompose_tag(self) -> usize {
        marked::decompose_tag::<T>(self.inner.as_ptr() as usize, Self::MARK_BITS)
    }

    /// TODO: Doc...
    #[inline]
    pub unsafe fn decompose_ref(&self) -> (&T, usize) {
        let (ptr, tag) = self.decompose();
        (&*ptr.as_ptr(), tag)
    }

    /// TODO: Doc...
    #[inline]
    pub unsafe fn decompose_mut(&mut self) -> (&mut T, usize) {
        let (ptr, tag) = self.decompose();
        (&mut *ptr.as_ptr(), tag)
    }

    /// TODO: Doc...
    #[inline]
    pub unsafe fn as_ref(&self) -> &T {
        &*self.decompose_non_null().as_ptr()
    }

    /// TODO: Doc...
    #[inline]
    pub unsafe fn as_mut<'a>(self) -> &'a mut T {
        &mut *self.decompose_non_null().as_ptr()
    }
}

impl<T, N: Unsigned> fmt::Debug for MarkedNonNull<T, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.decompose();
        f.debug_struct("MarkedNonNull")
            .field("ptr", &ptr)
            .field("tag", &tag)
            .finish()
    }
}

impl<T, N> From<NonNull<T>> for MarkedNonNull<T, N> {
    #[inline]
    fn from(ptr: NonNull<T>) -> Self {
        Self {
            inner: ptr,
            _marker: PhantomData,
        }
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

impl<T, N: Unsigned> fmt::Pointer for MarkedNonNull<T, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.decompose_non_null(), f)
    }
}

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
