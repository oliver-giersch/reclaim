use core::sync::atomic::Ordering;

use typenum::Unsigned;

use crate::atomic::Atomic;
use crate::pointer::Internal;
use crate::{
    AcquireResult, Marked, MarkedPointer, MarkedPtr, NotEqualError, Protect, ProtectRegion,
    Reclaim, Shared,
};

////////////////////////////////////////////////////////////////////////////////////////////////////
// LoadProtected (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub trait LoadProtected
where
    Self: Internal + Sized,
{
    /// The type of the value protected from reclamation
    type Item: Sized;

    /// The reclamation scheme associated with this type of guard
    type Reclaimer: Reclaim;

    /// The Number of bits available for storing additional information
    type MarkBits: Unsigned;

    /// Loads a value from the pointer and uses `guard` to protect it.
    ///
    /// If the loaded value is non-null, the value is guaranteed to be protected
    /// from reclamation during the lifetime of `guard` or until the guard is
    /// used to protect a different value.
    /// This method internally relies on [`protect`](crate::Protect::protect).
    ///
    /// `load` takes an [`Ordering`][ordering] argument, which describes the
    /// memory ordering of this operation.
    ///
    /// # Panics
    ///
    /// *May* panic if `order` is [`Release`][release] or [`AcqRel`][acq_rel].
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [release]: core::sync::atomic::Ordering::Release
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
    fn load<'g>(
        &self,
        order: Ordering,
        guard: &'g mut impl Protect<Reclaimer = Self::Reclaimer>,
    ) -> Option<Shared<'g, Self::Item, Self::Reclaimer, Self::MarkBits>> {
        self.load_marked(order, guard).value()
    }

    /// Loads a value from the pointer and uses `guard` to protect it, but only
    /// if the loaded value equals `expected`.
    ///
    /// If the loaded value is non-null, the value is guaranteed to be protected
    /// from reclamation during the lifetime of `guard` or until the guard is
    /// used to protect a different value.
    /// This method internally relies on [`protect`](crate::Protect::protect).
    ///
    /// `load_if_equal` takes an [`Ordering`][ordering] argument, which
    /// describes the memory ordering of this operation.
    ///
    /// # Panics
    ///
    /// *May* panic if `order` is [`Release`][release] or [`AcqRel`][acq_rel].
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [release]: core::sync::atomic::Ordering::Release
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
    fn load_if_equal<'g>(
        &self,
        expected: MarkedPtr<Self::Item, Self::MarkBits>,
        order: Ordering,
        guard: &'g mut impl Protect<Reclaimer = Self::Reclaimer>,
    ) -> Result<Option<Shared<'g, Self::Item, Self::Reclaimer, Self::MarkBits>>, NotEqualError>
    {
        self.load_marked_if_equal(expected, order, guard).map(Marked::value)
    }

    /// Loads a value from the pointer and uses `guard` to protect it.
    /// The (optional) protected [`Shared`] value is wrapped in a [`Marked].
    ///
    /// If the loaded value is non-null, the value is guaranteed to be protected
    /// from reclamation during the lifetime of `guard` or until the guard is
    /// used to protect a different value.
    /// This method internally relies on [`protect`](crate::Protect::protect).
    ///
    /// `load_marked` takes an [`Ordering`][ordering] argument, which describes
    /// the memory ordering of this operation.
    ///
    /// # Panics
    ///
    /// *May* panic if `order` is [`Release`][release] or [`AcqRel`][acq_rel].
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [release]: core::sync::atomic::Ordering::Release
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
    fn load_marked<'g>(
        &self,
        order: Ordering,
        guard: &'g mut impl Protect<Reclaimer = Self::Reclaimer>,
    ) -> Marked<Shared<'g, Self::Item, Self::Reclaimer, Self::MarkBits>>;

    /// Loads a value from the pointer and uses `guard` to protect it, but only
    /// if the loaded value equals `expected`.
    /// The (optional) protected [`Shared`] value is wrapped in a [`Marked].
    ///
    /// If the loaded value is non-null, the value is guaranteed to be protected
    /// from reclamation during the lifetime of `guard` or until the guard is
    /// used to protect a different value.
    /// internally calls [`acquire_if_equal`][Protect::acquire_if_equal].
    ///
    /// `load_if_equal` takes an [`Ordering`][ordering] argument, which
    /// describes the memory ordering of this operation.
    ///
    /// # Panics
    ///
    /// *May* panic if `order` is [`Release`][release] or [`AcqRel`][acq_rel].
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [release]: core::sync::atomic::Ordering::Release
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
    fn load_marked_if_equal<'g>(
        &self,
        expected: MarkedPtr<Self::Item, Self::MarkBits>,
        order: Ordering,
        guard: &'g mut impl Protect<Reclaimer = Self::Reclaimer>,
    ) -> AcquireResult<'g, Self::Item, Self::Reclaimer, Self::MarkBits>;
}

