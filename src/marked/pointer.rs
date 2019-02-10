use core::fmt;
use core::ptr;

use crate::marked::{self, MarkedPtr};

impl<T> Clone for MarkedPtr<T> {
    fn clone(&self) -> Self {
        Self { inner: self.inner }
    }
}

impl<T> Copy for MarkedPtr<T> {}

impl<T> MarkedPtr<T> {
    pub const MARK_BITS: usize = marked::mark_bits::<T>();
    pub const MARK_MASK: usize = marked::mark_mask::<T>();
    pub const POINTER_MASK: usize = !Self::MARK_MASK;

    /// Creates a new unmarked null pointer.
    pub const fn null() -> Self {
        Self {
            inner: ptr::null_mut(),
        }
    }

    /// Creates a new marked pointer from the numeric representation of a marked pointer
    pub const fn from_usize(val: usize) -> Self {
        Self {
            inner: val as *mut _,
        }
    }

    /// Creates a new unmarked pointer
    pub fn new(ptr: *mut T) -> Self {
        debug_assert_eq!(
            ptr as usize & Self::MARK_MASK,
            0,
            "pointer is not well-aligned"
        );
        Self { inner: ptr }
    }

    /// Composes a new marked pointer.
    pub fn compose(ptr: *mut T, tag: usize) -> Self {
        debug_assert_eq!(
            ptr as usize & Self::MARK_MASK,
            0,
            "pointer is not well-aligned"
        );
        Self {
            inner: marked::compose(ptr, tag),
        }
    }

    /// Gets the inner representation of the pointer with its tag.
    pub fn into_inner(self) -> *mut T {
        self.inner
    }

    /// Gets the numeric inner representation of the pointer with its tag.
    pub fn into_inner_usize(self) -> usize {
        self.inner as usize
    }

    /// Decomposes the marked pointer and returns the separated raw pointer and its tag.
    pub fn decompose(&self) -> (*mut T, usize) {
        marked::decompose(self.into_inner_usize())
    }

    /// Decomposes the marked pointer and returns only the separated raw pointer.
    pub fn decompose_ptr(&self) -> *mut T {
        marked::decompose_ptr(self.into_inner_usize())
    }

    /// Decomposes the marked pointer and returns only the separated tag.
    pub fn decompose_tag(&self) -> usize {
        marked::decompose_tag::<T>(self.into_inner_usize())
    }

    /// Returns if the pointer is null.
    pub fn is_null(&self) -> bool {
        self.decompose_ptr().is_null()
    }

    /// Decomposes the marked pointer and returns a tuple with `None` if the pointer is null, or
    /// else returns a reference to the value wrapped in `Some` and the separated tag.
    ///
    /// # Safety
    ///
    /// While this method and its mutable counterpart are useful for null-safety, it is important to
    /// note that this is still an unsafe operation because the returned value could be pointing to
    /// invalid memory.
    ///
    /// Additionally, the lifetime 'a returned is arbitrarily chosen and does not necessarily
    /// reflect the actual lifetime of the data.
    pub unsafe fn decompose_ref<'a>(&self) -> (Option<&'a T>, usize) {
        let (ptr, tag) = self.decompose();
        (ptr.as_ref(), tag)
    }

    pub unsafe fn decompose_mut<'a>(&mut self) -> (Option<&'a mut T>, usize) {
        let (ptr, tag) = self.decompose();
        (ptr.as_mut(), tag)
    }

    pub unsafe fn as_ref<'a>(&self) -> Option<&'a T> {
        self.decompose_ptr().as_ref()
    }

    pub unsafe fn as_mut<'a>(&mut self) -> Option<&'a mut T> {
        self.decompose_ptr().as_mut()
    }
}

impl<T> fmt::Debug for MarkedPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.decompose();
        f.debug_struct("MarkedPtr")
            .field("ptr", &ptr)
            .field("tag", &tag)
            .finish()
    }
}

impl<T> Default for MarkedPtr<T> {
    fn default() -> Self {
        MarkedPtr::null()
    }
}

impl<T> From<*const T> for MarkedPtr<T> {
    fn from(ptr: *const T) -> Self {
        Self {
            inner: ptr as *mut _,
        }
    }
}

impl<T> From<*mut T> for MarkedPtr<T> {
    fn from(ptr: *mut T) -> Self {
        Self { inner: ptr }
    }
}

impl<'a, T> From<&'a T> for MarkedPtr<T> {
    fn from(reference: &'a T) -> Self {
        Self {
            inner: reference as *const _ as *mut _,
        }
    }
}

impl<'a, T> From<&'a mut T> for MarkedPtr<T> {
    fn from(reference: &'a mut T) -> Self {
        Self {
            inner: reference as *mut _,
        }
    }
}

impl<T> fmt::Pointer for MarkedPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.decompose_ptr(), f)
    }
}

#[cfg(test)]
mod test {
    use super::MarkedPtr;

    #[test]
    fn from() {
        let r = &1;
        let from_ref = MarkedPtr::from(r);
        let from_const = MarkedPtr::from(r as *const i32);
        let from_mut = MarkedPtr::from(r as *const _ as *mut i32);
        assert_eq!(from_ref, from_const);
        assert_eq!(from_const, from_mut);
        assert_eq!(from_mut, from_ref);
    }

    #[test]
    fn eq_ord() {
        let null: MarkedPtr<i32> = MarkedPtr::null();
        assert!(null.is_null());
        assert_eq!(null, null);

        let reference: &i32 = &1;
        let marked1 = MarkedPtr::compose(reference as *const _ as *mut i32, 1);
        let marked2 = MarkedPtr::compose(reference as *const _ as *mut i32, 2);
        assert_ne!(marked1, marked2);
        assert!(marked1 < marked2);
    }

    #[test]
    fn decompose_ref() {
        let null: MarkedPtr<i32> = MarkedPtr::null();
        assert_eq!((None, 0), unsafe { null.decompose_ref() });
        let marked = MarkedPtr::compose(&1 as *const _ as *mut i32, 0b11);
        assert_eq!((Some(&1), 0b11), unsafe { marked.decompose_ref() });
    }


}