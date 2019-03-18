use core::cmp::{self, PartialEq, PartialOrd};
use core::fmt;

use crate::marked::{MarkedNonNull, MarkedPtr};
use crate::{Reclaim, Shared, Unlinked, Unprotected, Unsigned};

////////////////////////////////////////////////////////////////////////////////////////////////////
/// Shared trait impls
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<'g, T, N: Unsigned, R: Reclaim> fmt::Debug for Shared<'g, T, N, R> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (ptr, tag) = self.inner.decompose();
        f.debug_struct("Shared")
        .field("ptr", &ptr)
            .field("tag", &tag)
            .finish()
    }
}

impl<'g, T, N: Unsigned, R: Reclaim> fmt::Pointer for Shared<'g, T, N, R> {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
    }
}

impl<'g, T, N: Unsigned, R: Reclaim> PartialEq for Shared<'g, T, N, R> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<'g, T, N: Unsigned, R: Reclaim> PartialOrd for Shared<'g, T, N, R> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl<'g, T, N: Unsigned, R: Reclaim> PartialEq<MarkedPtr<T, N>> for Shared<'g, T, N, R> {
    #[inline]
    fn eq(&self, other: &MarkedPtr<T, N>) -> bool {
        self.inner.into_marked().eq(other)
    }
}

impl<'g, T, N: Unsigned, R: Reclaim> PartialOrd<MarkedPtr<T, N>> for Shared<'g, T, N, R> {
    #[inline]
    fn partial_cmp(&self, other: &MarkedPtr<T, N>) -> Option<cmp::Ordering> {
        self.inner.into_marked().partial_cmp(other)
    }
}

impl<'g, T, N: Unsigned, R: Reclaim> PartialEq<MarkedNonNull<T, N>> for Shared<'g, T, N, R> {
    #[inline]
    fn eq(&self, other: &MarkedNonNull<T, N>) -> bool {
        self.inner.eq(other)
    }
}

impl<'g, T, N: Unsigned, R: Reclaim> PartialOrd<MarkedNonNull<T, N>> for Shared<'g, T, N, R> {
    #[inline]
    fn partial_cmp(&self, other: &MarkedNonNull<T, N>) -> Option<cmp::Ordering> {
        self.inner.partial_cmp(other)
    }
}

impl<'g, T, N: Unsigned, R: Reclaim> PartialEq<Unlinked<T, N, R>> for Shared<'g, T, N, R> {
    #[inline]
    fn eq(&self, other: &Unlinked<T, N, R>) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<'g, T, N: Unsigned, R: Reclaim> PartialOrd<Unlinked<T, N, R>> for Shared<'g, T, N, R> {
    #[inline]
    fn partial_cmp(&self, other: &Unlinked<T, N, R>) -> Option<cmp::Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl<'g, T, N: Unsigned, R: Reclaim> PartialEq<Unprotected<T, N, R>> for Shared<'g, T, N, R> {
    #[inline]
    fn eq(&self, other: &Unprotected<T, N, R>) -> bool {
        self.inner.eq(&other.inner)
    }
}

impl<'g, T, N: Unsigned, R: Reclaim> PartialOrd<Unprotected<T, N, R>> for Shared<'g, T, N, R> {
    #[inline]
    fn partial_cmp(&self, other: &Unprotected<T, N, R>) -> Option<cmp::Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
/// Unlinked & Unprotected trait impls
////////////////////////////////////////////////////////////////////////////////////////////////////

macro_rules! impl_traits {
    ($ptr:ty, $name:tt) => {
        impl<T, N: Unsigned, R: Reclaim> fmt::Debug for $ptr {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                let (ptr, tag) = self.inner.decompose();
                f.debug_struct($name)
                    .field("ptr", &ptr)
                    .field("tag", &tag)
                    .finish()
            }
        }

        impl<T, N: Unsigned, R: Reclaim> fmt::Pointer for $ptr {
            #[inline]
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                fmt::Pointer::fmt(&self.inner.decompose_ptr(), f)
            }
        }

        impl<T, N: Unsigned, R: Reclaim> PartialEq<MarkedPtr<T, N>> for $ptr {
            #[inline]
            fn eq(&self, other: &MarkedPtr<T, N>) -> bool {
                self.inner.into_marked().eq(other)
            }
        }

        impl<T, N: Unsigned, R: Reclaim> PartialOrd<MarkedPtr<T, N>> for $ptr {
            #[inline]
            fn partial_cmp(&self, other: &MarkedPtr<T, N>) -> Option<cmp::Ordering> {
                self.inner.into_marked().partial_cmp(other)
            }
        }

        impl<T, N: Unsigned, R: Reclaim> PartialEq<MarkedNonNull<T, N>> for $ptr {
            #[inline]
            fn eq(&self, other: &MarkedNonNull<T, N>) -> bool {
                self.inner.eq(other)
            }
        }

        impl<T, N: Unsigned, R: Reclaim> PartialOrd<MarkedNonNull<T, N>> for $ptr {
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

        impl<T, N: Unsigned, R: Reclaim> PartialEq<Unlinked<T, N, R>> for $ptr {
            #[inline]
            fn eq(&self, other: &Unlinked<T, N, R>) -> bool {
                self.inner.eq(&other.inner)
            }
        }

        impl<T, N: Unsigned, R: Reclaim> PartialOrd<Unlinked<T, N, R>> for $ptr {
            #[inline]
            fn partial_cmp(&self, other: &Unlinked<T, N, R>) -> Option<cmp::Ordering> {
                self.inner.partial_cmp(&other.inner)
            }
        }

        impl<T, N: Unsigned, R: Reclaim> PartialEq<Unprotected<T, N, R>> for $ptr {
            #[inline]
            fn eq(&self, other: &Unprotected<T, N, R>) -> bool {
                self.inner.eq(&other.inner)
            }
        }

        impl<T, N: Unsigned, R: Reclaim> PartialOrd<Unprotected<T, N, R>> for $ptr {
            #[inline]
            fn partial_cmp(&self, other: &Unprotected<T, N, R>) -> Option<cmp::Ordering> {
                self.inner.partial_cmp(&other.inner)
            }
        }
    };
}

impl_traits!(Unlinked<T, N, R>, "Unlinked");
impl_traits!(Unprotected<T, N, R>, "Unprotected");