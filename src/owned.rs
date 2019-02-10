use core::alloc::Alloc;
use core::borrow::{Borrow, BorrowMut};
use core::fmt;
use core::marker::PhantomData;
use core::mem;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;

#[cfg(not(feature = "no_std"))]
use std::alloc::Global;

use crate::marked::MarkedNonNull;
use crate::{Reclaim, Record};

#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct Owned<T, R: Reclaim<A>, A: Alloc> {
    inner: MarkedNonNull<T>,
    _marker: PhantomData<(R, A)>,
}

unsafe impl<T: Send, R: Reclaim<A>, A: Alloc> Send for Owned<T, R, A> {}
unsafe impl<T: Sync, R: Reclaim<A>, A: Alloc> Sync for Owned<T, R, A> {}

#[cfg(not(feature = "no_std"))]
impl<T, R: Reclaim<Global>> Owned<T, R, Global> {
    /// TODO: Doc...
    pub fn new(owned: T) -> Self {
        let boxed = Box::new(Record::<T, R, Global>::new(owned));
        let raw = &mut Box::leak(boxed).elem;

        unsafe { Owned::from_raw(raw) }
    }
}

impl<T, R: Reclaim<A>, A: Alloc> Owned<T, R, A> {
    /// TODO: Doc...
    pub fn new_in(owned: T, a: A) -> Self {
        unimplemented!()
    }

    /// TODO: Doc...
    pub fn with_tag(owned: T, tag: usize) -> Self {
        let leaked = &mut Box::leak(Box::new(Record::<T, R, A>::new(owned))).elem;
        Self {
            inner: MarkedNonNull::compose(NonNull::from(leaked), tag),
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

impl<T, R: Reclaim<A>, A: Alloc> AsRef<T> for Owned<T, R, A> {
    fn as_ref(&self) -> &T {
        &**self
    }
}

impl<T, R: Reclaim<A>, A: Alloc> AsMut<T> for Owned<T, R, A> {
    fn as_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<T, R: Reclaim<A>, A: Alloc> Borrow<T> for Owned<T, R, A> {
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T, R: Reclaim<A>, A: Alloc> BorrowMut<T> for Owned<T, R, A> {
    fn borrow_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<T: Clone, R: Reclaim<A>, A: Alloc> Clone for Owned<T, R, A> {
    fn clone(&self) -> Self {
        let (ptr, tag) = self.inner.decompose();
        let reference = unsafe { ptr.as_ref() };
        Self::with_tag(reference.clone(), tag)
    }
}

impl<T, R: Reclaim<A>, A: Alloc> Deref for Owned<T, R, A> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.inner.decompose_ptr() }
    }
}

impl<T, R: Reclaim<A>, A: Alloc> DerefMut for Owned<T, R, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.inner.decompose_ptr() }
    }
}

impl<T, R: Reclaim<A>, A: Alloc> Drop for Owned<T, R, A> {
    fn drop(&mut self) {
        let elem = self.inner.decompose_ptr();
        unsafe {
            let record = Record::<T, R, A>::get_record(elem);
            mem::drop(Box::from_raw(record));
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

impl<T: fmt::Debug, R: Reclaim<A>, A: Alloc> fmt::Debug for Owned<T, R, A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (reference, tag) = unsafe { self.inner.decompose_ref() };
        f.debug_struct("Owned")
            .field("value", reference)
            .field("tag", &tag)
            .finish()
    }
}

impl<T, R: Reclaim<A>, A: Alloc> fmt::Pointer for Owned<T, R, A> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}
