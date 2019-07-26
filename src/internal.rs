//! Internal traits which may appear in public interfaces, but are not actually
//! exported by the crate.

use core::ptr::NonNull;
use core::sync::atomic::Ordering;

use typenum::Unsigned;

use crate::atomic::Atomic;
use crate::pointer::{Marked, MarkedPointer, MarkedPtr};
use crate::{AcquireResult, Reclaim, Shared};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Guard (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A sealed trait for abstracting over different types for valid guard references.
///
/// For guard types implementing only the [`Protect`](crate::Protect) trait,
/// this trait is only implemented for *mutable* references to this type.
/// For guard types that also implement the
/// [`ProtectRegion`](crate::ProtectRegion) trait, this trait is also
/// implemented for *shared* references.
pub trait GuardRef<'g> {
    type Reclaimer: Reclaim;

    fn load_protected<T, N: Unsigned>(
        self,
        atomic: &Atomic<T, Self::Reclaimer, N>,
        order: Ordering,
    ) -> Marked<Shared<'g, T, Self::Reclaimer, N>>;

    fn load_protected_if_equal<T, N: Unsigned>(
        self,
        atomic: &Atomic<T, Self::Reclaimer, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> AcquireResult<'g, T, Self::Reclaimer, N>;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Compare (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Trait for pointer types that can be compared against in atomic
/// *compare-and-swap* operations.
pub trait Compare: MarkedPointer + Sized {
    type Reclaimer: Reclaim;
    type Unlinked: MarkedPointer<Item = Self::Item, MarkBits = Self::MarkBits>;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Store (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Trait for pointer types that can be stored in an `Atomic`.
pub trait Store: MarkedPointer + Sized {
    type Reclaimer: Reclaim;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Internal (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A general purpose sealed marker trait for all relevant types of this crate.
pub trait Internal {}

impl<'a, T> Internal for &'a T {}
impl<'a, T> Internal for &'a mut T {}
impl<T> Internal for NonNull<T> {}
