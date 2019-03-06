use core::alloc::{Alloc, Layout};
use core::borrow::{Borrow, BorrowMut};
use core::fmt;
use core::marker::PhantomData;
use core::mem;
use core::ops::{Deref, DerefMut};
use core::ptr::{self, NonNull};

use typenum::Unsigned;

use crate::marked::MarkedNonNull;
use crate::MarkedPtr;
use crate::{Reclaim, Record};

/// TODO: Docs...
#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct Owned<T, N: Unsigned, R: Reclaim> {
    inner: MarkedNonNull<T, N>,
    _marker: PhantomData<(T, R)>,
}

unsafe impl<T, N: Unsigned, R: Reclaim> Send for Owned<T, N, R> where T: Send {}
unsafe impl<T, N: Unsigned, R: Reclaim> Sync for Owned<T, N, R> where T: Sync {}

impl<T, N: Unsigned, R: Reclaim> Owned<T, N, R> {
    /// TODO: Doc...
    pub fn new(owned: T) -> Self {
        Self {
            inner: MarkedNonNull::from(Self::allocate_record(owned)),
            _marker: PhantomData,
        }
    }

    /// TODO: Doc...
    pub fn with_tag(owned: T, tag: usize) -> Self {
        Self {
            inner: MarkedNonNull::compose(Self::allocate_record(owned), tag),
            _marker: PhantomData,
        }
    }

    /// TODO: Doc...
    pub unsafe fn from_marked(marked: MarkedPtr<T, N>) -> Option<Self> {
        mem::transmute(marked)
    }

    /// TODO: Doc...
    pub unsafe fn from_marked_non_null(marked: MarkedNonNull<T, N>) -> Self {
        Self {
            inner: marked,
            _marker: PhantomData,
        }
    }

    /// TODO: Doc...
    pub fn decompose_ref(&self) -> (&T, usize) {
        // this is safe because is `inner` is guaranteed to be backed by a valid allocation
        unsafe { self.inner.decompose_ref() }
    }

    /// TODO: Doc...
    pub fn decompose_mut(&mut self) -> (&mut T, usize) {
        // this is safe because is `inner` is guaranteed to be backed by a valid allocation
        unsafe { self.inner.decompose_mut() }
    }

    /// TODO: Doc...
    pub fn tag(&self) -> usize {
        self.inner.decompose_tag()
    }

    /// TODO: Doc...
    pub fn header(&self) -> &R::RecordHeader {
        unsafe { Record::<T, R>::get_header(self.inner.decompose_ptr()) }
    }

    /// TODO: Doc...
    pub fn header_mut(&mut self) -> &mut R::RecordHeader {
        unsafe { Record::<T, R>::get_header_mut(self.inner.decompose_ptr()) }
    }

    pub fn set_tag(&mut self, tag: usize) {
        self.inner = MarkedNonNull::compose(self.inner.decompose_non_null(), tag);
    }

    /// TODO: Doc...
    pub fn as_marked(&self) -> MarkedPtr<T, N> {
        self.inner.into_marked()
    }

    /// TODO: Doc...
    pub fn into_marked(owned: Self) -> MarkedPtr<T, N> {
        let marked = owned.inner.into_marked();
        mem::forget(owned);
        marked
    }

    /// TODO: Doc...
    pub fn into_marked_non_null(owned: Self) -> MarkedNonNull<T, N> {
        let marked = owned.inner;
        mem::forget(owned);
        marked
    }

    /// TODO: Doc...
    pub fn leak<'a>(owned: Self) -> &'a mut T
    where
        T: 'a,
    {
        let leaked = unsafe { &mut *owned.inner.decompose_non_null().as_ptr() };
        mem::forget(owned);
        leaked
    }

