use std::alloc::Global;
use std::sync::atomic::Ordering;

use crate::atomic::Atomic;
use crate::marked::MarkedPtr;
use crate::{Reclaim, Protected, Shared, Unlinked};

#[derive(Default)]
pub struct Leaking;

#[derive(Default)]
pub struct LeakingGuard<T>(MarkedPtr<T>);

pub struct Header {
    pub checksum: usize
}

impl Default for Header {
    fn default() -> Self {
        Self { checksum: 0xDEADBEEF }
    }
}

unsafe impl Reclaim<Global> for Leaking {
    type RecordHeader = Header;

    unsafe fn reclaim<T>(unlinked: Unlinked<T, Self, Global>) where T: 'static {}
    unsafe fn reclaim_unchecked<T>(unlinked: Unlinked<T, Self, Global>) {}
}

impl<T> Protected<Global> for LeakingGuard<T> {
    type Item = T;
    type Reclaimer = Leaking;

    fn new() -> Self {
        Self(MarkedPtr::null())
    }

    fn shared(&self) -> Option<Shared<Self::Item, Self::Reclaimer, Global>> {
        unsafe { Shared::from_marked(self.0) }
    }

    fn acquire(
        &mut self,
        atomic: &Atomic<Self::Item, Self::Reclaimer, Global>,
        order: Ordering
    ) -> Option<Shared<Self::Item, Self::Reclaimer, Global>> {
        self.0 = atomic.load_raw(order);
        unsafe { Shared::from_marked(self.0) }
    }

    fn acquire_if_equal(
        &mut self,
        atomic: &Atomic<Self::Item, Self::Reclaimer, Global>,
        compare: MarkedPtr<Self::Item>,
        order: Ordering
    ) -> Result<Option<Shared<Self::Item, Self::Reclaimer, Global>>, crate::NotEqual> {
        match atomic.load_raw(order) {
            marked if marked == compare => {
                self.0 = marked;
                unsafe { Ok(Shared::from_marked(marked)) }
            },
            _ => Err(crate::NotEqual),
        }
    }

    fn release(&mut self) {
        self.0 = MarkedPtr::null();
    }
}