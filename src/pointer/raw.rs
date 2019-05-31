use core::cmp::{self, PartialEq, PartialOrd};
use core::fmt;
use core::marker::PhantomData;
use core::ptr::{self, NonNull};

use typenum::{IsGreaterOrEqual, True, Unsigned};

use crate::pointer::{self, MarkedNonNull, MarkedPtr};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Copy & Clone
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, N> Clone for MarkedPtr<T, N> {
    #[inline]
    fn clone(&self) -> Self {
        Self::new(self.inner)
    }
}

impl<T, N> Copy for MarkedPtr<T, N> {}

////////////////////////////////////////////////////////////////////////////////////////////////////
// inherent (const)
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, N> MarkedPtr<T, N> {
    /// Creates an unmarked pointer.
    #[inline]
    pub const fn new(ptr: *mut T) -> Self {
        Self { inner: ptr, _marker: PhantomData }
    }

    /// Creates a new & unmarked `null` pointer.
    #[inline]
    pub const fn null() -> Self {
        Self::new(ptr::null_mut())
    }

    /// Creates a marked pointer from the numeric representation of a
    /// potentially marked pointer.
    #[inline]
    pub const fn from_usize(val: usize) -> Self {
        Self::new(val as *mut _)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// inherent
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, N: Unsigned> MarkedPtr<T, N> {
    /// The number of available mark bits for this type.
    pub const MARK_BITS: usize = N::USIZE;
    /// The bitmask for the lower markable bits.
    pub const MARK_MASK: usize = pointer::mark_mask::<T>(Self::MARK_BITS);
    /// The bitmask for the higher pointer bits.
    pub const POINTER_MASK: usize = !Self::MARK_MASK;

    /// Returns the numeric representation of the pointer with its tag.
    #[inline]
    pub fn into_usize(self) -> usize {
        self.inner as usize
    }

    /// Returns the inner pointer *as is*, meaning potential tags are not
    /// stripped.
    #[inline]
    pub fn into_ptr(self) -> *mut T {
        self.inner
    }

    /// Composes a new marked pointer from a raw unmarked pointer and a tag
    /// value.
    #[inline]
    pub fn compose(ptr: *mut T, tag: usize) -> Self {
        debug_assert_eq!(0, ptr as usize & Self::MARK_MASK, "pointer must be properly aligned");
        Self::new(pointer::compose::<_, N>(ptr, tag))
    }

    /// Converts a marked pointer with `M` potential mark bits to the **same**
    /// marked pointer with `N` potential mark bits, requires that `N >= M`.
    #[inline]
    pub fn convert<M: Unsigned>(other: MarkedPtr<T, M>) -> Self
    where
        N: IsGreaterOrEqual<M, Output = True>,
    {
        Self::new(other.inner)
    }

    /// Clears the tag of `self` and returns the same but untagged pointer.
    #[inline]
    pub fn clear_tag(self) -> Self {
        Self::new(self.decompose_ptr())
    }

    /// Clears the tag of `self` and replaces it with `tag`.
    #[inline]
    pub fn with_tag(self, tag: usize) -> Self {
        Self::compose(self.decompose_ptr(), tag)
    }

    /// Decomposes the marked pointer, returning the separated raw pointer and
    /// its tag.
    #[inline]
    pub fn decompose(self) -> (*mut T, usize) {
        pointer::decompose(self.into_usize(), Self::MARK_BITS)
    }

    /// Decomposes the marked pointer, returning only the separated raw pointer.
    #[inline]
    pub fn decompose_ptr(self) -> *mut T {
        pointer::decompose_ptr(self.into_usize(), Self::MARK_BITS)
    }

    /// Decomposes the marked pointer, returning only the separated tag.
    #[inline]
    pub fn decompose_tag(self) -> usize {
        pointer::decompose_tag::<T>(self.into_usize(), Self::MARK_BITS)
    }

    /// Decomposes the marked pointer, returning an optional reference and the
    /// separated tag.
    ///
    /// In case the pointer stripped of its tag is null, [`None`] is returned as
    /// part of the tuple. Otherwise, the reference is wrapped in a [`Some`].
    ///
    /// # Safety
    ///
    /// While this method and its mutable counterpart are useful for
    /// null-safety, it is important to note that this is still an unsafe
    /// operation because the returned value could be pointing to invalid
    /// memory.
    ///
    /// Additionally, the lifetime 'a returned is arbitrarily chosen and does
    /// not necessarily reflect the actual lifetime of the data.
    #[inline]
    pub unsafe fn decompose_ref<'a>(self) -> (Option<&'a T>, usize) {
        let (ptr, tag) = self.decompose();
        (ptr.as_ref(), tag)
    }

    /// Decomposes the marked pointer returning an optional mutable reference
    /// and the separated tag.
    ///
    /// In case the pointer stripped of its tag is null, [`None`] is returned as
    /// part of the tuple. Otherwise, the mutable reference is wrapped in a
    /// [`Some`].
    ///
    /// # Safety
    ///
    /// As with [`decompose_ref`][MarkedPtr::decompose_ref], this is unsafe
    /// because it cannot verify the validity of the returned pointer, nor can
    /// it ensure that the lifetime `'a` returned is indeed a valid lifetime for
    /// the contained data.
    #[inline]
    pub unsafe fn decompose_mut<'a>(self) -> (Option<&'a mut T>, usize) {
        let (ptr, tag) = self.decompose();
        (ptr.as_mut(), tag)
    }

    /// Decomposes the marked pointer, returning an optional reference and
    /// discarding the tag.
    ///
    /// # Safety
    ///
    /// The same caveats as with [`decompose_ref`][MarkedPtr::decompose_ref]
    /// apply for this method as well.
    #[inline]
    pub unsafe fn as_ref<'a>(self) -> Option<&'a T> {
        self.decompose_ptr().as_ref()
    }

    /// Decomposes the marked pointer, returning an optional mutable reference
    /// and discarding the tag.
    ///
    /// # Safety
    ///
    /// The same caveats as with [`decompose_mut`][MarkedPtr::decompose_mut]
    /// apply for this method as well.
    #[inline]
    pub unsafe fn as_mut<'a>(self) -> Option<&'a mut T> {
        self.decompose_ptr().as_mut()
    }

    /// Returns true if the pointer is `null` (regardless of the tag).
    #[inline]
    pub fn is_null(self) -> bool {
        self.decompose_ptr().is_null()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Default
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, N: Unsigned> Default for MarkedPtr<T, N> {
    #[inline]
    fn default() -> Self {
        Self::null()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Debug & Pointer (fmt)
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, N: Unsigned> fmt::Debug for MarkedPtr<T, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.decompose();
        f.debug_struct("MarkedPtr").field("ptr", &ptr).field("tag", &tag).finish()
    }
}

impl<T, N: Unsigned> fmt::Pointer for MarkedPtr<T, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.decompose_ptr(), f)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// From
////////////////////////////////////////////////////////////////////////////////////////////////////

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

impl<T, N: Unsigned> From<(*mut T, usize)> for MarkedPtr<T, N> {
    #[inline]
    fn from(pair: (*mut T, usize)) -> Self {
        let (ptr, tag) = pair;
        Self::compose(ptr, tag)
    }
}

impl<T, N: Unsigned> From<(*const T, usize)> for MarkedPtr<T, N> {
    #[inline]
    fn from(pair: (*const T, usize)) -> Self {
        let (ptr, tag) = pair;
        Self::compose(ptr as *mut _, tag)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// PartialEq & PartialOrd
////////////////////////////////////////////////////////////////////////////////////////////////////

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
        self.inner.eq(&other.inner.as_ptr())
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
    use core::ptr;

    use matches::assert_matches;
    use typenum::{U0, U1, U3};

    use crate::align::Aligned8;

    type UnmarkedMarkedPtr = super::MarkedPtr<Aligned8<i32>, U0>;
    type MarkedPtr1N = super::MarkedPtr<Aligned8<i32>, U1>;
    type MarkedPtr3N = super::MarkedPtr<Aligned8<i32>, U3>;

    #[test]
    fn decompose_ref() {
        let null = MarkedPtr3N::null();
        assert_eq!((None, 0), unsafe { null.decompose_ref() });

        let marked_null = MarkedPtr3N::compose(ptr::null_mut(), 0b111);
        assert_eq!((None, 0b111), unsafe { marked_null.decompose_ref() });

        let value = Aligned8(1);
        let marked = MarkedPtr3N::compose(&value as *const Aligned8<i32> as *mut _, 0b11);
        assert_eq!((Some(&value), 0b11), unsafe { marked.decompose_ref() });
    }

    #[test]
    fn decompose_mut() {
        let null = MarkedPtr3N::null();
        assert_eq!((None, 0), unsafe { null.decompose_mut() });

        let marked_null = MarkedPtr3N::compose(ptr::null_mut(), 0b111);
        assert_eq!((None, 0b111), unsafe { marked_null.decompose_mut() });

        let mut value = Aligned8(1);
        let marked = MarkedPtr3N::compose(&mut value, 0b11);
        assert_eq!((Some(&mut value), 0b11), unsafe { marked.decompose_mut() });
    }

    #[test]
    fn from_usize() {
        unsafe {
            let unmarked = UnmarkedMarkedPtr::from_usize(&Aligned8(1) as *const _ as usize);
            assert_matches!(unmarked.as_ref(), Some(&Aligned8(1)));

            let tagged = (&Aligned8(1i32) as *const _ as usize) | 0b1;
            assert_eq!(
                (Some(&Aligned8(1i32)), 0b1),
                MarkedPtr1N::from_usize(tagged).decompose_ref()
            );
        }
    }

    #[test]
    fn from() {
        let mut x = Aligned8(1);

        let from_ref = MarkedPtr1N::from(&x);
        let from_mut = MarkedPtr1N::from(&mut x);
        let from_const_ptr = MarkedPtr1N::from(&x as *const _);
        let from_mut_ptr = MarkedPtr1N::from(&mut x as *mut _);

        assert!(from_ref == from_mut && from_const_ptr == from_mut_ptr);
    }

    #[test]
    fn eq_ord() {
        let null = MarkedPtr3N::null();
        assert!(null.is_null());
        assert_eq!(null, null);

        let mut aligned = Aligned8(1);
        let marked1 = MarkedPtr3N::compose(&mut aligned, 0b01);
        let marked2 = MarkedPtr3N::compose(&mut aligned, 0b11);

        assert_ne!(marked1, marked2);
        assert!(marked1 < marked2);
    }

    #[test]
    fn convert() {
        let mut aligned = Aligned8(1);

        let marked = MarkedPtr1N::compose(&mut aligned, 0b1);
        let convert = MarkedPtr3N::convert(marked);

        assert_eq!((Some(&aligned), 0b1), unsafe { convert.decompose_ref() });
    }
}
