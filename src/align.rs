use core::fmt;
use core::ops::{Deref, DerefMut};

pub type Aligned8<T> = Aligned<T, Alignment8>;
pub type Aligned16<T> = Aligned<T, Alignment16>;
pub type Aligned32<T> = Aligned<T, Alignment32>;
pub type Aligned64<T> = Aligned<T, Alignment64>;
pub type Aligned128<T> = Aligned<T, Alignment128>;
pub type Aligned256<T> = Aligned<T, Alignment256>;
pub type Aligned512<T> = Aligned<T, Alignment512>;
pub type Aligned1024<T> = Aligned<T, Alignment1K>;
pub type Aligned2048<T> = Aligned<T, Alignment2K>;
pub type Aligned4096<T> = Aligned<T, Alignment4K>;

pub type CachePadded<T> = Aligned64<T>;

#[derive(Copy, Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct Aligned<T, A: Alignment> {
    inner: T,
    align: A,
}

impl<T, A: Alignment> Aligned<T, A> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            align: Default::default(),
        }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T, A: Alignment> Deref for Aligned<T, A> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T, A: Alignment> DerefMut for Aligned<T, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// TODO: Doc...
pub trait Alignment: Copy + Clone + Default + fmt::Debug + Eq + Ord + PartialEq + PartialOrd {}

macro_rules! impl_alignment {
    ( $( $id:ident => $align:expr ),+ ) => {
        $(
            #[derive(Copy, Clone, Default, Debug, Eq, Ord, PartialEq, PartialOrd)]
            #[repr(align($align))]
            pub struct $id;
            impl Alignment for $id {}
        )*
    };
}

impl_alignment! {
    Alignment1    => 0x1,
    Alignment2    => 0x2,
    Alignment4    => 0x4,
    Alignment8    => 0x8,
    Alignment16   => 0x10,
    Alignment32   => 0x20,
    Alignment64   => 0x40,
    Alignment128  => 0x80,
    Alignment256  => 0x100,
    Alignment512  => 0x200,
    Alignment1K   => 0x400,
    Alignment2K   => 0x800,
    Alignment4K   => 0x1000,
    Alignment8k   => 0x2000,
    Alignment16k  => 0x4000,
    Alignment32k  => 0x8000,
    Alignment64K  => 0x10000,
    Alignment128K => 0x20000,
    Alignment256K => 0x40000,
    Alignment512K => 0x80000,
    Alignment1M   => 0x100000,
    Alignment2M   => 0x200000,
    Alignment4M   => 0x400000,
    Alignment8M   => 0x800000,
    Alignment16M  => 0x1000000,
    Alignment32M  => 0x2000000,
    Alignment64M  => 0x4000000,
    Alignment128M => 0x8000000,
    Alignment256M => 0x10000000,
    Alignment512M => 0x20000000
}

#[cfg(test)]
mod tests {
    use std::mem;

    use super::*;

    #[test]
    fn alignments() {
        assert_eq!(mem::align_of::<Aligned8<u8>>(), 8);
        assert_eq!(mem::align_of::<Aligned16<u8>>(), 16);
        assert_eq!(mem::align_of::<Aligned32<u8>>(), 32);
        assert_eq!(mem::align_of::<Aligned64<u8>>(), 64);
        assert_eq!(mem::align_of::<Aligned128<u8>>(), 128);
        assert_eq!(mem::align_of::<Aligned256<u8>>(), 256);
        assert_eq!(mem::align_of::<Aligned512<u8>>(), 512);
        assert_eq!(mem::align_of::<Aligned1024<u8>>(), 1024);
        assert_eq!(mem::align_of::<Aligned2048<u8>>(), 2048);
        assert_eq!(mem::align_of::<Aligned4096<u8>>(), 4096);
    }
}