use core::fmt;
use core::marker::PhantomData;
use core::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

use typenum::Unsigned;

use crate::pointer::{self, AtomicMarkedPtr, MarkedPtr};

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl inherent (const)
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, N> AtomicMarkedPtr<T, N> {
    /// Creates a new `AtomicMarkedPtr`.
    #[inline]
    pub const fn new(ptr: MarkedPtr<T, N>) -> Self {
        Self { inner: AtomicPtr::new(ptr.inner), _marker: PhantomData }
    }

    /// Creates a new & unmarked `null` pointer.
    #[inline]
    pub const fn null() -> Self {
        Self::new(MarkedPtr::null())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl inherent
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, N: Unsigned> AtomicMarkedPtr<T, N> {
    /// The number of available mark bits for this type.
    pub const MARK_BITS: usize = N::USIZE;
    /// The bitmask for the lower markable bits.
    pub const MARK_MASK: usize = pointer::mark_mask::<T>(Self::MARK_BITS);
    /// The bitmask for the (higher) pointer bits.
    pub const POINTER_MASK: usize = !Self::MARK_MASK;

    /// Consumes `self` and returns the inner [`MarkedPtr`](crate::pointer::MarkedPtr)
    #[inline]
    pub fn into_inner(self) -> MarkedPtr<T, N> {
        MarkedPtr::new(self.inner.into_inner())
    }

    /// Loads a value from the pointer.
    ///
    /// `load` takes an [`Ordering`][ordering] argument which describes the memory
    /// ordering of this operation. Possible values are [`SeqCst`][seq_cst],
    /// [`Acquire`][acquire] and [`Relaxed`][relaxed].
    ///
    /// # Panics
    ///
    /// Panics if `order` is [`Release`][release] or [`AcqRel`][acq_rel].
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [relaxed]: core::sync::atomic::Ordering::Relaxed
    /// [acquire]: core::sync::atomic::Ordering::Acquire
    /// [release]: core::sync::atomic::Ordering::Release
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
    /// [seq_cst]: core::sync::atomic::Ordering::SeqCst
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::atomic::Ordering;
    ///
    /// type MarkedPtr<T> = reclaim::MarkedPtr<T, reclaim::typenum::U1>;
    /// type AtomicMarkedPtr<T> = reclaim::AtomicMarkedPtr<T, reclaim::typenum::U1>;
    ///
    /// let ptr = &mut 5;
    /// let marked = MarkedPtr::compose(ptr, 0b1);
    /// let atomic = AtomicMarkedPtr::new(marked);
    ///
    /// let value = atomic.load(Ordering::Relaxed);
    /// assert_eq!((Some(&mut 5), 0b1), unsafe { value.decompose_mut() });
    /// ```
    #[inline]
    pub fn load(&self, order: Ordering) -> MarkedPtr<T, N> {
        MarkedPtr::new(self.inner.load(order))
    }

    /// Stores a value into the pointer.
    ///
    /// `store` takes an [`Ordering`][ordering] argument which describes the
    /// memory ordering of this operation. Possible values are
    /// [`SeqCst`][seq_cst], [`Release`][release] and [`Relaxed`][relaxed].
    ///
    /// # Panics
    ///
    /// Panics if `order` is [`Acquire`][acquire] or [`AcqRel`][acq_rel].
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [relaxed]: core::sync::atomic::Ordering::Relaxed
    /// [acquire]: core::sync::atomic::Ordering::Acquire
    /// [release]: core::sync::atomic::Ordering::Release
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
    /// [seq_cst]: core::sync::atomic::Ordering::SeqCst
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::atomic::Ordering;
    ///
    /// type MarkedPtr<T> = reclaim::MarkedPtr<T, reclaim::typenum::U0>;
    /// type AtomicMarkedPtr<T> = reclaim::AtomicMarkedPtr<T, reclaim::typenum::U0>;
    ///
    /// let ptr = &mut 5;
    /// let marked = MarkedPtr::new(ptr);
    /// let atomic = AtomicMarkedPtr::new(marked);
    ///
    /// let other_marked = MarkedPtr::new(&mut 10);
    ///
    /// atomic.store(other_marked, Ordering::Relaxed);
    /// ```
    #[inline]
    pub fn store(&self, ptr: MarkedPtr<T, N>, order: Ordering) {
        self.inner.store(ptr.inner, order);
    }

    /// Stores a value into the pointer, returning the previous value.
    #[inline]
    pub fn swap(&self, ptr: MarkedPtr<T, N>, order: Ordering) -> MarkedPtr<T, N> {
        MarkedPtr::new(self.inner.swap(ptr.inner, order))
    }

    /// Stores a value into the pointer if the current value is the same
    /// as `current`.
    #[inline]
    pub fn compare_and_swap(
        &self,
        current: MarkedPtr<T, N>,
        new: MarkedPtr<T, N>,
        order: Ordering,
    ) -> MarkedPtr<T, N> {
        MarkedPtr::new(self.inner.compare_and_swap(current.inner, new.inner, order))
    }

    /// Stores a value into the pointer if the current value is the same
    /// as `current`.
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

    /// Stores a value into the pointer if the current value is the same
    /// as `current`.
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

    /// Bitwise `and` with the current tag value.
    ///
    /// Performs a bitwise `and` operation on the current tag and the argument `value` and sets the
    /// new value to the result.
    ///
    /// Returns the [`MarkedPtr`] with the previous tag, the pointer itself can not change.
    /// It `value` is larger than the mask of markable bits of this type it is silently truncated.
    ///
    /// `fetch_and` takes an [`Ordering`] argument, which describes the memory ordering of this
    /// operation.
    /// All orderings modes are possible.
    /// Note, that using [`Acquire`][acq] makes the store part of this operation [`Relaxed`][rlx]
    /// and using [`Release`][rel] makes the load part [`Relaxed][rlx]
    ///
    /// [acq]: core::sync::atomic::Ordering::Acquire
    /// [rel]: core::sync::atomic::Ordering::Release
    /// [rlx]: core::sync::atomic::Ordering::Relaxed
    #[inline]
    pub fn fetch_and(&self, value: usize, order: Ordering) -> MarkedPtr<T, N> {
        unsafe { self.fetch_and_x(AtomicUsize::fetch_and, value, order) }
    }

    /// Bitwise `nand` with the current tag value.
    ///
    /// Performs a bitwise `nand` operation on the current tag and the argument `value` and sets the
    /// new value to the result.
    ///
    /// Returns the [`MarkedPtr`] with the previous tag, the pointer itself can not change.
    /// It `value` is larger than the mask of markable bits of this type it is silently truncated.
    ///
    /// `fetch_nand` takes an [`Ordering`] argument, which describes the memory ordering of this
    /// operation.
    /// All orderings modes are possible.
    /// Note, that using [`Acquire`][acq] makes the store part of this operation [`Relaxed`][rlx]
    /// and using [`Release`][rel] makes the load part [`Relaxed][rlx]
    ///
    /// [acq]: core::sync::atomic::Ordering::Acquire
    /// [rel]: core::sync::atomic::Ordering::Release
    /// [rlx]: core::sync::atomic::Ordering::Relaxed
    #[inline]
    pub fn fetch_nand(&self, value: usize, order: Ordering) -> MarkedPtr<T, N> {
        unsafe { self.fetch_and_x(AtomicUsize::fetch_nand, value, order) }
    }

    /// Bitwise `or` with the current tag value.
    ///
    /// Performs a bitwise `or` operation on the current tag and the argument `value` and sets the
    /// new value to the result.
    ///
    /// Returns the [`MarkedPtr`] with the previous tag, the pointer itself can not change.
    /// It `value` is larger than the mask of markable bits of this type it is silently truncated.
    ///
    /// `fetch_or` takes an [`Ordering`] argument, which describes the memory ordering of this
    /// operation.
    /// All orderings modes are possible.
    /// Note, that using [`Acquire`][acq] makes the store part of this operation [`Relaxed`][rlx]
    /// and using [`Release`][rel] makes the load part [`Relaxed][rlx]
    ///
    /// [acq]: core::sync::atomic::Ordering::Acquire
    /// [rel]: core::sync::atomic::Ordering::Release
    /// [rlx]: core::sync::atomic::Ordering::Relaxed
    #[inline]
    pub fn fetch_or(&self, value: usize, order: Ordering) -> MarkedPtr<T, N> {
        unsafe { self.fetch_and_x(AtomicUsize::fetch_or, value, order) }
    }

    /// Bitwise `xor` with the current tag value.
    ///
    /// Performs a bitwise `xor` operation on the current tag and the argument `value` and sets the
    /// new value to the result.
    ///
    /// Returns the [`MarkedPtr`] with the previous tag, the pointer itself can not change.
    /// It `value` is larger than the mask of markable bits of this type it is silently truncated.
    ///
    /// `fetch_xor` takes an [`Ordering`] argument, which describes the memory ordering of this
    /// operation.
    /// All orderings modes are possible.
    /// Note, that using [`Acquire`][acq] makes the store part of this operation [`Relaxed`][rlx]
    /// and using [`Release`][rel] makes the load part [`Relaxed][rlx]
    ///
    /// [acq]: core::sync::atomic::Ordering::Acquire
    /// [rel]: core::sync::atomic::Ordering::Release
    /// [rlx]: core::sync::atomic::Ordering::Relaxed
    #[inline]
    pub fn fetch_xor(&self, value: usize, order: Ordering) -> MarkedPtr<T, N> {
        unsafe { self.fetch_and_x(AtomicUsize::fetch_xor, value, order) }
    }

    #[inline]
    unsafe fn fetch_and_x(
        &self,
        op: impl FnOnce(&AtomicUsize, usize, Ordering) -> usize,
        value: usize,
        order: Ordering,
    ) -> MarkedPtr<T, N> {
        let cast = &*(self as *const _ as *const AtomicUsize);
        let prev = op(cast, value & Self::MARK_MASK, order);
        MarkedPtr::from(prev as *const _)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl Default
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, N: Unsigned> Default for AtomicMarkedPtr<T, N> {
    #[inline]
    fn default() -> Self {
        Self::null()
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl Debug & Pointer
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, N: Unsigned> fmt::Debug for AtomicMarkedPtr<T, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.load(Ordering::SeqCst).decompose();
        f.debug_struct("AtomicMarkedPtr").field("ptr", &ptr).field("tag", &tag).finish()
    }
}

impl<T, N: Unsigned> fmt::Pointer for AtomicMarkedPtr<T, N> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.load(Ordering::SeqCst).decompose_ptr(), f)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// impl From
////////////////////////////////////////////////////////////////////////////////////////////////////

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

#[cfg(test)]
mod tests {
    use core::ptr;
    use core::sync::atomic::Ordering;

    use typenum::U3;

    use crate::align::Aligned8;

    type AtomicMarkedPtr<T> = crate::pointer::AtomicMarkedPtr<T, U3>;
    type MarkedPtr<T> = crate::pointer::MarkedPtr<T, U3>;

    #[test]
    fn null() {
        let ptr: AtomicMarkedPtr<usize> = AtomicMarkedPtr::null();
        assert_eq!(ptr.load(Ordering::Relaxed).into_usize(), 0);
        assert_eq!(ptr.into_inner().into_usize(), 0);
    }

    #[test]
    fn new() {
        let reference = &Aligned8(1usize);
        let marked = AtomicMarkedPtr::new(MarkedPtr::from(reference));
        let from = AtomicMarkedPtr::from(reference as *const _ as *mut Aligned8<usize>);
        assert_eq!(marked.load(Ordering::Relaxed).into_usize(), reference as *const _ as usize);
        assert_eq!(from.load(Ordering::Relaxed).into_usize(), reference as *const _ as usize);
    }

    #[test]
    fn store() {
        let raw = MarkedPtr::from(&Aligned8(1usize));
        let atomic = AtomicMarkedPtr::null();

        atomic.store(raw, Ordering::Relaxed);
        assert_eq!(atomic.load(Ordering::Relaxed), raw);
    }

    #[test]
    fn swap() {
        let reference = &1i32;
        let atomic: AtomicMarkedPtr<i32> = AtomicMarkedPtr::from(reference as *const _);
        let swap = atomic.swap(MarkedPtr::null(), Ordering::Relaxed);
        assert_eq!(swap.into_usize(), reference as *const _ as usize);
        assert_eq!(atomic.load(Ordering::Relaxed).into_usize(), 0);
    }

    #[test]
    fn compare_exchange() {
        let marked = MarkedPtr::compose(&mut Aligned8(1), 0b11);
        let swap = MarkedPtr::compose(ptr::null_mut(), 0b100);
        let atomic = AtomicMarkedPtr::new(marked);
        let prev =
            atomic.compare_exchange(marked, swap, Ordering::Relaxed, Ordering::Relaxed).unwrap();

        assert_eq!(prev, marked);
        assert_eq!(atomic.load(Ordering::Relaxed), swap);
    }
}
