use core::marker::PhantomData;
use core::mem;
use core::ptr::NonNull;
use core::sync::atomic::AtomicPtr;

use typenum::Unsigned;

mod atomic;
mod ptr;
mod raw;

////////////////////////////////////////////////////////////////////////////////////////////////////
/// AtomicMarkedPtr
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A raw pointer type which can be safely shared between threads and which can store additional
/// information in its lower (unused) bits.
///
/// This type has the same in-memory representation as a *mut T. It is similar to `AtomicPtr`,
/// except that all its methods involve a `MarkedPtr` instead of *mut T.
pub struct AtomicMarkedPtr<T, N: Unsigned> {
    inner: AtomicPtr<T>,
    _marker: PhantomData<N>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
/// MarkedPtr
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Eq, Ord)]
pub struct MarkedPtr<T, N: Unsigned> {
    inner: *mut T,
    _marker: PhantomData<N>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
/// MarkedNonNull
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct MarkedNonNull<T, N: Unsigned> {
    inner: NonNull<T>,
    _marker: PhantomData<N>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
/// helper functions
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Decomposes the integer representation of a marked pointer into a raw pointer and the tag.
const fn decompose<T, N: Unsigned>(marked: usize) -> (*mut T, usize) {
    (decompose_ptr::<T, N>(marked), decompose_tag::<T, N>(marked))
}

/// Decomposes the integer representation of a marked pointer into the raw pointer stripped of the
/// tag.
const fn decompose_ptr<T, N: Unsigned>(marked: usize) -> *mut T {
    (marked & !mark_mask::<T, N>()) as *mut _
}

/// Decomposes the integer representation of a marked pointer into only the tag.
const fn decompose_tag<T, N: Unsigned>(marked: usize) -> usize {
    marked & mark_mask::<T, N>()
}

/// Returns the bitmask of markable lower bits of a type's pointer.
const fn lower_bits<T>() -> usize {
    mem::align_of::<T>().trailing_zeros() as usize
}

/// TODO: Doc...
const fn mark_mask<T, N: Unsigned>() -> usize {
    let _assert = lower_bits::<T>() - N::USIZE;
    (1 << N::USIZE) - 1
}

/// Composes a marked pointer from a raw (i.e. unmarked) pointer and a tag.
///
/// If the size of the tag exceeds the markable bits of `T` the tag is truncated.
fn compose<T, N: Unsigned>(ptr: *mut T, tag: usize) -> *mut T {
    debug_assert_eq!(ptr as usize & mark_mask::<T, N>(), 0);
    ((ptr as usize) | (mark_mask::<T, N>() & tag)) as *mut _
}

#[cfg(test)]
mod test {
    use core::ptr;

    use typenum::{U0, U1, U2, U3, U6};

    use crate::marked;

    #[repr(align(1))]
    struct Align1;
    #[repr(align(2))]
    struct Align2;
    #[repr(align(4))]
    struct Align4;
    #[repr(align(8))]
    struct Align8;
    #[repr(align(16))]
    struct Align16;
    #[repr(align(32))]
    struct Align32;
    #[repr(align(64))]
    struct Align64;
    #[repr(align(1024))]
    struct Align1024;
    #[repr(align(4096))]
    struct Align4096;

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
        assert_eq!(0b000, marked::mark_mask::<Align8, U0>());
        assert_eq!(0b001, marked::mark_mask::<Align8, U1>());
        assert_eq!(0b011, marked::mark_mask::<Align8, U2>());
        assert_eq!(0b111, marked::mark_mask::<Align8, U3>());
    }

    #[test]
    fn compose() {
        let ptr: *mut Align4 = &Align4 as *const _ as *mut _;

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
        let mut aligned = Align8;

        let composed = marked::compose::<_, U3>(&mut aligned, 0b0);
        assert_eq!(
            marked::decompose::<_, U3>(composed as usize),
            (&mut aligned as *mut _, 0b0)
        );
        let composed = marked::compose::<_, U3>(&mut aligned, 0b1);
        assert_eq!(
            marked::decompose::<_, U3>(composed as usize),
            (&mut aligned as *mut _, 0b1)
        );
        let composed = marked::compose::<_, U3>(&mut aligned, 0b10);
        assert_eq!(
            marked::decompose::<_, U3>(composed as usize),
            (&mut aligned as *mut _, 0b10)
        );
        let composed = marked::compose::<_, U3>(&mut aligned, 0b100);
        assert_eq!(
            marked::decompose::<_, U3>(composed as usize),
            (&mut aligned as *mut _, 0b100)
        );
        let composed = marked::compose::<_, U3>(&mut aligned, 0b1000);
        assert_eq!(
            marked::decompose::<_, U3>(composed as usize),
            (&mut aligned as *mut _, 0b0)
        );
    }

    /*#[test]
    fn decompose() {
        let ptr = &Align8 as *const _ as *mut Align8;

        let composed = marked::compose(ptr::null_mut::<Align8>(), 0);
        assert_eq!(marked::decompose::<_, N>(composed as usize), (ptr::null_mut::<Align8>(), 0));
        let composed = marked::compose(ptr::null_mut::<Align8>(), 0b100);
        assert_eq!(marked::decompose(composed as usize), (ptr::null_mut::<Align8>(), 0b100));
        let composed = marked::compose(ptr, 0);
        assert_eq!(marked::decompose(composed as usize), (ptr, 0));
        let composed = marked::compose(ptr, 0b10);
        assert_eq!(marked::decompose(composed as usize), (ptr, 0b10));
        let composed = marked::compose(ptr, 0b1000);
        assert_eq!(marked::decompose(composed as usize), (ptr, 0))
    }*/

    /*#[test]
    fn alignments() {
        assert_eq!(0, marked::mark_bits::<Align1>());
        assert_eq!(0, marked::lower_bits::<Align1>());
        assert_eq!(3, marked::mark_bits::<Align8>());
        assert_eq!(0b111, marked::lower_bits::<Align8>());
        assert_eq!(6, marked::mark_bits::<Align64>());
        assert_eq!(0b111111, marked::lower_bits::<Align64>());
        assert_eq!(10, marked::mark_bits::<Align1024>());
        assert_eq!(0b1111111111, marked::lower_bits::<Align1024>());
        assert_eq!(12, marked::mark_bits::<Align4096>());
        assert_eq!(0b111111111111, marked::lower_bits::<Align4096>());
    }*/
}
