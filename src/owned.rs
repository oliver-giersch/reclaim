#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

use core::borrow::{Borrow, BorrowMut};
use core::fmt;
use core::marker::PhantomData;
use core::mem;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

use typenum::Unsigned;

use crate::pointer::{Internal, Marked, MarkedNonNull, MarkedPointer, MarkedPtr, NonNullable};
use crate::{LocalReclaim, Owned, Record, Shared};

impl<T: Clone, R: LocalReclaim, N: Unsigned> Clone for Owned<T, R, N> {
    #[inline]
    fn clone(&self) -> Self {
        let (reference, tag) = unsafe { self.inner.decompose_ref() };
        Self::compose(reference.clone(), tag)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl Send, Sync
////////////////////////////////////////////////////////////////////////////////////////////////////

unsafe impl<T, R: LocalReclaim, N: Unsigned> Send for Owned<T, R, N> where T: Send {}
unsafe impl<T, R: LocalReclaim, N: Unsigned> Sync for Owned<T, R, N> where T: Sync {}

////////////////////////////////////////////////////////////////////////////////////////////////////
// MarkedPointer
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> MarkedPointer for Owned<T, R, N> {
    type Item = T;
    type MarkBits = N;
    const MARK_BITS: usize = N::USIZE;

    #[inline]
    fn as_marked_ptr(&self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        self.inner.into_marked_ptr()
    }

    #[inline]
    fn decompose_tag(&self) -> usize {
        self.inner.decompose_tag()
    }

    #[inline]
    fn clear_tag(self) -> Self {
        self.with_tag(0)
    }

    #[inline]
    fn into_marked_ptr(self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        let marked = self.inner.into_marked_ptr();
        mem::forget(self);
        marked
    }

    #[inline]
    unsafe fn from_marked_ptr(marked: MarkedPtr<Self::Item, Self::MarkBits>) -> Self {
        debug_assert!(!marked.is_null());
        Self { inner: MarkedNonNull::new_unchecked(marked), _marker: PhantomData }
    }

    #[inline]
    unsafe fn from_marked_non_null(marked: MarkedNonNull<Self::Item, Self::MarkBits>) -> Self {
        Self { inner: marked, _marker: PhantomData }
    }
}

impl<T, R: LocalReclaim, N: Unsigned> MarkedPointer for Option<Owned<T, R, N>> {
    impl_trait_option!(Owned);
}

impl<T, R: LocalReclaim, N: Unsigned> MarkedPointer for Marked<Owned<T, R, N>> {
    impl_trait_marked!(Owned);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// inherent
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> Owned<T, R, N> {
    /// Allocates memory for a [`Record<T>`](Record) on the heap and then
    /// places a record with a default header and `owned` into it.
    ///
    /// This does only allocate memory if at least one of
    /// [`RecordHeader`][header] or `T` are not zero-sized.
    /// If the [`RecordHeader`][header] is a ZST, this behaves
    /// identically to `Box::new`.
    ///
    /// [header]: crate::LocalReclaim::RecordHeader
    #[inline]
    pub fn new(owned: T) -> Self {
        Self { inner: MarkedNonNull::from(Self::alloc_record(owned)), _marker: PhantomData }
    }

    /// Creates a new `Owned` like [`new`](Owned::new) but composes the
    /// returned pointer with an initial `tag` value.
    #[inline]
    pub fn compose(owned: T, tag: usize) -> Self {
        Self { inner: MarkedNonNull::compose(Self::alloc_record(owned), tag), _marker: PhantomData }
    }

    impl_inherent!();

    /// Decomposes the internal marked pointer, returning a reference and the
    /// separated tag.
    #[inline]
    pub fn decompose_ref(&self) -> (&T, usize) {
        // this is safe because is `inner` is guaranteed to be backed by a valid allocation
        unsafe { self.inner.decompose_ref() }
    }

    /// Decomposes the internal marked pointer, returning a mutable reference
    /// and the separated tag.
    #[inline]
    pub fn decompose_mut(&mut self) -> (&mut T, usize) {
        // this is safe because is `inner` is guaranteed to be backed by a valid allocation
        unsafe { self.inner.decompose_mut() }
    }

    /// Returns a reference to the header type that is automatically
    /// allocated alongside every new record.
    #[inline]
    pub fn header(&self) -> &R::RecordHeader {
        unsafe { Record::<T, R>::get_header(self.inner.decompose_non_null()) }
    }

    /// Returns a mutable reference to the header type that is automatically
    /// allocated alongside every new record.
    #[inline]
    pub fn header_mut(&mut self) -> &mut R::RecordHeader {
        unsafe { Record::<T, R>::get_header_mut(self.inner.decompose_non_null()) }
    }

    /// Consumes and leaks the `Owned`, returning a mutable reference
    /// `&'a mut T` and the decomposed tag.
    /// Note that the type `T` must outlive the chosen lifetime `'a`.
    /// If the type has only static references, or none at all, then this may
    /// chosen to be `'static`.
    #[inline]
    pub fn leak<'a>(owned: Self) -> (&'a mut T, usize)
    where
        T: 'a,
    {
        let (ptr, tag) = owned.inner.decompose();
        mem::forget(owned);
        unsafe { (&mut *ptr.as_ptr(), tag) }
    }

    /// Leaks the `owned` value and turns it into a "protected" [`Shared`][shared]
    /// value with arbitrary lifetime `'a`.
    ///
    /// Note, that the protection of the [`Shared`][shared] value in this case
    /// stems from the fact, that the given `owned` could not have previously
    /// been part of a concurrent data structure (barring unsafe construction).
    /// This rules out concurrent reclamation by other threads.
    ///
    /// # Safety
    ///
    /// Once a leaked [`Shared`][shared] has been successfully inserted into a
    /// concurrent data structure, it must not be accessed any more, if there is
    /// the possibility for concurrent reclamation of the record.
    ///
    /// [shared]: crate::Shared
    #[inline]
    pub unsafe fn leak_shared<'a>(owned: Self) -> Shared<'a, T, R, N> {
        let inner = owned.inner;
        mem::forget(owned);

        Shared { inner, _marker: PhantomData }
    }

    /// Allocates a records wrapping `owned` and returns the pointer to the
    /// wrapped value.
    #[inline]
    fn alloc_record(owned: T) -> NonNull<T> {
        let record = Box::leak(Box::new(Record::<_, R>::new(owned)));
        NonNull::from(&record.elem)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// AsRef & AsMut
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> AsRef<T> for Owned<T, R, N> {
    #[inline]
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T, R: LocalReclaim, N: Unsigned> AsMut<T> for Owned<T, R, N> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        &mut **self
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Borrow & BorrowMut
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> Borrow<T> for Owned<T, R, N> {
    #[inline]
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T, R: LocalReclaim, N: Unsigned> BorrowMut<T> for Owned<T, R, N> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        &mut **self
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Default
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T: Default, R: LocalReclaim, N: Unsigned> Default for Owned<T, R, N> {
    #[inline]
    fn default() -> Self {
        Owned::new(T::default())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Deref & DerefMut
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> Deref for Owned<T, R, N> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.inner.as_ref() }
    }
}

impl<T, R: LocalReclaim, N: Unsigned> DerefMut for Owned<T, R, N> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.inner.as_mut() }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Drop
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> Drop for Owned<T, R, N> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let elem = self.inner.decompose_non_null();
            let record = Record::<_, R>::get_record(elem);

            mem::drop(Box::from_raw(record.as_ptr()));
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// From
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> From<T> for Owned<T, R, N> {
    #[inline]
    fn from(owned: T) -> Self {
        Owned::new(owned)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Debug & Pointer
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> fmt::Debug for Owned<T, R, N>
where
    T: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (reference, tag) = unsafe { self.inner.decompose_ref() };
        f.debug_struct("Owned").field("value", reference).field("tag", &tag).finish()
    }
}

impl<T, R: LocalReclaim, N: Unsigned> fmt::Pointer for Owned<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Internal
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> Internal for Owned<T, R, N> {}
impl<T, R: LocalReclaim, N: Unsigned> Internal for Option<Owned<T, R, N>> {}
impl<T, R: LocalReclaim, N: Unsigned> Internal for Marked<Owned<T, R, N>> {}

////////////////////////////////////////////////////////////////////////////////////////////////////
// NonNullable
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: LocalReclaim, N: Unsigned> NonNullable for Owned<T, R, N> {}

#[cfg(test)]
mod test {
    use typenum::U2;

    use crate::leak::Leaking;
    use crate::pointer::MarkedPointer;

    type Owned<T> = super::Owned<T, Leaking, U2>;

    #[test]
    fn new() {
        let o1 = Owned::new(1);
        let o2 = Owned::new(2);
        let o3 = Owned::new(3);

        assert_eq!(1, *o1);
        assert_eq!(2, *o2);
        assert_eq!(3, *o3);
    }

    #[test]
    fn try_from_marked() {
        let owned = Owned::new(1);
        let marked = Owned::into_marked_ptr(owned);

        let from = unsafe { Owned::try_from_marked(marked).unwrap_ptr() };
        assert_eq!((&1, 0), from.decompose_ref());
    }

    #[test]
    fn compose() {
        let owned = Owned::compose(1, 0b11);
        assert_eq!((Some(&1), 0b11), unsafe { owned.into_marked_ptr().decompose_ref() });
        let owned = Owned::compose(2, 0);
        assert_eq!((Some(&2), 0), unsafe { owned.into_marked_ptr().decompose_ref() });
    }

    #[test]
    fn header() {
        let owned = Owned::new(1);
        assert_eq!(owned.header().checksum, 0xDEAD_BEEF);
    }

    #[test]
    fn header_mut() {
        let mut owned = Owned::new(1);
        assert_eq!(owned.header_mut().checksum, 0xDEAD_BEEF);
    }
}
