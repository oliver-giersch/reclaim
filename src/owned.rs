use core::borrow::{Borrow, BorrowMut};
use core::fmt;
use core::marker::PhantomData;
use core::mem;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

#[cfg(feature = "global_alloc")]
use std::alloc::Global;

use crate::marked::MarkedNonNull;
use crate::{Reclaim, Record, StatelessAlloc};

#[cfg(feature = "global_alloc")]
#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct Owned<T, R: Reclaim<A>, A: StatelessAlloc = Global> {
    inner: MarkedNonNull<T>,
    _marker: PhantomData<(T, R, A)>,
}

#[cfg(not(feature = "global_alloc"))]
#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct Owned<T, R: Reclaim<A>, A: StatelessAlloc> {
    inner: MarkedNonNull<T>,
    _marker: PhantomData<(T, R, A)>,
}

unsafe impl<T: Send, R: Reclaim<A>, A: StatelessAlloc> Send for Owned<T, R, A> {}
unsafe impl<T: Sync, R: Reclaim<A>, A: StatelessAlloc> Sync for Owned<T, R, A> {}

impl<T, R: Reclaim<A>, A: StatelessAlloc> Owned<T, R, A> {
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
    pub unsafe fn from_raw(raw: *mut T) -> Self {
        Self {
            inner: MarkedNonNull::new_unchecked(raw),
            _marker: PhantomData,
        }
    }

    /// TODO: Doc...
    pub fn into_raw(owned: Self) -> *mut T {
        let ptr = owned.inner.into_inner().as_ptr();
        mem::forget(owned);
        ptr
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
}

#[cfg(feature = "global_alloc")]
impl<T, R: Reclaim<A>, A: StatelessAlloc> Owned<T, R, A> {
    fn allocate_record(owned: T) -> NonNull<T> {
        let boxed = Box::new(Record::<T, R, A>::new(owned));
        NonNull::from(&mut Box::leak(boxed).elem)
    }

    unsafe fn deallocate_record(owned: &mut T) {
        let record = Record::<T, R, A>::get_record(owned as *mut _);
        mem::drop(Box::from_raw(record));
    }
}

#[cfg(not(feature = "global_alloc"))]
impl<T, R: Reclaim<A>, A: StatelessAlloc> Owned<T, R, A> {
    fn allocate_record(owned: T) -> NonNull<T> {
        use core::alloc::Layout;
        use core::ptr;

        let mut alloc = R::allocator();
        let layout = Layout::for_value(&x);
        let size = layout.size();

        let ptr = if size == 0 {
            NonNull::dangling()
        } else {
            unsafe {
                alloc
                    .alloc(layout)
                    .expect("oom")
                    .cast()
            }
        };

        unsafe { ptr::write(ptr.as_ptr() as *mut T, owned); }
        ptr
    }

    unsafe fn deallocate_record(owned: &mut T) {
        use core::alloc::Layout;
        use core::ptr;

        let record = &mut *Record::<T, R, A>::get_record(owned as *mut _);
        ptr::drop_in_place(record as *mut _);

        let layout = Layout::for_value(record);
        if layout.size() != 0 {
            let mut alloc = R::allocator();
            alloc.dealloc(NonNull::from(record).cast(), layout);
        }
    }
}

impl<T, R: Reclaim<A>, A: StatelessAlloc> AsRef<T> for Owned<T, R, A> {
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T, R: Reclaim<A>, A: StatelessAlloc> AsMut<T> for Owned<T, R, A> {
    fn as_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<T, R: Reclaim<A>, A: StatelessAlloc> Borrow<T> for Owned<T, R, A> {
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T, R: Reclaim<A>, A: StatelessAlloc> BorrowMut<T> for Owned<T, R, A> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<T: Clone, R: Reclaim<A>, A: StatelessAlloc> Clone for Owned<T, R, A> {
    fn clone(&self) -> Self {
        let (ptr, tag) = self.inner.decompose();
        let reference = unsafe { ptr.as_ref() };
        Self::with_tag(reference.clone(), tag)
    }
}

impl<T, R: Reclaim<A>, A: StatelessAlloc> Deref for Owned<T, R, A> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.inner.decompose_ptr() }
    }
}

impl<T, R: Reclaim<A>, A: StatelessAlloc> DerefMut for Owned<T, R, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.inner.decompose_ptr() }
    }
}

impl<T, R: Reclaim<A>, A: StatelessAlloc> Drop for Owned<T, R, A> {
    fn drop(&mut self) {
        unsafe {
            let elem = self.inner.as_mut();
            Self::deallocate_record(elem);
        }
    }
}

#[cfg(not(feature = "no_std"))]
impl<T: Default, R: Reclaim<Global>> Default for Owned<T, R, Global> {
    fn default() -> Self {
        Owned::new(T::default())
    }
}

#[cfg(not(feature = "no_std"))]
impl<T, R: Reclaim<Global>> From<T> for Owned<T, R, Global> {
    fn from(owned: T) -> Self {
        Owned::new(owned)
    }
}

impl<T: fmt::Debug, R: Reclaim<A>, A: StatelessAlloc> fmt::Debug for Owned<T, R, A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (reference, tag) = unsafe { self.inner.decompose_ref() };
        f.debug_struct("Owned")
            .field("value", reference)
            .field("tag", &tag)
            .finish()
    }
}

impl<T, R: Reclaim<A>, A: StatelessAlloc> fmt::Pointer for Owned<T, R, A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}