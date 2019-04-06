//! A NOP memory reclamation scheme that leaks memory, mainly for exemplary and testing purposes.

use core::sync::atomic::Ordering;

use typenum::Unsigned;

use crate::marked::MarkedPtr;
use crate::{Protect, Reclaim};

/// An [`Atomic`](../struct.Atomic.html) type that uses the NOP [`Leaking`](struct.Leaking.html) "reclamation scheme"
pub type Atomic<T, N> = crate::Atomic<T, N, Leaking>;
/// A [`Shared`](../struct.Shared.html) type that uses the NOP [`Leaking`](struct.Leaking.html) "reclamation scheme"
pub type Shared<'g, T, N> = crate::Shared<'g, T, N, Leaking>;
/// An [`Owned`](../struct.Owned.html) type that uses the NOP [`Leaking`](struct.Leaking.html) "reclamation scheme"
pub type Owned<T, N> = crate::Owned<T, N, Leaking>;
/// An [`Unlinked`](../struct.Unlinked.html) type that uses the NOP [`Leaking`](struct.Leaking.html) "reclamation scheme"
pub type Unlinked<T, N> = crate::Unlinked<T, N, Leaking>;
/// An [`Unprotected`](../struct.Unprotected.html) type that uses the NOP [`Leaking`](struct.Leaking.html) "reclamation scheme"
pub type Unprotected<T, N> = crate::Unprotected<T, N, Leaking>;

/// A NOP memory reclamation scheme that leaks memory.
#[derive(Debug, Default)]
pub struct Leaking;

/// TODO: Doc...
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

    #[inline]
    unsafe fn retire<T: 'static, N: Unsigned>(_: Unlinked<T, N>) {}
    #[inline]
    unsafe fn retire_unchecked<T, N: Unsigned>(_: Unlinked<T, N>) {}
}

impl<T, N: Unsigned> Protect for LeakingGuard<T, N> {
    type Item = T;
    type MarkBits = N;
    type Reclaimer = Leaking;

    #[inline]
    fn new() -> Self {
        Self(MarkedPtr::null())
    }

    #[inline]
    fn shared(&self) -> Option<Shared<Self::Item, Self::MarkBits>> {
        unsafe { Shared::try_from_marked(self.0) }
    }

    #[inline]
    fn acquire(
        &mut self,
        atomic: &Atomic<Self::Item, Self::MarkBits>,
        order: Ordering,
    ) -> Option<Shared<Self::Item, Self::MarkBits>> {
        self.0 = atomic.load_raw(order);
        unsafe { Shared::try_from_marked(self.0) }
    }

    #[inline]
    fn acquire_if_equal(
        &mut self,
        atomic: &Atomic<Self::Item, Self::MarkBits>,
        expected: MarkedPtr<Self::Item, Self::MarkBits>,
        order: Ordering,
    ) -> Result<Option<Shared<Self::Item, Self::MarkBits>>, crate::NotEqual> {
        match atomic.load_raw(order) {
            marked if marked == expected => {
                self.0 = marked;
                unsafe { Ok(Shared::try_from_marked(marked)) }
            }
            _ => Err(crate::NotEqual),
        }
    }

    #[inline]
    fn acquire_from_other(&mut self, other: &Self) {
        self.0 = other.0;
    }

    #[inline]
    fn release(&mut self) {
        self.0 = MarkedPtr::null();
    }
}
