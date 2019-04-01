use core::cmp::{self, PartialEq, PartialOrd};
use core::fmt;
use core::marker::PhantomData;
use core::ptr::{self, NonNull};

use typenum::{IsGreaterOrEqual, True, Unsigned};

use crate::marked::{self, MarkedNonNull, MarkedPtr};

impl<T, N: Unsigned> Clone for MarkedPtr<T, N> {
    #[inline]
    fn clone(&self) -> Self {
        Self::new(self.inner)
    }
}

impl<T, N: Unsigned> Copy for MarkedPtr<T, N> {}

impl<T, N> MarkedPtr<T, N> {
    /// Creates an unmarked pointer.
    #[inline]
    pub const fn new(ptr: *mut T) -> Self {
        Self {
            inner: ptr,
            _marker: PhantomData,
        }
    }

    /// Creates an unmarked null pointer.
    #[inline]
    pub const fn null() -> Self {
        Self::new(ptr::null_mut())
    }

    /// Creates a marked pointer from the numeric representation of a potentially marked pointer.
    #[inline]
    pub const fn from_usize(val: usize) -> Self {
        Self::new(val as *mut _)
    }
}

impl<T, N: Unsigned> MarkedPtr<T, N> {
    /// The number of available mark bits for this type.
    pub const MARK_BITS: usize = N::USIZE;
    /// The bitmask for the lower markable bits.
    pub const MARK_MASK: usize = marked::mark_mask::<T>(Self::MARK_BITS);
    /// The bitmask for the (higher) pointer bits.
    pub const POINTER_MASK: usize = !Self::MARK_MASK;

    /// Gets the numeric inner representation of the pointer with its tag.
    #[inline]
    pub fn into_usize(self) -> usize {
        self.inner as usize
    }

    /// Composes a new marked pointer from a raw pointer and a tag value.
    #[inline]
    pub fn compose(ptr: *mut T, tag: usize) -> Self {
        debug_assert_eq!(
            ptr as usize & Self::MARK_MASK,
            0,
            "pointer must be properly aligned"
        );
        Self::new(marked::compose::<_, N>(ptr, tag))
    }

    /// TODO: Doc...
    #[inline]
    pub fn convert<M: Unsigned>(other: MarkedPtr<T, M>) -> Self
    where
        N: IsGreaterOrEqual<M, Output = True>,
    {
        Self::new(other.inner)
    }

    /// Decomposes the marked pointer and returns the separated raw pointer and its tag.
    #[inline]
    pub fn decompose(self) -> (*mut T, usize) {
        marked::decompose(self.into_usize(), Self::MARK_BITS)
    }

    /// Decomposes the marked pointer and returns only the separated raw pointer.
    #[inline]
    pub fn decompose_ptr(self) -> *mut T {
        marked::decompose_ptr(self.into_usize(), Self::MARK_BITS)
    }

    /// Decomposes the marked pointer and returns only the separated tag.
    #[inline]
    pub fn decompose_tag(self) -> usize {
        marked::decompose_tag::<T>(self.into_usize(), Self::MARK_BITS)
    }

    /// Returns true if the pointer is null.
    #[inline]
    pub fn is_null(self) -> bool {
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
    #[inline]
    pub unsafe fn decompose_ref<'a>(self) -> (Option<&'a T>, usize) {
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
    #[inline]
    pub unsafe fn decompose_mut<'a>(&mut self) -> (Option<&'a mut T>, usize) {
        let (ptr, tag) = self.decompose();
        (ptr.as_mut(), tag)
    }

    /// Decomposes the marked pointer returning an optional reference.
    ///
    /// The tag is stripped and discarded.
    #[inline]
    pub unsafe fn as_ref<'a>(self) -> Option<&'a T> {
        self.decompose_ptr().as_ref()
    }

    /// Decomposes the marked pointer returning an optional mutable reference.
    ///
    /// The tag is stripped and discarded.
    #[inline]
    pub unsafe fn as_mut<'a>(self) -> Option<&'a mut T> {
        self.decompose_ptr().as_mut()
    }
}

impl<T, N: Unsigned> fmt::Debug for MarkedPtr<T, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.decompose();
        f.debug_struct("MarkedPtr")
            .field("ptr", &ptr)
            .field("tag", &tag)
            .finish()
    }
}

impl<T, N: Unsigned> Default for MarkedPtr<T, N> {
    #[inline]
    fn default() -> Self {
        MarkedPtr::null()
    }
}

impl<T, N: Unsigned> From<*const T> for MarkedPtr<T, N> {
    #[inline]
    fn from(ptr: *const T) -> Self {
        Self::new(ptr as *mut _)
    }
}

impl<T, N: Unsigned> From<*mut T> for MarkedPtr<T, N> {
    fn from(ptr: *mut T) -> Self {
        Self::new(ptr)
    }
}

