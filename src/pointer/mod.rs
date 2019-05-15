use core::fmt;
use core::marker::PhantomData;
use core::mem;
use core::ptr::{self, NonNull};
use core::sync::atomic::AtomicPtr;

use typenum::Unsigned;

mod atomic;
mod marked;
mod non_null;
mod raw;

use self::Marked::{Value, Null};

////////////////////////////////////////////////////////////////////////////////////////////////////
// MarkedPtr
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A raw, unsafe pointer type like `*mut T` in which up to `N` of the pointer's
/// lower bits can be used to store additional information (the *tag*).
///
/// Note, that the upper bound for `N` is dictated by the alignment of `T`.
/// A type with an alignment of `8` (e.g. a `usize` on 64-bit architectures) can
/// have up to `3` mark bits.
/// Attempts to use types with insufficient alignment will result in a compile-
/// time error.
pub struct MarkedPtr<T, N> {
    inner: *mut T,
    _marker: PhantomData<N>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// MarkedNonNull
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A non-nullable marked raw pointer type like [`NonNull`](std::ptr::NonNull).
///
/// Note, that unlike [`MarkedPtr`][MarkedPtr] this also **excludes** marked
/// null-pointers.
pub struct MarkedNonNull<T, N> {
    inner: NonNull<T>,
    _marker: PhantomData<N>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// AtomicMarkedPtr
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A raw pointer type which can be safely shared between threads, which
/// can store additional information in its lower (unused) bits.
///
/// This type has the same in-memory representation as a *mut T. It is mostly
/// identical to [`AtomicPtr`][atomic], except that all of its methods involve
/// a [`MarkedPtr`][marked] instead of `*mut T`.
///
/// [atomic]: std::sync::atomic::AtomicPtr
/// [marked]: MarkedPtr
pub struct AtomicMarkedPtr<T, N> {
    inner: AtomicPtr<T>,
    _marker: PhantomData<N>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Marked
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A value that represents the possible states of a nullable marked pointer.
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Marked<T: NonNullable> {
    /// A marked, non-nullable pointer or reference value.
    Value(T),
    /// A null pointer that may be marked, in which case the `usize` is
    /// non-zero.
    Null(usize),
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// InvalidNullError
////////////////////////////////////////////////////////////////////////////////////////////////////

/// An error type for representing failed conversions from nullable to
/// non-nullable pointer types.
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct InvalidNullError;

impl fmt::Display for InvalidNullError {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "failed conversion of null pointer to non-nullable type")
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// MarkedPointer (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Trait for nullable and non-nullable *markable* pointer types.
pub trait MarkedPointer: Sized + Internal {
    /// The pointer type.
    type Pointer: NonNullable<Item = Self::Item, MarkBits = Self::MarkBits>;
    /// The pointed-to type.
    type Item: Sized;
    /// Number of bits available for tagging.
    type MarkBits: Unsigned;

    /// Returns the equivalent raw marked pointer.
    ///
    /// # Note
    ///
    /// For types like [`Shared`][crate::Owned], [`Shared`][crate::Shared] and
    /// [`Unlinked`][crate::Unlinked], which implement [`Deref`][core::ops::Deref]
    /// this method may conflict with inherent methods of the dereferenced type
    /// and goes against Rust's API guidelines.
    /// This is a deliberate trade-off for enabling more ergonomic usage of
    /// this method
    fn as_marked_ptr(&self) -> MarkedPtr<Self::Item, Self::MarkBits>;

    /// Consumes `self` and returns the equivalent raw marked pointer.
    ///
    /// # Note
    ///
    /// For types like [`Shared`][crate::Owned], [`Shared`][crate::Shared] and
    /// [`Unlinked`][crate::Unlinked], which implement [`Deref`][core::ops::Deref]
    /// this method may conflict with inherent methods of the dereferenced type
    /// and goes against Rust's API guidelines.
    /// This is a deliberate trade-off for enabling more ergonomic usage of
    /// this method
    fn into_marked_ptr(self) -> MarkedPtr<Self::Item, Self::MarkBits>;

    /// Consumes the `Self` and returns the same value with the specified tag
    /// wrapped in a [`Marked`].
    fn marked(_: Self, tag: usize) -> Marked<Self::Pointer>;

    /// Consumes the `Self` and returns the same value but without any tag.
    fn unmarked(_: Self) -> Self;

    /// Constructs a `Self` from a raw marked pointer.
    ///
    /// # Safety
    ///
    /// The caller has to ensure that raw is a valid pointer for the respective
    /// `Self` type. If `Self` is nullable, a null pointer is a valid value.
    /// Otherwise, all values must be valid pointers.
    unsafe fn from_marked_ptr(marked: MarkedPtr<Self::Item, Self::MarkBits>) -> Self;

    /// Constructs a `Self` from a raw non-null marked pointer
    ///
    /// # Safety
    ///
    /// The same caveats as with [`from_marked_ptr`][MarkedPointer::from_marked_ptr]
    /// apply as well.
    unsafe fn from_marked_non_null(marked: MarkedNonNull<Self::Item, Self::MarkBits>) -> Self;
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// blanket implementations
////////////////////////////////////////////////////////////////////////////////////////////////////

impl<U, T, N: Unsigned> MarkedPointer for Option<U>
where
    U: MarkedPointer<Pointer = U, Item = T, MarkBits = N> + NonNullable<Item = T, MarkBits = N>
{
    type Pointer = U;
    type Item = T;
    type MarkBits = N;

    #[inline]
    fn as_marked_ptr(&self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        match self {
            Some(ptr) => Self::Pointer::as_marked_ptr(ptr),
            None => MarkedPtr::null()
        }
    }

    #[inline]
    fn into_marked_ptr(self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        match self {
            Some(ptr) => Self::Pointer::into_marked_ptr(ptr),
            None => MarkedPtr::null()
        }
    }

    #[inline]
    fn marked(opt: Self, tag: usize) -> Marked<Self::Pointer> {
        match opt {
            Some(ptr) => Self::Pointer::marked(ptr, tag),
            None => Null(tag),
        }
    }

    #[inline]
    fn unmarked(opt: Self) -> Self {
        opt.map(|ptr| Self::Pointer::unmarked(ptr))
    }

    #[inline]
    unsafe fn from_marked_ptr(marked: MarkedPtr<Self::Item, Self::MarkBits>) -> Self {
        match !marked.is_null() {
            true => Some(Self::Pointer::from_marked_non_null(MarkedNonNull::new_unchecked(marked))),
            false => None
        }
    }

    #[inline]
    unsafe fn from_marked_non_null(marked: MarkedNonNull<Self::Item, Self::MarkBits>) -> Self {
        Some(Self::Pointer::from_marked_non_null(marked))
    }
}

impl<U, T, N: Unsigned> MarkedPointer for Marked<U>
where
    U: MarkedPointer<Pointer = U, Item = T, MarkBits = N> + NonNullable<Item = T, MarkBits = N>
{
    type Pointer = U;
    type Item = T;
    type MarkBits = N;

    #[inline]
    fn as_marked_ptr(&self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        match self {
            Value(ptr) => Self::Pointer::as_marked_ptr(ptr),
            Null(tag) => MarkedPtr::compose(ptr::null_mut(), *tag)
        }
    }

    #[inline]
    fn into_marked_ptr(self) -> MarkedPtr<Self::Item, Self::MarkBits> {
        match self {
            Value(ptr) => Self::Pointer::into_marked_ptr(ptr),
            Null(tag) => MarkedPtr::compose(ptr::null_mut(), tag)
        }
    }

    #[inline]
    fn marked(marked: Self, tag: usize) -> Marked<Self::Pointer> {
        match marked {
            Value(ptr) => Self::Pointer::marked(ptr, tag),
            Null(_) => Null(tag)
        }
    }

    #[inline]
    fn unmarked(marked: Self) -> Self {
        match marked {
            Value(ptr) => Value(Self::Pointer::unmarked(ptr)),
            Null(_) => Null(0),
        }
    }

    #[inline]
    unsafe fn from_marked_ptr(marked: MarkedPtr<Self::Item, Self::MarkBits>) -> Self {
        MarkedNonNull::new(marked).map(|ptr| Self::Pointer::from_marked_non_null(ptr))
    }

    #[inline]
    unsafe fn from_marked_non_null(marked: MarkedNonNull<Self::Item, Self::MarkBits>) -> Self {
        Value(Self::Pointer::from_marked_non_null(marked))
    }
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// internal traits
////////////////////////////////////////////////////////////////////////////////////////////////////

/// An sealed (internal) marker trait for non-nullable pointer types.
pub trait NonNullable: Sized {
    /// The pointed-to type.
    type Item: Sized;
    /// Number of bits available for tagging.
    type MarkBits: Unsigned;

    /// Converts the given `Self` into a equivalent marked non-null pointer.
    ///
    /// # Note
    ///
    /// For types like [`Shared`][crate::Owned], [`Shared`][crate::Shared] and
    /// [`Unlinked`][crate::Unlinked], which implement [`Deref`][core::ops::Deref]
    /// this method may conflict with inherent methods of the dereferenced type
    /// and goes against Rust's API guidelines.
    /// This is a deliberate trade-off for enabling more ergonomic usage of
    /// this method
    fn into_marked_non_null(self) -> MarkedNonNull<Self::Item, Self::MarkBits>;
}

impl<'a, T> NonNullable for &'a T {
    type Item = T;
    type MarkBits = typenum::U0;

    #[inline]
    fn into_marked_non_null(self) -> MarkedNonNull<Self::Item, Self::MarkBits> {
        MarkedNonNull::from(self)
    }
}

impl<'a, T> NonNullable for &'a mut T {
    type Item = T;
    type MarkBits = typenum::U0;

    #[inline]
    fn into_marked_non_null(self) -> MarkedNonNull<Self::Item, Self::MarkBits> {
        MarkedNonNull::from(self)
    }
}

/// A marker trait for internal traits.
pub trait Internal {}

impl<U, T, N: Unsigned> Internal for Option<U>
where
    U: MarkedPointer<Item = T, MarkBits = N> + NonNullable<Item = T, MarkBits = N> {}

impl<U, T, N: Unsigned> Internal for Marked<U>
where
    U: MarkedPointer<Item = T, MarkBits = N> + NonNullable<Item = T, MarkBits = N> {}

////////////////////////////////////////////////////////////////////////////////////////////////////
// helper functions
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Decomposes the integer representation of a marked pointer into a
/// raw pointer and its tag.
#[inline]
const fn decompose<T>(marked: usize, mark_bits: usize) -> (*mut T, usize) {
    (decompose_ptr::<T>(marked, mark_bits), decompose_tag::<T>(marked, mark_bits))
}

/// Decomposes the integer representation of a marked pointer into
/// a raw pointer stripped of its tag.
#[inline]
const fn decompose_ptr<T>(marked: usize, mark_bits: usize) -> *mut T {
    (marked & !mark_mask::<T>(mark_bits)) as *mut _
}

/// Decomposes the integer representation of a marked pointer into
/// *only* the tag.
#[inline]
const fn decompose_tag<T>(marked: usize, mark_bits: usize) -> usize {
    marked & mark_mask::<T>(mark_bits)
}

/// Gets the number of unused (markable) lower bits in a pointer for
/// type `T`.
#[inline]
const fn lower_bits<T>() -> usize {
    mem::align_of::<T>().trailing_zeros() as usize
}

/// Gets the integer representation for the bitmask of markable lower
/// bits of a pointer for type `T`.
#[deny(const_err)]
#[inline]
const fn mark_mask<T>(mark_bits: usize) -> usize {
    let _assert_sufficient_alignment = lower_bits::<T>() - mark_bits;
    (1 << mark_bits) - 1
}

/// Composes a marked pointer from a raw (i.e. unmarked) pointer and a tag.
///
/// If the size of the tag exceeds the markable bits of `T` the tag is truncated.
#[inline]
fn compose<T, N: Unsigned>(ptr: *mut T, tag: usize) -> *mut T {
    debug_assert_eq!(ptr as usize & mark_mask::<T>(N::USIZE), 0);
    ((ptr as usize) | (mark_mask::<T>(N::USIZE) & tag)) as *mut _
}

#[cfg(test)]
mod test {
    use core::ptr;

    use typenum::{Unsigned, U0, U1, U2, U3, U6};

    use crate::align::{
        Aligned1, Aligned1024, Aligned16, Aligned2, Aligned32, Aligned4, Aligned4096, Aligned64,
        Aligned8,
    };

    #[test]
    fn lower_bits() {
        assert_eq!(0, super::lower_bits::<Aligned1<u8>>());
        assert_eq!(1, super::lower_bits::<Aligned2<u8>>());
        assert_eq!(2, super::lower_bits::<Aligned4<u8>>());
        assert_eq!(3, super::lower_bits::<Aligned8<u8>>());
        assert_eq!(4, super::lower_bits::<Aligned16<u8>>());
        assert_eq!(5, super::lower_bits::<Aligned32<u8>>());
        assert_eq!(6, super::lower_bits::<Aligned64<u8>>());
        assert_eq!(10, super::lower_bits::<Aligned1024<u8>>());
        assert_eq!(12, super::lower_bits::<Aligned4096<u8>>());
    }

    #[test]
    fn mark_mask() {
        assert_eq!(0b000, super::mark_mask::<Aligned8<u8>>(U0::USIZE));
        assert_eq!(0b001, super::mark_mask::<Aligned8<u8>>(U1::USIZE));
        assert_eq!(0b011, super::mark_mask::<Aligned8<u8>>(U2::USIZE));
        assert_eq!(0b111, super::mark_mask::<Aligned8<u8>>(U3::USIZE));
    }

    #[test]
    fn compose() {
        let reference = &mut Aligned4(0u8);
        let ptr = reference as *mut _ as usize;

        assert_eq!(super::compose::<Aligned8<u8>, U2>(ptr::null_mut(), 0), ptr::null_mut());
        assert_eq!(super::compose::<_, U2>(reference, 0), ptr as *mut _);
        assert_eq!(super::compose::<_, U2>(reference, 0b11), (ptr | 0b11) as *mut _);
        assert_eq!(super::compose::<_, U2>(reference, 0b1111), (ptr | 0b11) as *mut _);
        assert_eq!(
            super::compose::<Aligned64<u8>, U6>(ptr::null_mut(), 0b110101),
            0b110101 as *mut Aligned64<u8>
        );
    }

    #[test]
    fn decompose() {
        let mut aligned = Aligned8(0);

        let composed = super::compose::<_, U3>(&mut aligned, 0b0);
        assert_eq!(super::decompose(composed as usize, U3::USIZE), (&mut aligned as *mut _, 0b0));
        let composed = super::compose::<_, U3>(&mut aligned, 0b1);
        assert_eq!(super::decompose(composed as usize, U3::USIZE), (&mut aligned as *mut _, 0b1));
        let composed = super::compose::<_, U3>(&mut aligned, 0b10);
        assert_eq!(super::decompose(composed as usize, U3::USIZE), (&mut aligned as *mut _, 0b10));
        let composed = super::compose::<_, U3>(&mut aligned, 0b100);
        assert_eq!(super::decompose(composed as usize, U3::USIZE), (&mut aligned as *mut _, 0b100));
        let composed = super::compose::<_, U3>(&mut aligned, 0b1000);
        assert_eq!(super::decompose(composed as usize, U3::USIZE), (&mut aligned as *mut _, 0b0));
    }

    #[test]
    fn marked_null() {
        let ptr: *mut Aligned4<u8> = ptr::null_mut();
        let marked = super::compose::<_, U1>(ptr, 1);
        assert_eq!(super::decompose::<Aligned4<u8>>(marked as usize, 1), (ptr::null_mut(), 1));
    }
}
