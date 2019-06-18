use core::cmp;
use core::fmt;
use core::marker::PhantomData;
use core::mem;
use core::ptr::NonNull;

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

use crate::{Reclaim, Record};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Retired
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A type-erased fat pointer to a retired record.
pub struct Retired<R>(NonNull<dyn Any + 'static>, PhantomData<R>);

impl<R: Reclaim + 'static> Retired<R> {
    /// Creates a new [`Retired`] record from a raw pointer.
    ///
    /// # Safety
    ///
    /// The caller has to ensure numerous safety invariants in order for a
    /// [`Retired`] record to be used safely:
    ///
    /// - the given `record` pointer **must** point to a valid heap allocated
    ///   value
    /// - the record **must** have been allocated as part of a [`Record`] of
    ///   the appropriate [`LocalReclaim`] implementation
    /// - *if* the type of the retired record implements [`Drop`] *and* contains
    ///   any non-static references, it must be ensured that these are **not**
    ///   accessed by the [`drop`][Drop::drop] function.
    #[inline]
    pub unsafe fn new_unchecked<'a, T: 'a>(record: NonNull<T>) -> Self {
        let any: NonNull<dyn Any + 'a> = Record::<T, R>::from_raw_non_null(record);
        let any: NonNull<dyn Any + 'static> = mem::transmute(any);

        Self(any, PhantomData)
    }

    /// Converts a retired record to a raw pointer.
    ///
    /// Since retired records are type-erased trait object (fat) pointers to
    /// retired values that should no longer be used, only the 'address' part
    /// of the pointer is returned, i.e. a pointer to an `()`.
    #[inline]
    pub fn as_ptr(&self) -> *const () {
        self.0.as_ptr() as *mut () as *const ()
    }

    /// Returns the numeric representation of the retired record's memory
    /// address.
    #[inline]
    pub fn address(&self) -> usize {
        self.0.as_ptr() as *mut () as usize
    }

    /// Reclaims the retired record by dropping it and de-allocating its memory.
    ///
    /// # Safety
    ///
    /// This method **must** not be called more than once or when some other
    /// thread or scope still has some reference to the record.
    #[inline]
    pub unsafe fn reclaim(&mut self) {
        mem::drop(Box::from_raw(self.0.as_ptr()));
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// cmp
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<R: Reclaim + 'static> PartialEq for Retired<R> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_ptr().eq(&other.as_ptr())
    }
}

impl<R: Reclaim + 'static> PartialOrd for Retired<R> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.as_ptr().partial_cmp(&other.as_ptr())
    }
}

impl<R: Reclaim + 'static> Ord for Retired<R> {
    #[inline]
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        self.as_ptr().cmp(&other.as_ptr())
    }
}

impl<R: Reclaim + 'static> Eq for Retired<R> {}

////////////////////////////////////////////////////////////////////////////////////////////////////
// fmt
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<R: Reclaim + 'static> fmt::Debug for Retired<R> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Retired").field("address", &self.as_ptr()).finish()
    }
}

impl<R: Reclaim + 'static> fmt::Display for Retired<R> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.as_ptr(), f)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Any (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

trait Any {}
impl<T> Any for T {}
