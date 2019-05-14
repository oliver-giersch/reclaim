//! A no-op memory reclamation scheme that leaks memory, mainly for exemplary and testing purposes.

use core::sync::atomic::Ordering;

use typenum::Unsigned;

use crate::pointer::{Marked, MarkedPointer, MarkedPtr};
use crate::{AcquireResult, LocalReclaim, Protect};

/// An [`Atomic`][crate::Atomic] type that uses the no-op [`Leaking`] "reclamation" scheme.
pub type Atomic<T, N> = crate::Atomic<T, Leaking, N>;
/// A [`Shared`][crate::Shared] type that uses the no-op [`Leaking`] "reclamation" scheme.
pub type Shared<'g, T, N> = crate::Shared<'g, T, Leaking, N>;
/// An [`Owned`][crate::Owned] type that uses the no-op [`Leaking`] "reclamation" scheme.
pub type Owned<T, N> = crate::Owned<T, Leaking, N>;
/// An [`Unlinked`][crate::Unlinked] type that uses the no-op [`Leaking`] "reclamation" scheme.
pub type Unlinked<T, N> = crate::Unlinked<T, Leaking, N>;
/// An [`Unprotected`][crate::Unprotected] type that uses the no-op [`Leaking`] "reclamation" scheme.
pub type Unprotected<T, N> = crate::Unprotected<T, Leaking, N>;

/// A no-op memory "reclamation" scheme that deliberately leaks all memory.
#[derive(Debug, Default)]
pub struct Leaking;

/// The corresponding guard type for the [`Leaking`](Leaking) type.
///
/// Since leaking reclamation is a no-op, this is just a thin wrapper
/// for a marked pointer.
pub struct LeakingGuard<T, N: Unsigned>(MarkedPtr<T, N>);

impl<T, N: Unsigned> Clone for LeakingGuard<T, N> {
    #[inline]
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl<T, N: Unsigned> Default for LeakingGuard<T, N> {
    #[inline]
    fn default() -> Self {
        Self(MarkedPtr::null())
    }
}

#[cfg(test)]
pub struct Header {
    pub checksum: usize,
}

#[cfg(test)]
impl Default for Header {
    #[inline]
    fn default() -> Self {
        Self { checksum: 0xDEAD_BEEF }
    }
}

unsafe impl LocalReclaim for Leaking {
    type Local = ();

    #[cfg(test)]
    type RecordHeader = Header;
    #[cfg(not(test))]
    type RecordHeader = ();

    /// Leaks the given value.
    ///
    /// # Safety
    ///
    /// Contrary to the specifications of the trait methods, this particular specialization is
    /// always safe to call.
    #[inline]
    unsafe fn retire_local<T: 'static, N: Unsigned>(_: &(), _: Unlinked<T, N>) {}
    /// Leaks the given value.
    ///
    /// # Safety
    ///
    /// Contrary to the specifications of the trait methods, this particular specialization is
    /// always safe to call.
    #[inline]
    unsafe fn retire_local_unchecked<T, N: Unsigned>(_: &(), _: Unlinked<T, N>) {}
}

unsafe impl<T, N: Unsigned> Protect for LeakingGuard<T, N> {
    type Item = T;
    type Reclaimer = Leaking;
    type MarkBits = N;

    /// Gets a shared reference wrapped in a [`Marked`] for the protected value,
    /// which is tied to the lifetime of self.
    #[inline]
    fn marked(&self) -> Marked<Shared<Self::Item, Self::MarkBits>> {
        unsafe { Marked::from_marked_ptr(self.0) }
    }

    /// Acquires a value from shared memory.
    #[inline]
    fn acquire(
        &mut self,
        atomic: &Atomic<Self::Item, Self::MarkBits>,
        order: Ordering,
    ) -> Marked<Shared<Self::Item, Self::MarkBits>> {
        self.0 = atomic.load_raw(order);
        unsafe { Marked::from_marked_ptr(self.0) }
    }

    /// Acquires a value from shared memory if it equals `expected`.
    #[inline]
    fn acquire_if_equal(
        &mut self,
        atomic: &Atomic<Self::Item, Self::MarkBits>,
        expected: MarkedPtr<Self::Item, Self::MarkBits>,
        order: Ordering,
    ) -> AcquireResult<Self::Item, Self::Reclaimer, Self::MarkBits> {
        match atomic.load_raw(order) {
            marked if marked == expected => {
                self.0 = marked;
                unsafe { Ok(Marked::from_marked_ptr(marked)) }
            }
            _ => Err(crate::NotEqualError),
        }
    }

    /// Discards the current value, replacing it with `null`.
    #[inline]
    fn release(&mut self) {
        self.0 = MarkedPtr::null();
    }
}
