use core::mem;
use core::ptr::NonNull;
use core::sync::atomic::AtomicPtr;

// implementation modules
mod atomic;
mod non_null;
mod pointer;

////////////////////////////////////////////////////////////////////////////////////////////////////
/// AtomicMarkedPtr
////////////////////////////////////////////////////////////////////////////////////////////////////

/// A raw pointer type which can be safely shared between threads and which can store additional
/// information in its lower (unused) bits.
///
/// This type has the same in-memory representation as a *mut T. It is similar to `AtomicPtr`,
/// except that all its methods involve a `MarkedPtr` instead of *mut T.
pub struct AtomicMarkedPtr<T> {
    inner: AtomicPtr<T>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
/// MarkedPtr
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct MarkedPtr<T> {
    inner: *mut T,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
/// MarkedNonNull
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct MarkedNonNull<T> {
    inner: NonNull<T>,
}

////////////////////////////////////////////////////////////////////////////////////////////////////
/// helper functions
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Composes a marked pointer from a raw (i.e. unmarked) pointer and a tag.
///
/// If the size of the tag exceeds the markable bits of `T` the tag is truncated.
fn compose<T>(ptr: *mut T, tag: usize) -> *mut T {
    ((ptr as usize) | (mark_mask::<T>() & tag)) as *mut _
}

/// Decomposes the integer representation of a marked pointer into a raw pointer and the tag.
const fn decompose<T>(marked: usize) -> (*mut T, usize) {
    (decompose_ptr(marked), decompose_tag::<T>(marked))
}

/// Decomposes the integer representation of a marked pointer into the raw pointer stripped of the
/// tag.
const fn decompose_ptr<T>(marked: usize) -> *mut T {
    (marked & !mark_mask::<T>()) as *mut _
}

/// Decomposes the integer representation of a marked pointer into only the tag.
const fn decompose_tag<T>(marked: usize) -> usize {
    marked & mark_mask::<T>()
}

/// Gets the number of markable lower bits in a well-aligned pointer of type `T`.
const fn mark_bits<T>() -> usize {
    mem::align_of::<T>().trailing_zeros() as usize
}

/// Returns the bitmask of markable lower bits of a type's pointer.
const fn mark_mask<T>() -> usize {
    mem::align_of::<T>() - 1
}

#[cfg(test)]
mod test {
    use std::ptr;

    use crate::marked;

    #[repr(align(1))]
    struct Align1;
    #[repr(align(4))]
    struct Align4;
    #[repr(align(8))]
    struct Align8;
    #[repr(align(64))]
    struct Align64;
    #[repr(align(1024))]
    struct Align1024;
    #[repr(align(4096))]
    struct Align4096;

    #[test]
    fn compose() {
        let ptr = &Align4 as *const _ as *mut Align4;

        assert_eq!(marked::compose::<Align4>(ptr::null_mut(), 0), ptr::null_mut());
        assert_eq!(marked::compose(ptr, 0), ptr);
        assert_eq!(marked::compose(ptr, 0b11), ((ptr as usize) | 0b11) as *mut _);
        assert_eq!(marked::compose(ptr, 0b1111), ((ptr as usize) | 0b11) as *mut _);
        assert_eq!(
            marked::compose::<Align64>(ptr::null_mut(), 0b110101),
            0b110101 as *mut Align64
        );
    }

    #[test]
    fn decompose() {
        let ptr = &Align8 as *const _ as *mut Align8;

        let composed = marked::compose(ptr::null_mut::<Align8>(), 0);
        assert_eq!(marked::decompose(composed as usize), (ptr::null_mut::<Align8>(), 0));
        let composed = marked::compose(ptr::null_mut::<Align8>(), 0b100);
        assert_eq!(marked::decompose(composed as usize), (ptr::null_mut::<Align8>(), 0b100));
        let composed = marked::compose(ptr, 0);
        assert_eq!(marked::decompose(composed as usize), (ptr, 0));
        let composed = marked::compose(ptr, 0b10);
        assert_eq!(marked::decompose(composed as usize), (ptr, 0b10));
        let composed = marked::compose(ptr, 0b1000);
        assert_eq!(marked::decompose(composed as usize), (ptr, 0))
    }

    #[test]
    fn alignments() {
        assert_eq!(0, marked::mark_bits::<Align1>());
        assert_eq!(0, marked::mark_mask::<Align1>());
        assert_eq!(3, marked::mark_bits::<Align8>());
        assert_eq!(0b111, marked::mark_mask::<Align8>());
        assert_eq!(6, marked::mark_bits::<Align64>());
        assert_eq!(0b111111, marked::mark_mask::<Align64>());
        assert_eq!(10, marked::mark_bits::<Align1024>());
        assert_eq!(0b1111111111, marked::mark_mask::<Align1024>());
        assert_eq!(12, marked::mark_bits::<Align4096>());
        assert_eq!(0b111111111111, marked::mark_mask::<Align4096>());
    }
}
