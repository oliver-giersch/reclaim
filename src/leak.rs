//! A NOP memory reclamation scheme that leaks memory, mainly for exemplary and testing purposes.

use core::sync::atomic::Ordering;

use typenum::Unsigned;

use crate::marked::MarkedPtr;
use crate::{Reclaim, Protected};

pub type Atomic<T, N> = crate::Atomic<T, N, Leaking>;
pub type Shared<'g, T, N> = crate::Shared<'g, T, N, Leaking>;
pub type Owned<T, N> = crate::owned::Owned<T, N, Leaking>;
pub type Unlinked<T, N> = crate::Unlinked<T, N, Leaking>;
pub type Unprotected<T, N> = crate::Unprotected<T, N, Leaking>;

#[derive(Debug, Default)]
pub struct Leaking;

#[derive(Default)]
pub struct LeakingGuard<T, N: Unsigned>(MarkedPtr<T, N>);

pub struct Header {
    pub checksum: usize,
}

impl Default for Header {
    fn default() -> Self {
        Self {
            checksum: 0xDEADBEEF,
        }
    }
}

unsafe impl Reclaim for Leaking {
    type RecordHeader = Header;

    unsafe fn retire<T, N: Unsigned>(_: Unlinked<T, N>)
    where
        T: 'static,
    {
    }
    unsafe fn retire_unchecked<T, N: Unsigned>(_: Unlinked<T, N>) {}
}

impl<T, N: Unsigned> Protected for LeakingGuard<T, N> {
    type Item = T;
    type MarkBits = N;
    type Reclaimer = Leaking;

    fn new() -> Self {
        Self(MarkedPtr::null())
    }

    fn shared(&self) -> Option<Shared<Self::Item, Self::MarkBits>> {
        unsafe { Shared::from_marked(self.0) }
    }

    fn acquire(
        &mut self,
        atomic: &Atomic<Self::Item, Self::MarkBits,>,
        order: Ordering,
    ) -> Option<Shared<Self::Item, Self::MarkBits>> {
        self.0 = atomic.load_raw(order);
        unsafe { Shared::from_marked(self.0) }
    }

    fn acquire_if_equal(
        &mut self,
        atomic: &Atomic<Self::Item, Self::MarkBits>,
        compare: MarkedPtr<Self::Item, Self::MarkBits>,
        order: Ordering,
    ) -> Result<Option<Shared<Self::Item, Self::MarkBits>>, crate::NotEqual> {
        match atomic.load_raw(order) {
            marked if marked == compare => {
                self.0 = marked;
                unsafe { Ok(Shared::from_marked(marked)) }
            }
            _ => Err(crate::NotEqual),
        }
    }

    fn release(&mut self) {
        self.0 = MarkedPtr::null();
    }
}