use core::fmt;
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

use crate::marked::{self, AtomicMarkedPtr, MarkedPtr};

unsafe impl<T> Send for AtomicMarkedPtr<T> {}
unsafe impl<T> Sync for AtomicMarkedPtr<T> {}

impl<T> AtomicMarkedPtr<T> {
    pub const MARK_BITS: usize = marked::mark_bits::<T>();
    pub const MARK_MASK: usize = marked::mark_mask::<T>();
    pub const POINTER_MASK: usize = !Self::MARK_MASK;

    /// TODO: Doc...
    pub const fn null() -> Self {
        Self {
            inner: AtomicPtr::new(ptr::null_mut()),
        }
    }

    /// TODO: Doc...
    pub fn new(ptr: MarkedPtr<T>) -> Self {
        Self {
            inner: AtomicPtr::new(ptr.inner),
        }
    }

    /// TODO: Doc...
    pub fn into_inner(self) -> MarkedPtr<T> {
        MarkedPtr::new(self.inner.into_inner())
    }

    /// TODO: Doc...
    pub fn load(&self, order: Ordering) -> MarkedPtr<T> {
        MarkedPtr::new(self.inner.load(order))
    }

    /// TODO: Doc...
    pub fn store(&self, ptr: MarkedPtr<T>, order: Ordering) {
        self.inner.store(ptr.inner, order);
    }

    /// TODO: Doc...
    pub fn swap(&self, ptr: MarkedPtr<T>, order: Ordering) -> MarkedPtr<T> {
        MarkedPtr::new(self.inner.swap(ptr.inner, order))
    }

    /// TODO: Doc...
    pub fn compare_and_swap(
        &self,
        current: MarkedPtr<T>,
        new: MarkedPtr<T>,
        order: Ordering,
    ) -> MarkedPtr<T> {
        MarkedPtr::new(self.inner.compare_and_swap(current.inner, new.inner, order))
    }

    /// TODO: Doc...
    pub fn compare_exchange(
        &self,
        current: MarkedPtr<T>,
        new: MarkedPtr<T>,
        success: Ordering,
        failure: Ordering,
    ) -> Result<MarkedPtr<T>, MarkedPtr<T>> {
        self.inner
            .compare_exchange(current.inner, new.inner, success, failure)
            .map(MarkedPtr::new)
            .map_err(MarkedPtr::new)
    }

    /// TODO: Doc...
    pub fn compare_exchange_weak(
        &self,
        current: MarkedPtr<T>,
        new: MarkedPtr<T>,
        success: Ordering,
        failure: Ordering,
    ) -> Result<MarkedPtr<T>, MarkedPtr<T>> {
        self.inner
            .compare_exchange_weak(current.inner, new.inner, success, failure)
            .map(MarkedPtr::new)
            .map_err(MarkedPtr::new)
    }
}

impl<T> fmt::Debug for AtomicMarkedPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.load(Ordering::SeqCst).decompose();
        f.debug_struct("AtomicMarkedPtr")
            .field("ptr", &ptr)
            .field("tag", &tag)
            .finish()
    }
}

impl<T> Default for AtomicMarkedPtr<T> {
    fn default() -> Self {
        AtomicMarkedPtr::null()
    }
}

impl<T> From<*const T> for AtomicMarkedPtr<T> {
    fn from(ptr: *const T) -> Self {
        AtomicMarkedPtr::new(MarkedPtr::from(ptr))
    }
}

impl<T> From<*mut T> for AtomicMarkedPtr<T> {
    fn from(ptr: *mut T) -> Self {
        AtomicMarkedPtr::new(MarkedPtr::from(ptr))
    }
}

impl<T> From<MarkedPtr<T>> for AtomicMarkedPtr<T> {
    fn from(ptr: MarkedPtr<T>) -> Self {
        AtomicMarkedPtr::new(ptr)
    }
}

impl<T> fmt::Pointer for AtomicMarkedPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.load(Ordering::SeqCst).decompose_ptr(), f)
    }
}

#[cfg(test)]
mod test {
    use core::sync::atomic::Ordering;

    use crate::marked::{AtomicMarkedPtr, MarkedPtr};

    #[test]
    fn null() {
        let ptr: AtomicMarkedPtr<i32> = AtomicMarkedPtr::null();
        assert_eq!(ptr.load(Ordering::Relaxed).into_usize(), 0);
        assert_eq!(ptr.into_inner().into_usize(), 0);
    }

    #[test]
    fn new() {
        let reference = &1;
        let marked = AtomicMarkedPtr::new(MarkedPtr::from(reference));
        let from = AtomicMarkedPtr::from(reference as *const _ as *mut i32);
        assert_eq!(marked.load(Ordering::Relaxed).into_usize(), reference as *const _ as usize);
        assert_eq!(from.load(Ordering::Relaxed).into_usize(), reference as *const _ as usize);
    }

    #[test]
    fn store() {
        let raw: MarkedPtr<i32> = MarkedPtr::from(&1);
        let atomic = AtomicMarkedPtr::null();
        atomic.store(raw, Ordering::Relaxed);
        let load = atomic.load(Ordering::Relaxed);
        assert_eq!(load, raw);
    }

    #[test]
    fn swap() {
        let reference: &i32 = &1;
        let atomic: AtomicMarkedPtr<i32> = AtomicMarkedPtr::from(reference as *const _);
        let swap = atomic.swap(MarkedPtr::null(), Ordering::Relaxed);
        assert_eq!(swap.into_usize(), reference as *const _ as usize);
        assert_eq!(atomic.load(Ordering::Relaxed).into_usize(), 0);
    }
}
