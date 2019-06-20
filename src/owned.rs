//! Inherent implementation and trait implementations for the [`Owned`] type.

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

use core::borrow::{Borrow, BorrowMut};
use core::fmt;
use core::marker::PhantomData;
use core::mem;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

use typenum::Unsigned;

use crate::internal::Internal;
use crate::pointer::{Marked, MarkedNonNull, MarkedPointer, NonNullable};
use crate::{Owned, Reclaim, Record, Shared, Unprotected};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Clone
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T: Clone, R: Reclaim, N: Unsigned> Clone for Owned<T, R, N> {
    #[inline]
    fn clone(&self) -> Self {
        let (reference, tag) = unsafe { self.inner.decompose_ref() };
        Self::with_tag(reference.clone(), tag)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Send & Sync
////////////////////////////////////////////////////////////////////////////////////////////////////

unsafe impl<T, R: Reclaim, N: Unsigned> Send for Owned<T, R, N> where T: Send {}
unsafe impl<T, R: Reclaim, N: Unsigned> Sync for Owned<T, R, N> where T: Sync {}

////////////////////////////////////////////////////////////////////////////////////////////////////
// MarkedPointer
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: Reclaim, N: Unsigned> MarkedPointer for Owned<T, R, N> {
    impl_trait!(owned);
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// inherent
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: Reclaim, N: Unsigned> Owned<T, R, N> {
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
    ///
    /// # Example
    ///
    /// The primary use case for this is to pre-mark newly allocated values.
    ///
    /// ```
    /// use core::sync::atomic::Ordering;
    ///
    /// use reclaim::typenum::U1;
    /// use reclaim::Shared;
    ///
    /// type Atomic<T> = reclaim::leak::Atomic<T, U1>;
    /// type Owned<T> = reclaim::leak::Owned<T, U1>;
    ///
    /// let atomic = Atomic::null();
    /// let owned = Owned::with_tag("string", 0b1);
    ///
    /// atomic.store(owned, Ordering::Relaxed);
    /// let shared = atomic.load_shared(Ordering::Relaxed);
    ///
    /// assert_eq!((&"string", 0b1), Shared::decompose_ref(shared.unwrap()));
    /// ```
    #[inline]
    pub fn with_tag(owned: T, tag: usize) -> Self {
        Self { inner: MarkedNonNull::compose(Self::alloc_record(owned), tag), _marker: PhantomData }
    }

    impl_inherent!(owned);

    /// Decomposes the internal marked pointer, returning a reference and the
    /// separated tag.
    ///
    /// # Example
    ///
    /// ```
    /// use core::sync::atomic::Ordering::Relaxed;
    ///
    /// use reclaim::typenum::U1;
    /// use reclaim::leak::Owned;
    ///
    /// type Atomic<T> = reclaim::leak::Atomic<T, U1>;
    ///
    /// let mut atomic = Atomic::from(Owned::with_tag("string", 0b1));
    /// // ... potential operations by other threads ...
    /// let owned = atomic.take(); // after all threads have joined
    ///
    /// assert_eq!((&"string", 0b1), Owned::decompose_ref(owned.as_ref().unwrap()));
    /// ```
    #[inline]
    pub fn decompose_ref(owned: &Self) -> (&T, usize) {
        // this is safe because is `inner` is guaranteed to be backed by a valid allocation
        unsafe { owned.inner.decompose_ref() }
    }

    /// Decomposes the internal marked pointer, returning a mutable reference
    /// and the separated tag.
    #[inline]
    pub fn decompose_mut(owned: &mut Self) -> (&mut T, usize) {
        // this is safe because is `inner` is guaranteed to be backed by a valid allocation
        unsafe { owned.inner.decompose_mut() }
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

    /// Leaks the `owned` value and turns it into an [`Unprotected`] value,
    /// which has copy semantics, but can no longer be safely dereferenced.
    ///
    /// # Example
    ///
    /// ```
    /// use core::sync::atomic::Ordering::Relaxed;
    ///
    /// use reclaim::typenum::U0;
    /// use reclaim::{Owned, Shared};
    ///
    /// type Atomic<T> = reclaim::leak::Atomic<T, U0>;
    ///
    /// let atomic = Atomic::null();
    ///
    /// let unprotected = Owned::leak_unprotected(Owned::new("string"));
    ///
    /// loop {
    ///     // `unprotected` is simply copied in every loop iteration
    ///     if atomic.compare_exchange_weak(Shared::none(), unprotected, Relaxed, Relaxed).is_ok() {
    ///         break;
    ///     }
    /// }
    ///
    /// # assert_eq!(&"string", &*atomic.load_shared(Relaxed).unwrap())
    /// ```
    #[inline]
    pub fn leak_unprotected(owned: Self) -> Unprotected<T, R, N> {
        let inner = owned.inner;
        mem::forget(owned);
        Unprotected { inner, _marker: PhantomData }
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
    ///
    /// # Example
    ///
    /// The use case for this method is similar to [`leak_unprotected`][Owned::leak_unprotected]
    /// but the leaked value can be safely dereferenced **before** being
    /// inserted into a shared data structure.
    ///
    /// ```
    /// use core::sync::atomic::Ordering::Relaxed;
    ///
    /// use reclaim::typenum::U0;
    /// use reclaim::{Owned, Shared};
    ///
    /// type Atomic<T> = reclaim::leak::Atomic<T, U0>;
    ///
    /// let atomic = Atomic::null();
    ///
    /// let shared = unsafe {
    ///     Owned::leak_shared(Owned::new("string"))
    /// };
    ///
    /// assert_eq!(&"string", &*shared);
    ///
    /// loop {
    ///     // `shared` is simply copied in every loop iteration
    ///     if atomic.compare_exchange_weak(Shared::none(), shared, Relaxed, Relaxed).is_ok() {
    ///         // if (non-leaking) reclamation is going on, `shared` must not be accessed
    ///         // anymore after successful insertion!
    ///         break;
    ///     }
    /// }
    ///
    /// # assert_eq!(&"string", &*atomic.load_shared(Relaxed).unwrap())
    /// ```
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

impl<T, R: Reclaim, N: Unsigned> AsRef<T> for Owned<T, R, N> {
    #[inline]
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T, R: Reclaim, N: Unsigned> AsMut<T> for Owned<T, R, N> {
    #[inline]
    fn as_mut(&mut self) -> &mut T {
        &mut **self
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Borrow & BorrowMut
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: Reclaim, N: Unsigned> Borrow<T> for Owned<T, R, N> {
    #[inline]
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T, R: Reclaim, N: Unsigned> BorrowMut<T> for Owned<T, R, N> {
    #[inline]
    fn borrow_mut(&mut self) -> &mut T {
        &mut **self
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Default
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T: Default, R: Reclaim, N: Unsigned> Default for Owned<T, R, N> {
    #[inline]
    fn default() -> Self {
        Owned::new(T::default())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Deref & DerefMut
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: Reclaim, N: Unsigned> Deref for Owned<T, R, N> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { self.inner.as_ref() }
    }
}

impl<T, R: Reclaim, N: Unsigned> DerefMut for Owned<T, R, N> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.inner.as_mut() }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Drop
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: Reclaim, N: Unsigned> Drop for Owned<T, R, N> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            let record = Record::<_, R>::from_raw(self.inner.decompose_ptr());
            mem::drop(Box::from_raw(record.as_ptr()));
        }
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// From
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: Reclaim, N: Unsigned> From<T> for Owned<T, R, N> {
    #[inline]
    fn from(owned: T) -> Self {
        Owned::new(owned)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Debug & Pointer
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: Reclaim, N: Unsigned> fmt::Debug for Owned<T, R, N>
where
    T: fmt::Debug,
{
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (reference, tag) = unsafe { self.inner.decompose_ref() };
        f.debug_struct("Owned").field("value", reference).field("tag", &tag).finish()
    }
}

impl<T, R: Reclaim, N: Unsigned> fmt::Pointer for Owned<T, R, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// NonNullable
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: Reclaim, N: Unsigned> NonNullable for Owned<T, R, N> {
    type Item = T;
    type MarkBits = N;

    #[inline]
    fn into_marked_non_null(self) -> MarkedNonNull<T, N> {
        let inner = self.inner;
        mem::forget(self);
        inner
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Internal
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, R: Reclaim, N: Unsigned> Internal for Owned<T, R, N> {}

#[cfg(test)]
mod test {
    use typenum::U2;

    use crate::leak::Leaking;
    use crate::pointer::MarkedPointer;

    type Owned<T> = crate::Owned<T, Leaking, U2>;
    type Record<T> = crate::Record<T, Leaking>;

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
    fn from_marked_ptr() {
        let owned = Owned::new(1);
        let marked = Owned::into_marked_ptr(owned);

        let from = unsafe { Owned::from_marked_ptr(marked) };
        assert_eq!((&1, 0), Owned::decompose_ref(&from));
    }

    #[test]
    fn compose() {
        let owned = Owned::with_tag(1, 0b11);
        assert_eq!((Some(&1), 0b11), unsafe { Owned::into_marked_ptr(owned).decompose_ref() });
        let owned = Owned::with_tag(2, 0);
        assert_eq!((Some(&2), 0), unsafe { Owned::into_marked_ptr(owned).decompose_ref() });
    }

    #[test]
    fn header() {
        let owned = Owned::new(1);
        let header = unsafe { Record::header_from_raw_non_null(owned.inner.decompose_non_null()) };

        assert_eq!(header.checksum, 0xDEAD_BEEF);
    }
}