impl<'a, T, N: Unsigned> From<&'a T> for MarkedPtr<T, N> {
    #[inline]
    fn from(reference: &'a T) -> Self {
        Self::new(reference as *const _ as *mut _)
    }
}

impl<'a, T, N: Unsigned> From<&'a mut T> for MarkedPtr<T, N> {
    #[inline]
    fn from(reference: &'a mut T) -> Self {
        Self::new(reference)
    }
}

impl<T, N: Unsigned> From<NonNull<T>> for MarkedPtr<T, N> {
    #[inline]
    fn from(ptr: NonNull<T>) -> Self {
        Self::new(ptr.as_ptr())
    }
}

impl<T, N: Unsigned> fmt::Pointer for MarkedPtr<T, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.decompose_ptr(), f)
    }
}

impl<T, N> PartialEq for MarkedPtr<T, N> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T, N> PartialOrd for MarkedPtr<T, N> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl<T, N> PartialEq<MarkedNonNull<T, N>> for MarkedPtr<T, N> {
    #[inline]
    fn eq(&self, other: &MarkedNonNull<T, N>) -> bool {
        self.inner == other.inner.as_ptr()
    }
}

impl<T, N> PartialOrd<MarkedNonNull<T, N>> for MarkedPtr<T, N> {
    #[inline]
    fn partial_cmp(&self, other: &MarkedNonNull<T, N>) -> Option<cmp::Ordering> {
        self.inner.partial_cmp(&other.inner.as_ptr())
    }
}

#[cfg(test)]
mod test {
    use typenum::{U0, U1, U3};

    use crate::align::Aligned8;

    use super::MarkedPtr;

    #[test]
    fn decompose_ref() {
        let null: MarkedPtr<Aligned8<i32>, U3> = MarkedPtr::null();
        assert_eq!((None, 0), unsafe { null.decompose_ref() });

        let aligned = Aligned8::new(1);
        let marked: MarkedPtr<Aligned8<i32>, U3> =
            MarkedPtr::compose(&aligned as *const _ as *mut _, 0b11);
        assert_eq!((Some(&aligned), 0b11), unsafe { marked.decompose_ref() });
    }

    #[test]
    fn decompose_mut() {
        let mut null: MarkedPtr<Aligned8<i32>, U3> = MarkedPtr::null();
        assert_eq!((None, 0), unsafe { null.decompose_mut() });

        let mut aligned = Aligned8::new(1);
        let mut ptr: MarkedPtr<Aligned8<i32>, U3> = MarkedPtr::compose(&mut aligned, 3);
        assert_eq!((Some(&mut aligned), 3), unsafe { ptr.decompose_mut() });
    }

    #[test]
    fn default() {
        let default: MarkedPtr<Aligned8<i32>, U3> = MarkedPtr::default();
        assert!(default.is_null());
        assert_eq!(default.into_usize(), 0);
    }

    #[test]
    fn from_usize() {
        assert_eq!(Some(&1), unsafe {
            MarkedPtr::<usize, U0>::from_usize(&1usize as *const _ as usize).as_ref()
        });
    }

    #[test]
    fn from() {
        let mut x = 1;

        let from_ref: MarkedPtr<usize, U1> = MarkedPtr::from(&x);
        let from_mut_ref: MarkedPtr<usize, U1> = MarkedPtr::from(&mut x);
        let from_const: MarkedPtr<usize, U1> = MarkedPtr::from(&x as *const usize);
        let from_mut: MarkedPtr<usize, U1> = MarkedPtr::from(&x as *const _ as *mut usize);

        assert!(from_ref == from_mut_ref && from_const == from_mut);
        assert!(from_ref == from_mut && from_const == from_mut_ref);
    }

    #[test]
    fn eq_ord() {
        let null: MarkedPtr<Aligned8<i32>, U3> = MarkedPtr::null();
        assert!(null.is_null());
        assert_eq!(null, null);

        let reference = &Aligned8::new(1);
        let marked1: MarkedPtr<Aligned8<i32>, U3> =
            MarkedPtr::compose(reference as *const _ as *mut _, 1);
        let marked2: MarkedPtr<Aligned8<i32>, U3> =
            MarkedPtr::compose(reference as *const _ as *mut _, 2);
        assert_ne!(marked1, marked2);
        assert!(marked1 < marked2);
    }

    #[test]
    fn convert() {
        let mut aligned = Aligned8::new(1);
        let from: MarkedPtr<Aligned8<i32>, U1> = MarkedPtr::compose(&mut aligned, 0b1);
        let convert: MarkedPtr<Aligned8<i32>, U3> = MarkedPtr::convert(from);

        assert_eq!((Some(&aligned), 0b1), unsafe { convert.decompose_ref() });
    }
}
