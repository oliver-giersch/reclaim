use core::fmt;
use core::mem;
use core::ptr::NonNull;

use crate::marked::{self, MarkedNonNull};

impl<T> MarkedNonNull<T> {
    pub const MARK_BITS: usize = marked::mark_bits::<T>();
    pub const MARK_MASK: usize = marked::mark_mask::<T>();
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
        Self {
            inner: NonNull::dangling(),
        }
    }

    /// Creates a new `MarkedNonNull`.
    ///
    /// # Safety
    ///
    /// `ptr` may be marked, but must be non-null.
    pub const unsafe fn new_unchecked(ptr: *mut T) -> Self {
        Self {
            inner: NonNull::new_unchecked(ptr),
        }
    }

    /// Creates a new `MarkedNonNull` if `ptr` is non-null.
    ///
    /// `ptr` may be marked, but the tag will be discarded, when `ptr` is null and `None` is
    /// returned.
    pub fn new(ptr: *mut T) -> Option<Self> {
        let (raw, tag) = marked::decompose::<T>(ptr as usize);
        if raw.is_null() {
            None
        } else {
            Some(MarkedNonNull::compose(unsafe { NonNull::new_unchecked(raw) }, tag))
        }
    }

    pub fn compose(ptr: NonNull<T>, tag: usize) -> Self {
        debug_assert_eq!(
            ptr.as_ptr() as usize & Self::MARK_MASK,
            0,
            "pointer is not well-aligned"
        );
        Self {
            inner: unsafe { NonNull::new_unchecked(marked::compose(ptr.as_ptr(), tag)) },
        }
    }

    pub fn into_inner(self) -> NonNull<T> {
        self.inner
    }

    pub fn into_inner_raw(self) -> *mut T {
        self.inner.as_ptr()
    }

    pub fn decompose(&self) -> (NonNull<T>, usize) {
        let (ptr, tag) = marked::decompose(self.inner.as_ptr() as usize);
        (unsafe { NonNull::new_unchecked(ptr) }, tag)
    }

    pub fn decompose_ptr(&self) -> *mut T {
        marked::decompose_ptr(self.inner.as_ptr() as usize)
    }

    pub fn decompose_non_null(&self) -> NonNull<T> {
        unsafe { NonNull::new_unchecked(marked::decompose_ptr(self.inner.as_ptr() as usize)) }
    }

    pub fn decompose_tag(&self) -> usize {
        marked::decompose_tag::<T>(self.inner.as_ptr() as usize)
    }

    pub unsafe fn decompose_ref<'a>(&self) -> (&'a T, usize) {
        let (ptr, tag) = self.decompose();
        (&*ptr.as_ptr(), tag)
    }

    pub unsafe fn decompose_mut<'a>(&mut self) -> (&'a mut T, usize) {
        let (ptr, tag) = self.decompose();
        (&mut *ptr.as_ptr(), tag)
    }

    pub unsafe fn as_ref<'a>(&self) -> &'a T {
        &*self.decompose_non_null().as_ptr()
    }

    pub unsafe fn as_mut<'a>(&mut self) -> &'a mut T {
        &mut *self.decompose_non_null().as_ptr()
    }
}

impl<T> Clone for MarkedNonNull<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Copy for MarkedNonNull<T> {}

impl<'a, T> From<&'a T> for MarkedNonNull<T> {
    fn from(reference: &'a T) -> Self {
        unsafe {
            MarkedNonNull {
                inner: NonNull::new_unchecked(reference as *const _ as *mut _),
            }
        }
    }
}

impl<'a, T> From<&'a mut T> for MarkedNonNull<T> {
    fn from(reference: &'a mut T) -> Self {
        unsafe {
            MarkedNonNull {
                inner: NonNull::new_unchecked(reference as *mut _),
            }
        }
    }
}

impl<T> fmt::Debug for MarkedNonNull<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.decompose();
        f.debug_struct("MarkedNonNull")
            .field("ptr", &ptr)
            .field("tag", &tag)
            .finish()
    }
}

impl<T> fmt::Pointer for MarkedNonNull<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.decompose_non_null(), f)
    }
}
