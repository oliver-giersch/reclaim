//! TODO: new reclaim trait API

use core::sync::atomic::Ordering;

use typenum::Unsigned;

use crate::atomic::Atomic;
use crate::pointer::{Marked, MarkedPtr};
use crate::retired::Retired;
use crate::{NotEqualError, Shared};

/// TODO: Docs...
pub unsafe trait Reclaim: Sized + 'static {
    /// TODO: Docs...
    type RecordHeader: Default + Sync + Sized;
}

/// TODO: Docs...
pub unsafe trait Protect: Clone + Sized {
    /// TODO: Docs...
    type Reclaimer: Reclaim;

    // try_fuse(...)
    /// TODO: Docs...
    fn release(&mut self);

    /// TODO: Docs...
    fn protect<T, N: Unsigned>(
        &mut self,
        src: &Atomic<T, Self::Reclaimer, N>,
        order: Ordering,
    ) -> Marked<Shared<T, Self::Reclaimer, N>>;

    /// TODO: Docs...
    fn protect_if_equal<T, N: Unsigned>(
        &mut self,
        src: &Atomic<T, Self::Reclaimer, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> Result<Marked<Shared<T, Self::Reclaimer, N>>, crate::NotEqualError>;
}

/// TODO: Docs...
pub unsafe trait ProtectRegion: Protect {}

/// TODO: Docs...
pub trait StoreRetired {
    type Reclaimer: Reclaim;

    /// TODO: Docs...
    unsafe fn retire(&self, record: Retired<Self::Reclaimer>);
}
