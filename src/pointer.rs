use core::cmp;
use core::fmt;
use core::marker::PhantomData;
use core::mem;

use typenum::Unsigned;

use crate::owned::Owned;
use crate::{MarkedNonNull, MarkedPtr};
use crate::{Reclaim, Shared, Unlinked, Unprotected};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Pointer (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Trait for nullable and non-nullable markable pointer types.
pub trait MarkedPointer {
    type Item;
    type MarkBits: Unsigned;

    /// TODO: Doc...
    fn as_marked(&self) -> MarkedPtr<Self::Item, Self::MarkBits>;

    /// Consumes `self` and returns a marked pointer
    fn into_marked(self) -> MarkedPtr<Self::Item, Self::MarkBits>;

    /// Constructs a `Self` from a raw pointer.
    ///
    /// # Safety
    ///
    /// The caller has to ensure that raw is a valid pointer for the respective `Self` type.
    /// If `Self` is nullable, a null pointer is a valid value. Otherwise all values must be valid
    /// pointers.
    unsafe fn from_marked(marked: MarkedPtr<Self::Item, Self::MarkBits>) -> Self;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Owned & Option<Owned>
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, N: Unsigned, R: Reclaim> MarkedPointer for Owned<T, N, R> {
    type Item = T;
    type MarkBits = N;

    #[inline]
    fn as_marked(&self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        self.as_marked()
    }

    #[inline]
    fn into_marked(self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        Owned::into_marked(self)
    }

    #[inline]
    unsafe fn from_marked(marked: MarkedPtr<Self::Item, Self::MarkBits>) -> Self {
        debug_assert!(!marked.is_null());
        Owned::from_marked_non_null(MarkedNonNull::new_unchecked(marked))
    }
}

impl<T, N: Unsigned, R: Reclaim> MarkedPointer for Option<Owned<T, N, R>> {
    type Item = T;
    type MarkBits = N;

    #[inline]
    fn as_marked(&self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        unsafe { mem::transmute_copy(self) }
    }

    #[inline]
    fn into_marked(self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        unsafe { mem::transmute(self) }
    }

    #[inline]
    unsafe fn from_marked(marked: MarkedPtr<Self::Item, Self::MarkBits>) -> Self {
        mem::transmute(marked)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Blanket impls
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<T, N: Unsigned, U> PartialEq<U> for MarkedPtr<T, N>
where
    U: MarkedPointer<Item = T, MarkBits = N>,
{
    #[inline]
    fn eq(&self, other: &U) -> bool {
        self.eq(&other.as_marked())
    }
}

impl<T, N: Unsigned, U> PartialOrd<U> for MarkedPtr<T, N>
where
    U: MarkedPointer<Item = T, MarkBits = N>,
{
    #[inline]
    fn partial_cmp(&self, other: &U) -> Option<cmp::Ordering> {
        self.partial_cmp(&other.as_marked())
    }
}

impl<T, N: Unsigned, U> PartialEq<U> for MarkedNonNull<T, N>
where
    U: MarkedPointer<Item = T, MarkBits = N>,
{
    #[inline]
    fn eq(&self, other: &U) -> bool {
        self.into_marked().eq(&other.as_marked())
    }
}

impl<T, N: Unsigned, U> PartialOrd<U> for MarkedNonNull<T, N>
where
    U: MarkedPointer<Item = T, MarkBits = N>,
{
    #[inline]
    fn partial_cmp(&self, other: &U) -> Option<cmp::Ordering> {
        self.into_marked().partial_cmp(&other.as_marked())
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
/// Implementations for Shared, Unlinked and Unprotected + Option<_>
/// Traits:
///   - MarkedPointer
///   - fmt::Debug
///   - fmt::Pointer
///   - cmp::PartialEq<...>
///   - cmp::PartialOrd<...>
////////////////////////////////////////////////////////////////////////////////////////////////////

macro_rules! impl_for_non_nullable {
    ($name:tt, $ptr:ty, $($generics:tt)*) => {
        impl$($generics)* $ptr {
            /// Gets a `None` for `Option<Self>`.
            ///
            /// This is useful for calls to `Atomic::store`, `Atomic::swap` or
            /// `Atomic::compare_exchange_*`, when a `null` pointer needs to be inserted. These
            /// methods accept values of various non-nullable pointer types (`Shared`, `Unlinked`
            /// and `Unprotected`) and `Option` thereof as argument. However, the compiler is
            /// usually not able to infer the concrete type, when a `None` is inserted, and this
            /// function is designed for these cases.
            ///
            /// # Example
            ///
            /// ```
            /// use std::sync::atomic::Ordering;
            ///
            /// type Atomic<T> = reclaim::leak::Atomic<T, reclaim::typenum::U0>;
            /// type Owned<T> = reclaim::leak::Owned<T, reclaim::typenum::U0>;
            ///
            /// let atomic = Atomic::new(1);
            /// let swap = atomic.swap(Owned::none(), Ordering::Relaxed).unwrap();
            ///
            /// assert_eq!(&1, unsafe { swap.deref() });
            /// unsafe { swap.retire() }; // leaks memory
            /// ```
            #[inline]
            pub fn none() -> Option<Self> {
                None
            }

            /// TODO: Doc...
            ///
            /// # Safety
            ///
            /// The caller must ensure that the provided pointer is both non-null and valid.
            ///
            /// ## Shared
            ///
            /// For types with reclamation protection guarantees (e.g. `Shared`) the caller must
            /// also ensure the pointed to record is not reclaimed during the lifetime of the shared
            /// reference.
            #[inline]
            pub unsafe fn from_marked(marked: MarkedPtr<T, N>) -> Option<Self> {
                // this is safe because ...
                mem::transmute(marked)
            }

            /// TODO: Doc...
            #[inline]
            pub unsafe fn from_marked_non_null(marked: MarkedNonNull<T, N>) -> Self {
                Self {
                    inner: marked,
                    _marker: PhantomData,
                }
            }

            /// TODO: Doc...
            #[inline]
            pub fn into_marked(self) -> MarkedPtr<T, N> {
                self.inner.into_marked()
            }

            /// TODO: Doc...
            #[inline]
            pub fn into_marked_non_null(self) -> MarkedNonNull<T, N> {
                self.inner
            }

            /// TODO: Doc...
            #[inline]
            pub fn as_marked(&self) -> MarkedPtr<T, N> {
                self.inner.into_marked()
            }
        }

        impl$($generics)* MarkedPointer for $ptr {
            type Item = T;
            type MarkBits = N;

            #[inline]
            fn as_marked(&self) -> MarkedPtr<Self::Item, Self::MarkBits> {
                self.as_marked()
            }

            #[inline]
            fn into_marked(self) -> MarkedPtr<Self::Item, Self::MarkBits> {
                Self::into_marked(self)
            }

            #[inline]
            unsafe fn from_marked(marked: MarkedPtr<Self::Item, Self::MarkBits>) -> Self {
                debug_assert!(!marked.is_null());
                Self::from_marked_non_null(MarkedNonNull::new_unchecked(marked))
            }
        }

        impl$($generics)* MarkedPointer for Option<$ptr> {
            type Item = T;
            type MarkBits = N;

            #[inline]
            fn as_marked(&self) -> MarkedPtr<Self::Item, Self::MarkBits> {
                unsafe { mem::transmute_copy(self) }
            }

            #[inline]
            fn into_marked(self) -> MarkedPtr<Self::Item, Self::MarkBits> {
                unsafe { mem::transmute(self) }
            }

            #[inline]
            unsafe fn from_marked(marked: MarkedPtr<Self::Item, Self::MarkBits>) -> Self {
                mem::transmute(marked)
            }
        }

        impl$($generics)* fmt::Debug for $ptr {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                let (ptr, tag) = self.inner.decompose();
                f.debug_struct($name)
                    .field("ptr", &ptr)
                    .field("tag", &tag)
                    .finish()
            }
        }

        impl$($generics)* fmt::Pointer for $ptr {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
            }
        }

        impl$($generics)* PartialEq<MarkedPtr<T, N>> for $ptr {
            #[inline]
            fn eq(&self, other: &MarkedPtr<T, N>) -> bool {
                self.inner.into_marked().eq(other)
            }
        }

        impl$($generics)* PartialOrd<MarkedPtr<T, N>> for $ptr {
            #[inline]
            fn partial_cmp(&self, other: &MarkedPtr<T, N>) -> Option<cmp::Ordering> {
                self.inner.into_marked().partial_cmp(other)
            }
        }

        impl$($generics)* PartialEq<MarkedNonNull<T, N>> for $ptr {
            #[inline]
            fn eq(&self, other: &MarkedNonNull<T, N>) -> bool {
                self.inner.eq(other)
            }
        }

        impl$($generics)* PartialOrd<MarkedNonNull<T, N>> for $ptr {
            #[inline]
            fn partial_cmp(&self, other: &MarkedNonNull<T, N>) -> Option<cmp::Ordering> {
                self.inner.partial_cmp(other)
            }
        }

        impl<'g, T, N: Unsigned, R: Reclaim> PartialEq<Shared<'g, T, N, R>> for $ptr {
            #[inline]
            fn eq(&self, other: &Shared<'g, T, N, R>) -> bool {
                self.inner.eq(&other.inner)
            }
        }

        impl<'g, T, N: Unsigned, R: Reclaim> PartialOrd<Shared<'g, T, N, R>> for $ptr {
            #[inline]
            fn partial_cmp(&self, other: &Shared<'g, T, N, R>) -> Option<cmp::Ordering> {
                self.inner.partial_cmp(&other.inner)
            }
        }

        impl$($generics)* PartialEq<Unlinked<T, N, R>> for $ptr {
            #[inline]
            fn eq(&self, other: &Unlinked<T, N, R>) -> bool {
                self.inner.eq(&other.inner)
            }
        }

        impl$($generics)* PartialOrd<Unlinked<T, N, R>> for $ptr {
            #[inline]
            fn partial_cmp(&self, other: &Unlinked<T, N, R>) -> Option<cmp::Ordering> {
                self.inner.partial_cmp(&other.inner)
            }
        }

        impl$($generics)* PartialEq<Unprotected<T, N, R>> for $ptr {
            #[inline]
            fn eq(&self, other: &Unprotected<T, N, R>) -> bool {
                self.inner.eq(&other.inner)
            }
        }

        impl$($generics)* PartialOrd<Unprotected<T, N, R>> for $ptr {
            #[inline]
            fn partial_cmp(&self, other: &Unprotected<T, N, R>) -> Option<cmp::Ordering> {
                self.inner.partial_cmp(&other.inner)
            }
        }
    };
}

impl_for_non_nullable!("Shared", Shared<'g, T, N, R>, <'g, T, N: Unsigned, R: Reclaim>);
impl_for_non_nullable!("Unlinked", Unlinked<T, N, R>, <T, N: Unsigned, R: Reclaim>);
impl_for_non_nullable!("Unprotected", Unprotected<T, N, R>, <T, N: Unsigned, R: Reclaim>);
