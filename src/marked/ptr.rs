use core::cmp::{self, PartialEq, PartialOrd};
use core::fmt;
use core::marker::PhantomData;
use core::ptr::NonNull;

use typenum::{IsGreaterOrEqual, True, Unsigned};

use crate::marked::{self, MarkedNonNull, MarkedPtr};

impl<T, N: Unsigned> Clone for MarkedNonNull<T, N> {
    fn clone(&self) -> Self {
        Self::from(self.inner)
    }
}

impl<T, N: Unsigned> Copy for MarkedNonNull<T, N> {}

impl<T, N: Unsigned> MarkedNonNull<T, N> {
    pub const MARK_BITS: usize = N::USIZE;
    pub const MARK_MASK: usize = marked::mark_mask::<T, N>();
    pub const POINTER_MASK: usize = !Self::MARK_MASK;

    /// Creates a new `MarkedNonNull` that is dangling, but well-aligned.
    ///
    /// This is useful for initializing types which lazily allocate, like
    /// `Vec::new` does.
    ///
    /// Note that the pointer value may potentially represent a valid pointer to
    /// a `T`, which means this must not be used as a "not yet initialized"
    /// sentinel value. Types that lazily allocate must track initialization by
    /// some other means.
    pub fn dangling() -> Self {
        Self::from(NonNull::dangling())
    }

    /// TODO: Doc...
    pub fn convert<M: Unsigned>(other: MarkedNonNull<T, M>) -> Self
    where
        N: IsGreaterOrEqual<M, Output = True>
    {
        Self::from(other.inner)
    }

    /// Creates a new `MarkedNonNull` from e.g. a raw pointer or a marked pointer .
    ///
    /// # Safety
    ///
    /// `ptr` may be marked, but must be non-null.
    pub unsafe fn new_unchecked(ptr: impl Into<MarkedPtr<T, N>>) -> Self {
        Self::from(NonNull::new_unchecked(ptr.into().inner))
    }

    /// Creates a new `MarkedNonNull` if `ptr` is non-null.
    ///
    /// `ptr` may be marked, but the tag will be discarded, when `ptr` is null and only `None` is
    /// returned.
    pub fn new(ptr: impl Into<MarkedPtr<T, N>>) -> Option<Self> {
        match marked::decompose::<T, N>(ptr.into().into_usize()) {
            (raw, _) if raw.is_null() => None,
            (raw, tag) => Some(MarkedNonNull::compose(
                unsafe { NonNull::new_unchecked(raw) },
                tag,
            )),
        }
    }

    /// TODO: Doc...
    pub fn into_marked(self) -> MarkedPtr<T, N> {
        MarkedPtr::new(self.inner.as_ptr())
    }

    /// TODO: Doc...
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
    pub fn decompose(&self) -> (NonNull<T>, usize) {
        let (ptr, tag) = marked::decompose::<_, N>(self.inner.as_ptr() as usize);
        (unsafe { NonNull::new_unchecked(ptr) }, tag)
    }

    /// TODO: Doc...
    pub fn decompose_ptr(&self) -> *mut T {
        marked::decompose_ptr::<_, N>(self.inner.as_ptr() as usize)
    }

    /// TODO: Doc...
    pub fn decompose_non_null(&self) -> NonNull<T> {
        unsafe {
            NonNull::new_unchecked(marked::decompose_ptr::<_, N>(self.inner.as_ptr() as usize))
        }
    }

    /// TODO: Doc...
    pub fn decompose_tag(&self) -> usize {
        marked::decompose_tag::<T, N>(self.inner.as_ptr() as usize)
    }

    /// TODO: Doc...
    pub unsafe fn decompose_ref<'a>(&self) -> (&'a T, usize) {
        let (ptr, tag) = self.decompose();
        (&*ptr.as_ptr(), tag)
    }

    /// TODO: Doc...
    pub unsafe fn decompose_mut<'a>(&mut self) -> (&'a mut T, usize) {
        let (ptr, tag) = self.decompose();
        (&mut *ptr.as_ptr(), tag)
    }

    /// TODO: Doc...
    pub unsafe fn as_ref<'a>(&self) -> &'a T {
        &*self.decompose_non_null().as_ptr()
    }

    /// TODO: Doc...
    pub unsafe fn as_mut<'a>(&mut self) -> &'a mut T {
        &mut *self.decompose_non_null().as_ptr()
    }
}

impl<T, N: Unsigned> fmt::Debug for MarkedNonNull<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.decompose();
        f.debug_struct("MarkedNonNull")
            .field("ptr", &ptr)
            .field("tag", &tag)
            .finish()
    }
}

impl<T, N: Unsigned> From<NonNull<T>> for MarkedNonNull<T, N> {
    fn from(ptr: NonNull<T>) -> Self {
        Self {
            inner: ptr,
            _marker: PhantomData,
        }
    }
}

impl<'a, T, N: Unsigned> From<&'a T> for MarkedNonNull<T, N> {
    fn from(reference: &'a T) -> Self {
        Self::from(NonNull::from(reference))
    }
}

impl<'a, T, N: Unsigned> From<&'a mut T> for MarkedNonNull<T, N> {
    fn from(reference: &'a mut T) -> Self {
        Self::from(NonNull::from(reference))
    }
}

impl<T, N: Unsigned> fmt::Pointer for MarkedNonNull<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.decompose_non_null(), f)
    }
}

impl<T, N: Unsigned> PartialEq<MarkedPtr<T, N>> for MarkedNonNull<T, N> {
    fn eq(&self, other: &MarkedPtr<T, N>) -> bool {
        self.inner.as_ptr() == other.inner
    }
}

impl<T, N: Unsigned> PartialOrd<MarkedPtr<T, N>> for MarkedNonNull<T, N> {
    fn partial_cmp(&self, other: &MarkedPtr<T, N>) -> Option<cmp::Ordering> {
        self.inner.as_ptr().partial_cmp(&other.inner)
    }
}
