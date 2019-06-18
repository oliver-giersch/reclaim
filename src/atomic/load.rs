use core::marker::PhantomData;
use core::sync::atomic::Ordering;

use typenum::Unsigned;

use crate::atomic::Atomic;
use crate::pointer::Internal;
use crate::{
    AcquireResult, Marked, MarkedNonNull, MarkedPtr, NotEqualError, Protect, ProtectRegion,
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
    /// TODO: Docs...
    type Item: Sized;

    /// TODO: Docs...
    type Reclaimer: Reclaim;

    /// TODO: Docs...
    type MarkBits: Unsigned;

    /// Loads a value from the pointer and `guard` to protect it.
    ///
    /// If the loaded value is non-null, the value is guaranteed to be protected
    /// from reclamation during the lifetime of `guard`. This method
    /// internally relies on [`protect`](crate::Protect::protect).
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

    /// TODO: Docs...
    fn load_if_equal<'g>(
        &self,
        expected: MarkedPtr<Self::Item, Self::MarkBits>,
        order: Ordering,
        guard: &'g mut impl Protect<Reclaimer = Self::Reclaimer>,
    ) -> Result<Option<Shared<'g, T, R, N>>, NotEqualError> {
        self.load_marked_if_equal(expected, order, guard).map(Marked::value)
    }

    fn load_marked<'g>(
        &self,
        order: Ordering,
        guard: &'g mut impl Protect<Reclaimer = Self::Reclaimer>,
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
    Self::Reclaimer::Guard: ProtectRegion,
{
    /// TODO: Docs...
    type Item: Sized;

    /// TODO: Docs...
    type Reclaimer: Reclaim;

    /// TODO: Docs...
    type MarkBits: Unsigned;

    /// TODO: Docs...
    fn load<'g>(
        &self,
        order: Ordering,
        guard: &'g Self::Reclaimer::Guard,
    ) -> Option<Shared<'g, Self::Item, Self::Reclaimer, Self::MarkBits>> {
        self.load_marked(order, guard).value()
    }

    /// TODO: Docs...
    fn load_if_equal<'g>(
        &self,
        expected: MarkedPtr<Self::Item, Self::MarkBits>,
        order: Ordering,
        guard: &'g impl Protect<Reclaimer = Self::Reclaimer>,
    ) -> Result<Option<Shared<'g, T, R, N>>, NotEqualError> {
        self.load_marked_if_equal(expected, order, guard).map(Marked::value)
    }

    /// TODO: Docs...
    fn load_marked<'g>(
        &self,
        order: Ordering,
        _: &'g Self::Reclaimer::Guard,
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
        guard: &'g impl Protect<Reclaimer = Self::Reclaimer>,
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
        _: &'g Self::Reclaimer::Guard,
    ) -> Marked<Shared<'g, Self::Item, Self::Reclaimer, Self::MarkBits>> {
        MarkedNonNull::new(self.load_raw(order))
            .map(|ptr| Shared { inner: ptr, _marker: PhantomData })
    }

    fn load_marked_if_equal<'g>(
        &self,
        expected: MarkedPtr<Self::Item, Self::MarkBits>,
        order: Ordering,
        guard: &'g mut impl Protect<Reclaimer = Self::Reclaimer>,
    ) -> Result<Marked<Shared<Self::Item, Self::Reclaimer, Self::MarkBits>>, NotEqualError> {
        unimplemented!()
    }
}
