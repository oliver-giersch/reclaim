#[cfg(not(feature = "with_std"))]
use alloc::boxed::Box;

use core::borrow::{Borrow, BorrowMut};
use core::fmt;
use core::marker::PhantomData;
use core::mem;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

use typenum::Unsigned;

use crate::marked::{MarkedNonNull, MarkedPtr};
use crate::pointer::{Internal, MarkedPointer};
use crate::{Owned, Reclaim, Record, Shared};

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl Internal, Clone, Copy
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, N: Unsigned, R: Reclaim> Internal for Owned<T, N, R> {}
impl<T, N: Unsigned, R: Reclaim> Internal for Option<Owned<T, N, R>> {}

impl<T: Clone, N: Unsigned, R: Reclaim> Clone for Owned<T, N, R> {
    #[inline]
    fn clone(&self) -> Self {
        let (reference, tag) = unsafe { self.inner.decompose_ref() };
        Self::compose(reference.clone(), tag)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl Send, Sync
////////////////////////////////////////////////////////////////////////////////////////////////////

unsafe impl<T, N: Unsigned, R: Reclaim> Send for Owned<T, N, R> where T: Send {}
unsafe impl<T, N: Unsigned, R: Reclaim> Sync for Owned<T, N, R> where T: Sync {}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl MarkedPointer
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, N: Unsigned, R: Reclaim> MarkedPointer for Owned<T, N, R> {
    type Item = T;
    type MarkBits = N;

    fn as_marked(&self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        self.inner.into_marked()
    }

    fn into_marked(self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        let marked = self.inner.into_marked();
        mem::forget(self);
        marked
    }

    unsafe fn from_marked(marked: MarkedPtr<Self::Item, Self::MarkBits>) -> Self {
        debug_assert!(!marked.is_null());
        Self {
            inner: MarkedNonNull::new_unchecked(marked),
            _marker: PhantomData,
        }
    }

    unsafe fn from_marked_non_null(marked: MarkedNonNull<Self::Item, Self::MarkBits>) -> Self {
        Self {
            inner: marked,
            _marker: PhantomData,
        }
    }
}

impl<T, N: Unsigned, R: Reclaim> MarkedPointer for Option<Owned<T, N, R>> {
    impl_marked_pointer_option!();
}

impl<T, N: Unsigned, R: Reclaim> Owned<T, N, R> {
    /// Allocates memory for a [`Record<T>`](Record) on the heap and then
    /// places a record with a default header and `owned` into it.
    ///
    /// This does only allocate memory if at least one of
    /// [`RecordHeader`][header] or `T` are not zero-sized.
    /// If the [`RecordHeader`][header] is a ZST, this behaves
    /// identically to [`Box::new`](std::boxed::Box::new)
    ///
    /// [header]: crate::Reclaim::RecordHeader
    #[inline]
    pub fn new(owned: T) -> Self {
        Self {
            inner: MarkedNonNull::from(Self::alloc_record(owned)),
            _marker: PhantomData,
        }
    }

    /// Creates a new `Owned` like [`new`](Owned::new) but composes the
    /// returned pointer with an initial `tag` value.
    #[inline]
    pub fn compose(owned: T, tag: usize) -> Self {
        Self {
            inner: MarkedNonNull::compose(Self::alloc_record(owned), tag),
            _marker: PhantomData,
        }
    }

    impl_inherent!();

    /// TODO: Doc...
    #[inline]
    pub fn decompose_ref(&self) -> (&T, usize) {
        // this is safe because is `inner` is guaranteed to be backed by a valid allocation
        unsafe { self.inner.decompose_ref() }
    }

    /// TODO: Doc...
    #[inline]
    pub fn decompose_mut(&mut self) -> (&mut T, usize) {
        // this is safe because is `inner` is guaranteed to be backed by a valid allocation
        unsafe { self.inner.decompose_mut() }
    }

    /// TODO: Doc...
    #[inline]
    pub fn header(&self) -> &R::RecordHeader {
        unsafe { Record::<T, R>::get_header(self.inner.decompose_non_null()) }
    }

    /// TODO: Doc...
    #[inline]
    pub fn header_mut(&mut self) -> &mut R::RecordHeader {
        unsafe { Record::<T, R>::get_header_mut(self.inner.decompose_non_null()) }
    }

    /// TODO: Doc...
    #[inline]
    pub fn leak<'a>(owned: Self) -> &'a mut T
    where
        T: 'a,
    {
        let inner = owned.inner;
        mem::forget(owned);
        unsafe { inner.as_mut() }
    }

    /// Leaks the `owned` value and turns it into a "protected" [`Shared`][shared]
    /// value with arbitrary lifetime `'a`.
    ///
    /// Note, that the protection in this case stems from the fact, the given `owned` can
    /// not have been part of a concurrent data structure (barring unsafe construction) and
    /// can not possibly be reclaimed by another thread.
    /// Once the resulting [`Shared`][shared] has been successfully inserted into a data
    /// structure, this protection ceases to hold and calling [`deref`][deref] is no longer
    /// safe to call.
    ///
    /// [shared]: crate::Shared
    /// [deref]: crate::Shared::deref
    #[inline]
    pub fn leak_shared<'a>(owned: Self) -> Shared<'a, T, N, R> {
        let inner = owned.inner;
        mem::forget(owned);

        Shared {
            inner,
            _marker: PhantomData,
        }
    }

    /// Allocates a records wrapping `owned` and returns the pointer to the wrapped value.
    #[inline]
    fn alloc_record(owned: T) -> NonNull<T> {
        let record = Box::leak(Box::new(Record::<_, R>::new(owned)));
        NonNull::from(&record.elem)
    }
}

impl<T, N: Unsigned, R: Reclaim> AsRef<T> for Owned<T, N, R> {
    #[inline]
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T, N: Unsigned, R: Reclaim> AsMut<T> for Owned<T, N, R> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<T, N: Unsigned, R: Reclaim> Borrow<T> for Owned<T, N, R> {
    #[inline]
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T, N: Unsigned, R: Reclaim> BorrowMut<T> for Owned<T, N, R> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<T, N: Unsigned, R: Reclaim> Deref for Owned<T, N, R> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.inner.as_ref() }
    }
}

impl<T, N: Unsigned, R: Reclaim> DerefMut for Owned<T, N, R> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.inner.as_mut() }
    }
}

