//! TODO: Docs...

use core::marker::PhantomData;

use typenum::Unsigned;

use crate::pointer::MarkedNonNull;
use crate::{Reclaim, Shared};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Fuse (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub trait Fuse {
    /// TODO: Docs...
    type FuseErr;

    /// TODO: Docs...
    type Reclaimer: Reclaim;

    /// TODO: Docs...
    fn fuse<T, N: Unsigned>(
        self,
        fusable: Fusable<T, Self::Reclaimer, N>,
    ) -> Result<Fused<T, Self::Reclaimer, N>, Self::FuseErr>;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Fusable
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub struct Fusable<T, R, N> {
    ptr: MarkedNonNull<T, N>,
    _marker: PhantomData<R>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Fused
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub struct Fused<T, R: Reclaim, N> {
    guard: R::Guard,
    ptr: MarkedNonNull<T, N>,
}

impl<T, R: Reclaim, N: Unsigned> Fused<T, R, N> {
    /// TODO: Docs...
    #[inline]
    pub fn into_guard(self) -> R::Guard {
        self.guard
    }

    /// TODO: Docs...
    #[inline]
    pub fn shared(&self) -> Shared<T, R, N> {
        unimplemented!()
    }

    /// TODO: Docs...
    #[inline]
    pub unsafe fn from_raw_parts(guard: R::Guard, fusable: Fusable<T, R, N>) -> Self {
        Self { guard, ptr: fusable.ptr }
    }
}
