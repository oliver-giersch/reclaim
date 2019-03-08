use core::fmt;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicPtr, Ordering};

use typenum::Unsigned;

use crate::marked::{self, AtomicMarkedPtr, MarkedPtr};

unsafe impl<T, N: Unsigned> Send for AtomicMarkedPtr<T, N> {}
unsafe impl<T, N: Unsigned> Sync for AtomicMarkedPtr<T, N> {}

impl<T, N: Unsigned> AtomicMarkedPtr<T, N> {
    pub const MARK_BITS: usize = N::USIZE;
    pub const MARK_MASK: usize = marked::mark_mask::<T, N>();
    pub const POINTER_MASK: usize = !Self::MARK_MASK;

    /// TODO: Doc...
    #[inline]
    pub const fn new(ptr: MarkedPtr<T, N>) -> Self {
        Self {
            inner: AtomicPtr::new(ptr.inner),
            _marker: PhantomData,
        }
    }

    /// TODO: Doc...
    #[inline]
    pub const fn null() -> Self {
        Self::new(MarkedPtr::null())
    }

    /// TODO: Doc...
    #[inline]
    pub fn into_inner(self) -> MarkedPtr<T, N> {
        MarkedPtr::new(self.inner.into_inner())
    }

    /// TODO: Doc...
    #[inline]
    pub fn load(&self, order: Ordering) -> MarkedPtr<T, N> {
        MarkedPtr::new(self.inner.load(order))
    }

    /// TODO: Doc...
    #[inline]
    pub fn store(&self, ptr: MarkedPtr<T, N>, order: Ordering) {
        self.inner.store(ptr.inner, order);
    }

    /// TODO: Doc...
    #[inline]
    pub fn swap(&self, ptr: MarkedPtr<T, N>, order: Ordering) -> MarkedPtr<T, N> {
        MarkedPtr::new(self.inner.swap(ptr.inner, order))
    }

    /// TODO: Doc...
    #[inline]
    pub fn compare_and_swap(
        &self,
        current: MarkedPtr<T, N>,
        new: MarkedPtr<T, N>,
        order: Ordering,
    ) -> MarkedPtr<T, N> {
        MarkedPtr::new(self.inner.compare_and_swap(current.inner, new.inner, order))
    }

    /// TODO: Doc...
    #[inline]
    pub fn compare_exchange(
        &self,
        current: MarkedPtr<T, N>,
        new: MarkedPtr<T, N>,
        success: Ordering,
        failure: Ordering,
    ) -> Result<MarkedPtr<T, N>, MarkedPtr<T, N>> {
        self.inner
            .compare_exchange(current.inner, new.inner, success, failure)
            .map(MarkedPtr::new)
            .map_err(MarkedPtr::new)
    }

    /// TODO: Doc...
    #[inline]
    pub fn compare_exchange_weak(
        &self,
        current: MarkedPtr<T, N>,
        new: MarkedPtr<T, N>,
        success: Ordering,
        failure: Ordering,
    ) -> Result<MarkedPtr<T, N>, MarkedPtr<T, N>> {
        self.inner
            .compare_exchange_weak(current.inner, new.inner, success, failure)
            .map(MarkedPtr::new)
            .map_err(MarkedPtr::new)
    }
}

impl<T, N: Unsigned> fmt::Debug for AtomicMarkedPtr<T, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.load(Ordering::SeqCst).decompose();
        f.debug_struct("AtomicMarkedPtr")
            .field("ptr", &ptr)
            .field("tag", &tag)
            .finish()
    }
}

impl<T, N: Unsigned> Default for AtomicMarkedPtr<T, N> {
    #[inline]
    fn default() -> Self {
        AtomicMarkedPtr::null()
    }
}

impl<T, N: Unsigned> From<*const T> for AtomicMarkedPtr<T, N> {
    #[inline]
    fn from(ptr: *const T) -> Self {
        AtomicMarkedPtr::new(MarkedPtr::from(ptr))
    }
}

impl<T, N: Unsigned> From<*mut T> for AtomicMarkedPtr<T, N> {
    #[inline]
    fn from(ptr: *mut T) -> Self {
        AtomicMarkedPtr::new(MarkedPtr::from(ptr))
    }
}

impl<T, N: Unsigned> From<MarkedPtr<T, N>> for AtomicMarkedPtr<T, N> {
    #[inline]
    fn from(ptr: MarkedPtr<T, N>) -> Self {
        AtomicMarkedPtr::new(ptr)
    }
}

impl<T, N: Unsigned> fmt::Pointer for AtomicMarkedPtr<T, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.load(Ordering::SeqCst).decompose_ptr(), f)
    }
}

#[cfg(test)]
mod tests {
    use core::sync::atomic::Ordering;

    use typenum::U3;

    use crate::align::Aligned8;

    type AtomicMarkedPtr<T> = crate::marked::AtomicMarkedPtr<T, U3>;
    type MarkedPtr<T> = crate::marked::MarkedPtr<T, U3>;

    #[test]
    fn null() {
        let ptr: AtomicMarkedPtr<usize> = AtomicMarkedPtr::null();
        assert_eq!(ptr.load(Ordering::Relaxed).into_usize(), 0);
        assert_eq!(ptr.into_inner().into_usize(), 0);
    }

    #[test]
    fn new() {
        let reference = &Aligned8::new(1usize);
        let marked = AtomicMarkedPtr::new(MarkedPtr::from(reference));
        let from = AtomicMarkedPtr::from(reference as *const _ as *mut Aligned8<usize>);
        assert_eq!(
            marked.load(Ordering::Relaxed).into_usize(),
            reference as *const _ as usize
        );
        assert_eq!(
            from.load(Ordering::Relaxed).into_usize(),
            reference as *const _ as usize
        );
    }

    #[test]
    fn store() {
        let raw = MarkedPtr::from(&Aligned8::new(1usize));
        let atomic = AtomicMarkedPtr::null();

        atomic.store(raw, Ordering::Relaxed);
        assert_eq!(atomic.load(Ordering::Relaxed), raw);
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