impl<T, N: Unsigned, R: Reclaim> Drop for Owned<T, N, R> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let elem = self.inner.decompose_non_null();
            let record = Record::<_, R>::get_record(elem);

            mem::drop(Box::from_raw(record.as_ptr()));
        }
    }
}

impl<T: Default, N: Unsigned, R: Reclaim> Default for Owned<T, N, R> {
    #[inline]
    fn default() -> Self {
        Owned::new(T::default())
    }
}

impl<T, N: Unsigned, R: Reclaim> From<T> for Owned<T, N, R> {
    #[inline]
    fn from(owned: T) -> Self {
        Owned::new(owned)
    }
}

impl<T, N: Unsigned, R: Reclaim> fmt::Debug for Owned<T, N, R>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (reference, tag) = unsafe { self.inner.decompose_ref() };
        f.debug_struct("Owned")
            .field("value", reference)
            .field("tag", &tag)
            .finish()
    }
}

impl<T, N: Unsigned, R: Reclaim> fmt::Pointer for Owned<T, N, R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}

#[cfg(test)]
mod test {
    use typenum::U2;

    use crate::leak::Leaking;
    use crate::prelude::*;

    type Owned<T> = super::Owned<T, U2, Leaking>;

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
        let marked = Owned::into_marked(owned);

        let from = unsafe { Owned::try_from_marked(marked).unwrap() };
        assert_eq!((&1, 0), from.decompose_ref());
    }

    #[test]
    fn compose() {
        let owned = Owned::compose(1, 0b11);
        assert_eq!((Some(&1), 0b11), unsafe {
            owned.as_marked().decompose_ref()
        });
        let owned = Owned::compose(2, 0);
        assert_eq!((Some(&2), 0), unsafe { owned.as_marked().decompose_ref() });
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
