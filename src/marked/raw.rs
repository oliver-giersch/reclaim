use core::cmp::{self, PartialEq, PartialOrd};
use core::fmt;
use core::ptr::{self, NonNull};

use crate::marked::{self, MarkedNonNull, MarkedPtr};

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

    /// Creates an unmarked null pointer.
    pub const fn null() -> Self {
        Self { inner: ptr::null_mut() }
    }

    /// Creates an unmarked pointer.
    pub const fn new(ptr: *mut T) -> Self {
        Self { inner: ptr }
    }

    /// Creates a marked pointer from the numeric representation of a potentially marked pointer.
    pub const fn from_usize(val: usize) -> Self {
        Self { inner: val as *mut _ }
    }

    /// Gets the numeric inner representation of the pointer with its tag.
    pub fn into_usize(self) -> usize {
        self.inner as usize
    }

    /// Composes a new marked pointer from a raw pointer and a tag value.
    pub fn compose(ptr: *mut T, tag: usize) -> Self {
        Self { inner: marked::compose(ptr, tag) }
    }

    /// Decomposes the marked pointer and returns the separated raw pointer and its tag.
    pub fn decompose(&self) -> (*mut T, usize) {
        marked::decompose(self.into_usize())
    }

    /// Decomposes the marked pointer and returns only the separated raw pointer.
    pub fn decompose_ptr(&self) -> *mut T {
        marked::decompose_ptr(self.into_usize())
    }

    /// Decomposes the marked pointer and returns only the separated tag.
    pub fn decompose_tag(&self) -> usize {
        marked::decompose_tag::<T>(self.into_usize())
    }

    /// Returns true if the pointer is null.
    pub fn is_null(&self) -> bool {
        self.decompose_ptr().is_null()
    }

    /// Decomposes the marked pointer returning an optional reference and the separated tag.
    ///
    /// In case the pointer stripped of its tag is null, `None` is returned as part of the tuple.
    /// Otherwise, the reference is wrapped in a `Some`.
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

    /// Decomposes the marked pointer returning an optional mutable reference and the separated tag.
    ///
    /// In case the pointer stripped of its tag is null, `None` is returned as part of the tuple.
    /// Otherwise, the mutable reference is wrapped in a `Some`.
    ///
    /// # Safety
    ///
    /// While this method and its mutable counterpart are useful for null-safety, it is important to
    /// note that this is still an unsafe operation because the returned value could be pointing to
    /// invalid memory.
    ///
    /// Additionally, the lifetime 'a returned is arbitrarily chosen and does not necessarily
    /// reflect the actual lifetime of the data.
    pub unsafe fn decompose_mut<'a>(&mut self) -> (Option<&'a mut T>, usize) {
        let (ptr, tag) = self.decompose();
        (ptr.as_mut(), tag)
    }

    /// Decomposes the marked pointer returning an optional reference.
    ///
    /// The tag is stripped and discarded.
    pub unsafe fn as_ref<'a>(&self) -> Option<&'a T> {
        self.decompose_ptr().as_ref()
    }

    /// Decomposes the marked pointer returning an optional mutable reference.
    ///
    /// The tag is stripped and discarded.
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
        Self { inner: ptr as *mut _ }
    }
}

impl<T> From<*mut T> for MarkedPtr<T> {
    fn from(ptr: *mut T) -> Self {
        Self { inner: ptr }
    }
}

impl<'a, T> From<&'a T> for MarkedPtr<T> {
    fn from(reference: &'a T) -> Self {
        Self { inner: reference as *const _ as *mut _ }
    }
}

impl<'a, T> From<&'a mut T> for MarkedPtr<T> {
    fn from(reference: &'a mut T) -> Self {
        Self { inner: reference as *mut _ }
    }
}

impl<T> From<NonNull<T>> for MarkedPtr<T> {
    fn from(ptr: NonNull<T>) -> Self {
        Self { inner: ptr.as_ptr() }
    }
}

impl<T> fmt::Pointer for MarkedPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.decompose_ptr(), f)
    }
}

impl<T> PartialEq for MarkedPtr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T> PartialOrd for MarkedPtr<T> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl<T> PartialEq<MarkedNonNull<T>> for MarkedPtr<T> {
    fn eq(&self, other: &MarkedNonNull<T>) -> bool {
        self.inner == other.inner.as_ptr()
    }
}

impl<T> PartialOrd<MarkedNonNull<T>> for MarkedPtr<T> {
    fn partial_cmp(&self, other: &MarkedNonNull<T>) -> Option<cmp::Ordering> {
        self.inner.partial_cmp(&other.inner.as_ptr())
    }
}

#[cfg(test)]
mod test {
    use super::MarkedPtr;

    #[test]
    fn decompose_ref() {
        let null: MarkedPtr<usize> = MarkedPtr::null();
        assert_eq!((None, 0), unsafe { null.decompose_ref() });
        let marked: MarkedPtr<usize> = MarkedPtr::compose(&1usize as *const _ as *mut _, 0b11);
        assert_eq!((Some(&1), 0b11), unsafe { marked.decompose_ref() });
    }

    #[test]
    fn decompose_mut() {
        let mut null: MarkedPtr<usize> = MarkedPtr::null();
        assert_eq!((None, 0), unsafe { null.decompose_mut() });
        let mut ptr = MarkedPtr::compose(&mut 1, 3);
        assert_eq!((Some(&mut 1), 3), unsafe { ptr.decompose_mut() });
    }

    #[test]
    fn default() {
        let default = MarkedPtr::<i32>::default();
        assert!(default.is_null());
        assert_eq!(default.into_usize(), 0);
    }

    #[test]
    fn from_usize() {
        assert_eq!(Some(&1), unsafe { MarkedPtr::from_usize(&1 as *const _ as usize).as_ref() });
    }

    #[test]
    fn from() {
        let mut x = 1;

        let from_ref = MarkedPtr::from(&x);
        let from_mut_ref = MarkedPtr::from(&mut x);
        let from_const = MarkedPtr::from(&x as *const i32);
        let from_mut = MarkedPtr::from(&x as *const _ as *mut i32);

        assert!(from_ref == from_mut_ref && from_const == from_mut);
        assert!(from_ref == from_mut && from_const == from_mut_ref);
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
}