    fn allocate_record(owned: T) -> NonNull<T> {
        let record = Record::<_, R>::new(owned);

        let mut alloc = R::allocator();
        let layout = Layout::for_value(&record);
        let size = layout.size();

        let ptr = if size == 0 {
            NonNull::dangling()
        } else {
            unsafe { alloc.alloc(layout).expect("oom").cast() }
        };

        unsafe {
            ptr::write(ptr.as_ptr(), record);
            NonNull::from(&ptr.as_ref().elem)
        }
    }
}

impl<T, N: Unsigned, R: Reclaim> AsRef<T> for Owned<T, N, R> {
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T, N: Unsigned, R: Reclaim> AsMut<T> for Owned<T, N, R> {
    fn as_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<T, N: Unsigned, R: Reclaim> Borrow<T> for Owned<T, N, R> {
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T, N: Unsigned, R: Reclaim> BorrowMut<T> for Owned<T, N, R> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<T, N: Unsigned, R: Reclaim> Clone for Owned<T, N, R> where T: Clone {
    fn clone(&self) -> Self {
        let (ptr, tag) = self.inner.decompose();
        let reference = unsafe { ptr.as_ref() };
        Self::with_tag(reference.clone(), tag)
    }
}

impl<T, N: Unsigned, R: Reclaim> Deref for Owned<T, N, R> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.inner.decompose_ptr() }
    }
}

impl<T, N: Unsigned, R: Reclaim> DerefMut for Owned<T, N, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.inner.decompose_ptr() }
    }
}

impl<T, N: Unsigned, R: Reclaim> Drop for Owned<T, N, R> {
    fn drop(&mut self) {
        unsafe {
            let elem = self.inner.as_mut();
            let record = &mut *Record::<T, R>::get_record(elem as *mut _);
            ptr::drop_in_place(record as *mut _);

            let layout = Layout::for_value(record);
            if layout.size() != 0 {
                let mut alloc = R::allocator();
                alloc.dealloc(NonNull::from(record).cast(), layout);
            }
        }
    }
}

impl<T, N: Unsigned, R: Reclaim> Default for Owned<T, N, R> where T: Default {
    fn default() -> Self {
        Owned::new(T::default())
    }
}

impl<T, N: Unsigned, R: Reclaim> From<T> for Owned<T, N, R> {
    fn from(owned: T) -> Self {
        Owned::new(owned)
    }
}

impl<T, N: Unsigned, R: Reclaim> fmt::Debug for Owned<T, N, R> where T: fmt::Debug {
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
    use std::alloc::Global;

    use typenum::U2;

    use crate::tests::Leaking;

    type Owned<T> = super::Owned<T, U2, Leaking<Global>>;

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
    fn from_marked() {
        let owned = Owned::new(1);
        let marked = Owned::into_marked(owned);

        let from = unsafe { Owned::from_marked(marked).unwrap() };
        assert_eq!((&1, 0), from.decompose_ref());
    }

    #[test]
    fn with_tag() {
        let owned = Owned::with_tag(1, 0b11);
        assert_eq!((Some(&1), 0b11), unsafe {
            owned.as_marked().decompose_ref()
        });
        let owned = Owned::with_tag(2, 0);
        assert_eq!((Some(&2), 0), unsafe { owned.as_marked().decompose_ref() });
    }

    #[test]
    fn header() {
        let owned = Owned::new(1);
        assert_eq!(owned.header().checksum, 0xDEADBEEF);
    }

    #[test]
    fn header_mut() {
        let mut owned = Owned::new(1);
        assert_eq!(owned.header_mut().checksum, 0xDEADBEEF);
    }

    #[test]
    fn set_tag() {
        let mut owned = Owned::with_tag(1, 0b11);
        owned.set_tag(0);
        assert_eq!((Some(&1), 0), unsafe { owned.as_marked().decompose_ref() });
        owned.set_tag(0b1);
        assert_eq!((Some(&1), 0b1), unsafe {
            owned.as_marked().decompose_ref()
        });
        owned.set_tag(0b11);
        assert_eq!((Some(&1), 0b11), unsafe {
            owned.as_marked().decompose_ref()
        });
    }
}
