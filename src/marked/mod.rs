use core::marker::PhantomData;
use core::mem;
use core::ptr::NonNull;
use core::sync::atomic::AtomicPtr;

use typenum::Unsigned;

mod atomic;
mod ptr;
mod raw;

////////////////////////////////////////////////////////////////////////////////////////////////////
// AtomicMarkedPtr
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A raw pointer type which can be safely shared between threads and which can store additional
/// information in its lower (unused) bits.
///
/// This type has the same in-memory representation as a *mut T. It is similar to `AtomicPtr`,
/// except that all its methods involve a `MarkedPtr` instead of *mut T.
pub struct AtomicMarkedPtr<T, N> {
    inner: AtomicPtr<T>,
    _marker: PhantomData<N>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// MarkedPtr
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Doc...
#[derive(Eq, Ord)]
pub struct MarkedPtr<T, N> {
    inner: *mut T,
    _marker: PhantomData<N>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// MarkedNonNull
////////////////////////////////////////////////////////////////////////////////////////////////////

/// TODO: Doc...
#[derive(Eq, Ord)]
pub struct MarkedNonNull<T, N> {
    inner: NonNull<T>,
    _marker: PhantomData<N>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// helper functions
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Decomposes the integer representation of a marked pointer into a raw pointer and the tag.
#[inline]
const fn decompose<T>(marked: usize, mark_bits: usize) -> (*mut T, usize) {
    (
        decompose_ptr::<T>(marked, mark_bits),
        decompose_tag::<T>(marked, mark_bits),
    )
}

/// Decomposes the integer representation of a marked pointer into the raw pointer stripped of the
/// tag.
#[inline]
const fn decompose_ptr<T>(marked: usize, mark_bits: usize) -> *mut T {
    (marked & !mark_mask::<T>(mark_bits)) as *mut _
}

/// Decomposes the integer representation of a marked pointer into only the tag.
#[inline]
const fn decompose_tag<T>(marked: usize, mark_bits: usize) -> usize {
    marked & mark_mask::<T>(mark_bits)
}

/// Returns the bitmask of markable lower bits of a type's pointer.
#[inline]
const fn lower_bits<T>() -> usize {
    mem::align_of::<T>().trailing_zeros() as usize
}

/// TODO: Doc...
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

    use crate::align;
    use crate::marked;

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
        assert_eq!(0, marked::lower_bits::<Align1>());
        assert_eq!(1, marked::lower_bits::<Align2>());
        assert_eq!(2, marked::lower_bits::<Align4>());
        assert_eq!(3, marked::lower_bits::<Align8>());
        assert_eq!(4, marked::lower_bits::<Align16>());
        assert_eq!(5, marked::lower_bits::<Align32>());
        assert_eq!(6, marked::lower_bits::<Align64>());
        assert_eq!(10, marked::lower_bits::<Align1024>());
        assert_eq!(12, marked::lower_bits::<Align4096>());
    }

    #[test]
    fn mark_mask() {
        assert_eq!(0b000, marked::mark_mask::<Align8>(U0::USIZE));
        assert_eq!(0b001, marked::mark_mask::<Align8>(U1::USIZE));
        assert_eq!(0b011, marked::mark_mask::<Align8>(U2::USIZE));
        assert_eq!(0b111, marked::mark_mask::<Align8>(U3::USIZE));
    }

    #[test]
    fn compose() {
        let ptr: *mut Align4 = &Align4::new(0) as *const _ as *mut _;

        assert_eq!(
            marked::compose::<Align4, U2>(ptr::null_mut(), 0),
            ptr::null_mut()
        );
        assert_eq!(marked::compose::<_, U2>(ptr, 0), ptr);
        assert_eq!(
            marked::compose::<_, U2>(ptr, 0b11),
            ((ptr as usize) | 0b11) as *mut _
        );
        assert_eq!(
            marked::compose::<_, U2>(ptr, 0b1111),
            ((ptr as usize) | 0b11) as *mut _
        );
        assert_eq!(
            marked::compose::<Align64, U6>(ptr::null_mut(), 0b110101),
            0b110101 as *mut Align64
        );
    }

    #[test]
    fn decompose() {
        let mut aligned = Align8::new(0);

        let composed = marked::compose::<_, U3>(&mut aligned, 0b0);
        assert_eq!(
            marked::decompose(composed as usize, U3::USIZE),
            (&mut aligned as *mut _, 0b0)
        );
        let composed = marked::compose::<_, U3>(&mut aligned, 0b1);
        assert_eq!(
            marked::decompose(composed as usize, U3::USIZE),
            (&mut aligned as *mut _, 0b1)
        );
        let composed = marked::compose::<_, U3>(&mut aligned, 0b10);
        assert_eq!(
            marked::decompose(composed as usize, U3::USIZE),
            (&mut aligned as *mut _, 0b10)
        );
        let composed = marked::compose::<_, U3>(&mut aligned, 0b100);
        assert_eq!(
            marked::decompose(composed as usize, U3::USIZE),
            (&mut aligned as *mut _, 0b100)
        );
        let composed = marked::compose::<_, U3>(&mut aligned, 0b1000);
        assert_eq!(
            marked::decompose(composed as usize, U3::USIZE),
            (&mut aligned as *mut _, 0b0)
        );
    }
}