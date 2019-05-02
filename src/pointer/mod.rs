use core::marker::PhantomData;
use core::mem;
use core::ptr::NonNull;
use core::sync::atomic::AtomicPtr;

use typenum::Unsigned;

mod atomic;
mod marked;
mod non_null;
mod raw;

////////////////////////////////////////////////////////////////////////////////////////////////////
// MarkedPointer (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Trait for nullable and non-nullable *markable* pointer types.
pub trait MarkedPointer: Sized + Internal {
    /// The pointed-to type.
    type Item: Sized;
    /// Number of bits available for tagging.
    type MarkBits: Unsigned;
    /// Number of bits available for tagging.
    const MARK_BITS: usize;

    /// Returns the tag value of the marked pointer.
    fn tag(&self) -> usize;

    /// Consumes `self` and returns the same value but without any tag.
    fn clear_tag(&self) -> Self;

    /// Returns the equivalent raw marked pointer.
    fn as_marked_ptr(&self) -> MarkedPtr<Self::Item, Self::MarkBits>;

    /// Consumes `self` and returns the equivalent raw marked pointer.
    fn into_marked_ptr(self);

    /// Constructs a `Self` from a raw marked pointer.
    ///
    /// # Safety
    ///
    /// The caller has to ensure that raw is a valid pointer for the respective
    /// `Self` type. If `Self` is nullable, a null pointer is a valid value.
    /// Otherwise, all values must be valid pointers.
    unsafe fn from_marked_ptr();

    /// Constructs a `Self` from a raw non-null marked pointer
    ///
    /// # Safety
    ///
    /// The same caveats as with [`from_marked_ptr`][MarkedPointer::from_marked_ptr]
    /// apply as well.
    unsafe fn from_marked_non_null();
}

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
#[derive(Eq, Ord)]
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

/// TODO: Doc...
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Marked<T: NonNullable> {
    Value(T),
    OnlyTag(usize),
    Null,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// InvalidNullError
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub struct InvalidNullError;

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

////////////////////////////////////////////////////////////////////////////////////////////////////
// internal traits
////////////////////////////////////////////////////////////////////////////////////////////////////

/// An internal marker trait for non-nullable pointer types.
pub trait NonNullable {}

/// A marker trait for internal traits.
pub trait Internal {}

#[cfg(test)]
mod test {
    use core::ptr;

    use typenum::{Unsigned, U0, U1, U2, U3, U6};

    use crate::align;

    type Align1 = align::Aligned<u8, align::Alignment1>;
    type Align2 = align::Aligned<u8, align::Alignment2>;
    type Align4 = align::Aligned<u8, align::Alignment4>;
    type Align8 = align::Aligned8<u8>;
    type Align16 = align::Aligned16<u8>;
    type Align32 = align::Aligned32<u8>;
    type Align64 = align::Aligned64<u8>;
    type Align1024 = align::Aligned1024<u8>;
    type Align4096 = align::Aligned4096<u8>;

    #[test]
    fn lower_bits() {
        assert_eq!(0, super::lower_bits::<Align1>());
        assert_eq!(1, super::lower_bits::<Align2>());
        assert_eq!(2, super::lower_bits::<Align4>());
        assert_eq!(3, super::lower_bits::<Align8>());
        assert_eq!(4, super::lower_bits::<Align16>());
        assert_eq!(5, super::lower_bits::<Align32>());
        assert_eq!(6, super::lower_bits::<Align64>());
        assert_eq!(10, super::lower_bits::<Align1024>());
        assert_eq!(12, super::lower_bits::<Align4096>());
    }

    #[test]
    fn mark_mask() {
        assert_eq!(0b000, super::mark_mask::<Align8>(U0::USIZE));
        assert_eq!(0b001, super::mark_mask::<Align8>(U1::USIZE));
        assert_eq!(0b011, super::mark_mask::<Align8>(U2::USIZE));
        assert_eq!(0b111, super::mark_mask::<Align8>(U3::USIZE));
    }

    #[test]
    fn compose() {
        let ptr: *mut Align4 = &Align4::new(0) as *const _ as *mut _;

        assert_eq!(super::compose::<Align4, U2>(ptr::null_mut(), 0), ptr::null_mut());
        assert_eq!(super::compose::<_, U2>(ptr, 0), ptr);
        assert_eq!(super::compose::<_, U2>(ptr, 0b11), ((ptr as usize) | 0b11) as *mut _);
        assert_eq!(super::compose::<_, U2>(ptr, 0b1111), ((ptr as usize) | 0b11) as *mut _);
        assert_eq!(
            super::compose::<Align64, U6>(ptr::null_mut(), 0b110101),
            0b110101 as *mut Align64
        );
    }

    #[test]
    fn decompose() {
        let mut aligned = Align8::new(0);

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
        let ptr: *mut Align4 = ptr::null_mut();
        let marked = super::compose::<_, U1>(ptr, 1);
        assert_eq!(super::decompose::<Align4>(marked as usize, 1), (ptr::null_mut(), 1));
    }
}
