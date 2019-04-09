//! A no-op memory reclamation scheme that leaks memory, mainly for exemplary and testing purposes.

use core::sync::atomic::Ordering;

use typenum::Unsigned;

use crate::marked::MarkedPtr;
use crate::{AcquireResult, Protect, Reclaim};

/// An [`Atomic`](../struct.Atomic.html) type that uses the no-op [`Leaking`](struct.Leaking.html) "reclamation" scheme
pub type Atomic<T, N> = crate::Atomic<T, N, Leaking>;
/// A [`Shared`](../struct.Shared.html) type that uses the no-op [`Leaking`](struct.Leaking.html) "reclamation" scheme
pub type Shared<'g, T, N> = crate::Shared<'g, T, N, Leaking>;
/// An [`Owned`](../struct.Owned.html) type that uses the no-op [`Leaking`](struct.Leaking.html) "reclamation" scheme
pub type Owned<T, N> = crate::Owned<T, N, Leaking>;
/// An [`Unlinked`](../struct.Unlinked.html) type that uses the no-op [`Leaking`](struct.Leaking.html) "reclamation" scheme
pub type Unlinked<T, N> = crate::Unlinked<T, N, Leaking>;
/// An [`Unprotected`](../struct.Unprotected.html) type that uses the no-op [`Leaking`](struct.Leaking.html) "reclamation" scheme
pub type Unprotected<T, N> = crate::Unprotected<T, N, Leaking>;

/// A no-op memory "reclamation" scheme that deliberately leaks all memory.
#[derive(Debug, Default)]
pub struct Leaking;

/// The corresponding guard type for the [`Leaking`](Leaking) type.
///
/// Since leaking reclamation is a no-op, this is just a thin wrapper
/// for a marked pointer.
pub struct LeakingGuard<T, N: Unsigned>(MarkedPtr<T, N>);

impl<T, N: Unsigned> Clone for LeakingGuard<T, N> {
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
        Self {
            checksum: 0xDEAD_BEEF,
        }
    }
}

unsafe impl Reclaim for Leaking {
    #[cfg(test)]
    type RecordHeader = Header;
    #[cfg(not(test))]
    type RecordHeader = ();

    /// Leaks the given value.
    #[inline]
    unsafe fn retire<T: 'static, N: Unsigned>(_: Unlinked<T, N>) {}
    /// Leaks the given value.
    #[inline]
    unsafe fn retire_unchecked<T, N: Unsigned>(_: Unlinked<T, N>) {}
}

impl<T, N: Unsigned> Protect for LeakingGuard<T, N> {
    type Item = T;
    type MarkBits = N;
    type Reclaimer = Leaking;

    /// Gets a new `null` pointer guard.
    #[inline]
    fn new() -> Self {
        Self(MarkedPtr::null())
    }

    /// Gets the optional [`Shared`](crate::Shared) value for the guard.
    #[inline]
    fn shared(&self) -> Option<Shared<Self::Item, Self::MarkBits>> {
        unsafe { Shared::try_from_marked(self.0) }
    }

    /// Acquires a value from shared memory.
    #[inline]
    fn acquire(
        &mut self,
        atomic: &Atomic<Self::Item, Self::MarkBits>,
        order: Ordering,
    ) -> Option<Shared<Self::Item, Self::MarkBits>> {
        self.0 = atomic.load_raw(order);
        unsafe { Shared::try_from_marked(self.0) }
    }

    /// Acquires a value from shared memory if it equals `expected`.
    #[inline]
    fn acquire_if_equal(
        &mut self,
        atomic: &Atomic<Self::Item, Self::MarkBits>,
        expected: MarkedPtr<Self::Item, Self::MarkBits>,
        order: Ordering,
    ) -> AcquireResult<Self::Item, Self::MarkBits, Self::Reclaimer> {
        match atomic.load_raw(order) {
            marked if marked == expected => {
                self.0 = marked;
                unsafe { Ok(Shared::try_from_marked(marked)) }
            }
            _ => Err(crate::NotEqual),
        }
    }

    /// Discards the current value, replacing it with `null`.
    #[inline]
    fn release(&mut self) {
        self.0 = MarkedPtr::null();
    }
}
