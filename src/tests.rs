use std::sync::atomic::Ordering;

use typenum::Unsigned;

use crate::atomic::Atomic;
use crate::marked::MarkedPtr;
use crate::{Protected, Reclaim, Shared, Unlinked};

#[derive(Default)]
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

    unsafe fn reclaim<T, N: Unsigned>(_: Unlinked<T, N, Self>)
    where
        T: 'static,
    {
    }
    unsafe fn reclaim_unchecked<T, N: Unsigned>(_: Unlinked<T, N, Self>) {}
}

impl<T, N: Unsigned> Protected for LeakingGuard<T, N> {
    type Item = T;
    type MarkBits = N;
    type Reclaimer = Leaking;

    fn new() -> Self {
        Self(MarkedPtr::null())
    }

    fn shared(&self) -> Option<Shared<Self::Item, Self::MarkBits, Self::Reclaimer>> {
        unsafe { Shared::from_marked(self.0) }
    }

    fn acquire(
        &mut self,
        atomic: &Atomic<Self::Item, Self::MarkBits, Self::Reclaimer>,
        order: Ordering,
    ) -> Option<Shared<Self::Item, Self::MarkBits, Self::Reclaimer>> {
        self.0 = atomic.load_raw(order);
        unsafe { Shared::from_marked(self.0) }
    }

    fn acquire_if_equal(
        &mut self,
        atomic: &Atomic<Self::Item, Self::MarkBits, Self::Reclaimer>,
        compare: MarkedPtr<Self::Item, Self::MarkBits>,
        order: Ordering,
    ) -> Result<Option<Shared<Self::Item, Self::MarkBits, Self::Reclaimer>>, crate::NotEqual> {
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
