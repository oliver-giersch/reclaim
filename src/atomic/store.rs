use crate::pointer::{Marked, MarkedPointer};
use crate::{Owned, Reclaim, Shared, Unlinked, Unprotected, Unsigned};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Store (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Trait for pointer types that can be stored in an `Atomic`.
pub trait Store: MarkedPointer + Sized {
    type Reclaimer: Reclaim;
}

impl<T, R: Reclaim, N: Unsigned> Store for Owned<T, R, N> {
    type Reclaimer = R;
}

impl<T, R: Reclaim, N: Unsigned> Store for Option<Owned<T, R, N>> {
    type Reclaimer = R;
}

impl<T, R: Reclaim, N: Unsigned> Store for Marked<Owned<T, R, N>> {
    type Reclaimer = R;
}

impl<'g, T, R: Reclaim, N: Unsigned> Store for Shared<'g, T, R, N> {
    type Reclaimer = R;
}

impl<'g, T, R: Reclaim, N: Unsigned> Store for Option<Shared<'g, T, R, N>> {
    type Reclaimer = R;
}

impl<'g, T, R: Reclaim, N: Unsigned> Store for Marked<Shared<'g, T, R, N>> {
    type Reclaimer = R;
}

impl<T, R: Reclaim, N: Unsigned> Store for Unlinked<T, R, N> {
    type Reclaimer = R;
}

impl<T, R: Reclaim, N: Unsigned> Store for Option<Unlinked<T, R, N>> {
    type Reclaimer = R;
}

impl<T, R: Reclaim, N: Unsigned> Store for Marked<Unlinked<T, R, N>> {
    type Reclaimer = R;
}

impl<T, R: Reclaim, N: Unsigned> Store for Unprotected<T, R, N> {
    type Reclaimer = R;
}

impl<T, R: Reclaim, N: Unsigned> Store for Option<Unprotected<T, R, N>> {
    type Reclaimer = R;
}

impl<T, R: Reclaim, N: Unsigned> Store for Marked<Unprotected<T, R, N>> {
    type Reclaimer = R;
}