impl<T, R: Reclaim, N: Unsigned> LoadProtected for Atomic<T, R, N> {
    type Item = T;
    type Reclaimer = R;
    type MarkBits = N;

    #[inline]
    fn load_marked<'g>(
        &self,
        order: Ordering,
        guard: &'g mut impl Protect<Reclaimer = Self::Reclaimer>,
    ) -> Marked<Shared<'g, Self::Item, Self::Reclaimer, Self::MarkBits>> {
        guard.protect(self, order)
    }

    #[inline]
    fn load_marked_if_equal<'g>(
        &self,
        expected: MarkedPtr<Self::Item, Self::MarkBits>,
        order: Ordering,
        guard: &'g mut impl Protect<Reclaimer = Self::Reclaimer>,
    ) -> Result<Marked<Shared<'g, Self::Item, Self::Reclaimer, Self::MarkBits>>, NotEqualError>
    {
        guard.protect_if_equal(self, expected, order)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// LoadRegionProtected (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Docs...
pub trait LoadRegionProtected
where
    Self: Internal + Sized,
    <Self::Reclaimer as Reclaim>::Guard: ProtectRegion,
{
    /// The type of the value protected from reclamation
    type Item: Sized;

    /// The reclamation scheme associated with this type of guard
    type Reclaimer: Reclaim;

    /// The Number of bits available for storing additional information
    type MarkBits: Unsigned;

    /// TODO: Docs...
    fn load<'g>(
        &self,
        order: Ordering,
        guard: &'g impl Protect<Reclaimer = Self::Reclaimer>,
    ) -> Option<Shared<'g, Self::Item, Self::Reclaimer, Self::MarkBits>> {
        self.load_marked(order, guard).value()
    }

    /// TODO: Docs...
    fn load_if_equal<'g>(
        &self,
        expected: MarkedPtr<Self::Item, Self::MarkBits>,
        order: Ordering,
        guard: &'g impl Protect<Reclaimer = Self::Reclaimer>,
    ) -> Result<Option<Shared<'g, Self::Item, Self::Reclaimer, Self::MarkBits>>, NotEqualError>
    {
        self.load_marked_if_equal(expected, order, guard).map(Marked::value)
    }

    /// TODO: Docs...
    fn load_marked<'g>(
        &self,
        order: Ordering,
        _: &'g impl Protect<Reclaimer = Self::Reclaimer>,
    ) -> Marked<Shared<'g, Self::Item, Self::Reclaimer, Self::MarkBits>>;

    /// Loads a value wrapped in a [`Marked] from the pointer and stores it
    /// within `guard`, but only if the loaded value equals `expected`.
    ///
    /// If the loaded value is non-null, the value is guaranteed to be protected
    /// from reclamation for as long as it is stored within `guard`. This method
    /// internally calls [`acquire_if_equal`][Protect::acquire_if_equal].
    ///
    /// `load_if_equal` takes an [`Ordering`][ordering] argument, which
    /// describes the memory ordering of this operation.
    ///
    /// # Panics
    ///
    /// *May* panic if `order` is [`Release`][release] or [`AcqRel`][acq_rel].
    ///
    /// [ordering]: core::sync::atomic::Ordering
    /// [release]: core::sync::atomic::Ordering::Release
    /// [acq_rel]: core::sync::atomic::Ordering::AcqRel
    fn load_marked_if_equal<'g>(
        &self,
        expected: MarkedPtr<Self::Item, Self::MarkBits>,
        order: Ordering,
        _: &'g impl Protect<Reclaimer = Self::Reclaimer>,
    ) -> AcquireResult<'g, Self::Item, Self::Reclaimer, Self::MarkBits>;
}

impl<T, R: Reclaim, N: Unsigned> LoadRegionProtected for Atomic<T, R, N>
where
    R::Guard: ProtectRegion,
{
    type Item = T;
    type Reclaimer = R;
    type MarkBits = N;

    #[inline]
    fn load_marked<'g>(
        &self,
        order: Ordering,
        _: &'g impl Protect<Reclaimer = Self::Reclaimer>,
    ) -> Marked<Shared<'g, Self::Item, Self::Reclaimer, Self::MarkBits>> {
        unsafe { Marked::from_marked_ptr(self.load_raw(order)) }
    }

    #[inline]
    fn load_marked_if_equal<'g>(
        &self,
        expected: MarkedPtr<Self::Item, Self::MarkBits>,
        order: Ordering,
        _: &'g impl Protect<Reclaimer = Self::Reclaimer>,
    ) -> Result<Marked<Shared<'g, Self::Item, Self::Reclaimer, Self::MarkBits>>, NotEqualError>
    {
        match self.load_raw(order) {
            raw if raw == expected => Ok(unsafe { Marked::from_marked_ptr(self.load_raw(order)) }),
            _ => Err(NotEqualError),
        }
    }
}
