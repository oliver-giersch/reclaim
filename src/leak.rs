//! A no-op memory reclamation scheme that leaks memory, mainly for exemplary
//! and testing purposes.

use core::sync::atomic::Ordering;

use typenum::Unsigned;

use crate::pointer::{Marked, MarkedPointer, MarkedPtr};
use crate::{AcquireResult, GlobalReclaim, Protect, ProtectRegion, Reclaim};

/// An [`Atomic`][crate::Atomic] type that uses the no-op [`Leaking`]
/// "reclamation" scheme.
pub type Atomic<T, N> = crate::Atomic<T, Leaking, N>;
/// A [`Shared`][crate::Shared] type that uses the no-op [`Leaking`]
/// "reclamation" scheme.
pub type Shared<'g, T, N> = crate::Shared<'g, T, Leaking, N>;
/// An [`Owned`][crate::Owned] type that uses the no-op [`Leaking`]
/// "reclamation" scheme.
pub type Owned<T, N> = crate::Owned<T, Leaking, N>;
/// An [`Unlinked`][crate::Unlinked] type that uses the no-op [`Leaking`]
/// "reclamation" scheme.
pub type Unlinked<T, N> = crate::Unlinked<T, Leaking, N>;
/// An [`Unprotected`][crate::Unprotected] type that uses the no-op [`Leaking`]
/// "reclamation" scheme.
pub type Unprotected<T, N> = crate::Unprotected<T, Leaking, N>;

////////////////////////////////////////////////////////////////////////////////////////////////////
// Leaking
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A no-op memory "reclamation" scheme that deliberately leaks all memory.
#[derive(Debug, Default)]
pub struct Leaking;

/********** impl inherent *************************************************************************/

impl Leaking {
    /// Leaks the given `unlinked`.
    ///
    /// This is safe wrapper for [`retire`][Reclaim::retire], which does not
    /// require any invariants to be maintained, because retired records are
    /// not freed but leaked.
    #[inline]
    pub fn leak<T, N: Unsigned>(unlinked: Unlinked<T, N>) {
        unsafe { Self::retire_unchecked(unlinked) };
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Guard
////////////////////////////////////////////////////////////////////////////////////////////////////

/// The [`Guard`][Reclaim::Guard] type for the [`Leaking`] "reclamation".
#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Guard;

/********** impl inherent *************************************************************************/

impl Guard {
    /// Creates a new empty [`LeakingGuard`].
    #[inline]
    pub fn new() -> Self {
        Self
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Header
////////////////////////////////////////////////////////////////////////////////////////////////////

#[cfg(test)]
pub struct Header {
    pub checksum: usize,
}

/********** impl Default **************************************************************************/

#[cfg(test)]
impl Default for Header {
    #[inline]
    fn default() -> Self {
        Self { checksum: 0xDEAD_BEEF }
    }
}

/********** impl GlobalReclaim ********************************************************************/

unsafe impl GlobalReclaim for Leaking {
    type Guard = Guard;

    #[inline]
    fn try_reclaim() {}

    #[inline]
    unsafe fn retire<T: 'static, N: Unsigned>(_: Unlinked<T, N>) {}

    #[inline]
    unsafe fn retire_unchecked<T, N: Unsigned>(_: Unlinked<T, N>) {}
}

/********** impl Reclaim **************************************************************************/

unsafe impl Reclaim for Leaking {
    type Local = ();

    #[cfg(test)]
    type RecordHeader = Header;
    #[cfg(not(test))]
    type RecordHeader = ();

    /// Leaks the given value.
    ///
    /// # Safety
    ///
    /// Contrary to the specifications of the trait's method, this particular
    /// implementation is always safe to call.
    #[inline]
    unsafe fn retire_local<T: 'static, N: Unsigned>(_: &(), _: Unlinked<T, N>) {}

    /// Leaks the given value.
    ///
    /// # Safety
    ///
    /// Contrary to the specifications of the trait's method, this particular
    /// implementation is always safe to call.
    #[inline]
    unsafe fn retire_local_unchecked<T, N: Unsigned>(_: &(), _: Unlinked<T, N>) {}
}

/********** impl Protect **************************************************************************/

unsafe impl Protect for Guard {
    type Reclaimer = Leaking;

    /// This is a no-op.
    #[inline]
    fn release(&mut self) {}

    /// Acquires a value from shared memory.
    #[inline]
    fn protect<T, N: Unsigned>(
        &mut self,
        atomic: &Atomic<T, N>,
        order: Ordering,
    ) -> Marked<Shared<T, N>> {
        unsafe { Marked::from_marked_ptr(atomic.load_raw(order)) }
    }

    /// Acquires a value from shared memory if it equals `expected`.
    #[inline]
    fn protect_if_equal<T, N: Unsigned>(
        &mut self,
        atomic: &Atomic<T, N>,
        expected: MarkedPtr<T, N>,
        order: Ordering,
    ) -> AcquireResult<T, Self::Reclaimer, N> {
        match atomic.load_raw(order) {
            raw if raw == expected => Ok(unsafe { Marked::from_marked_ptr(raw) }),
            _ => Err(crate::NotEqualError),
        }
    }
}

/********** impl ProtectRegion ********************************************************************/

unsafe impl ProtectRegion for Guard {}
