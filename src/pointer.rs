use typenum::Unsigned;

use crate::marked::{MarkedNonNull, MarkedPtr};

////////////////////////////////////////////////////////////////////////////////////////////////////
// Pointer (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Trait for nullable and non-nullable markable pointer types.
pub trait MarkedPointer: Sized + Internal {
    /// Generic pointed-to type
    type Item: Sized;
    /// Number of bits available for storing additional information
    type MarkBits: Unsigned;

    /// TODO: Doc...
    fn tag(&self) -> usize {
        self.as_marked().decompose_tag()
    }

    /// TODO: Doc...
    fn strip_tag(self) -> Self {
        let ptr = self.into_marked().decompose_ptr();
        unsafe { Self::from_marked(MarkedPtr::new(ptr)) }
    }

    /// TODO: Doc...
    fn as_marked(&self) -> MarkedPtr<Self::Item, Self::MarkBits>;

    /// Consumes `self` and returns a marked pointer
    fn into_marked(self) -> MarkedPtr<Self::Item, Self::MarkBits>;

    /// Constructs a `Self` from a raw marked pointer.
    ///
    /// # Safety
    ///
    /// The caller has to ensure that raw is a valid pointer for the respective `Self` type.
    /// If `Self` is nullable, a null pointer is a valid value. Otherwise all values must be valid
    /// pointers.
    unsafe fn from_marked(marked: MarkedPtr<Self::Item, Self::MarkBits>) -> Self;

    /// Constructs a `Self` from a raw non-null marked pointer
    ///
    /// #Safety
    /// The caller has to ensure that raw is a valid pointer for the respective `Self` type.
    unsafe fn from_marked_non_null(marked: MarkedNonNull<Self::Item, Self::MarkBits>) -> Self;
}

macro_rules! impl_marked_pointer {
    () => {
        type Item = T;
        type MarkBits = N;

        #[inline]
        fn as_marked(&self) -> $crate::marked::MarkedPtr<Self::Item, Self::MarkBits> {
            self.inner.into_marked()
        }

        #[inline]
        fn into_marked(self) -> $crate::marked::MarkedPtr<Self::Item, Self::MarkBits> {
            self.inner.into_marked()
        }

        #[inline]
        unsafe fn from_marked(
            marked: $crate::marked::MarkedPtr<Self::Item, Self::MarkBits>
        ) -> Self
        {
            debug_assert!(!marked.is_null());
            Self {
                inner: MarkedNonNull::new_unchecked(marked),
                _marker: PhantomData,
            }
        }

        #[inline]
        unsafe fn from_marked_non_null(
            marked: $crate::marked::MarkedNonNull<Self::Item, Self::MarkBits>
        ) -> Self
        {
            Self {
                inner: marked,
                _marker: PhantomData,
            }
        }
    };
}

macro_rules! impl_marked_pointer_option {
    () => {
        type Item = T;
        type MarkBits = N;

        #[inline]
        fn as_marked(&self) -> $crate::marked::MarkedPtr<Self::Item, Self::MarkBits> {
            unsafe { core::mem::transmute_copy(self) }
        }

        #[inline]
        fn into_marked(self) -> $crate::marked::MarkedPtr<Self::Item, Self::MarkBits> {
            unsafe { core::mem::transmute(self) }
        }

        #[inline]
        unsafe fn from_marked(
            marked: $crate::marked::MarkedPtr<Self::Item, Self::MarkBits>
        ) -> Self
        {
            core::mem::transmute(marked)
        }

        #[inline]
        unsafe fn from_marked_non_null(
            marked: $crate::marked::MarkedNonNull<Self::Item, Self::MarkBits>
        ) -> Self
        {
            core::mem::transmute(Some(marked))
        }
    };
}

macro_rules! impl_inherent {
    () => {
        /// Gets a `None` for an [`Option<Self>`](std::option::Option).
        ///
        /// This is useful for calls to [`store`][store], [`swap`][swap] or
        /// [`compare_exchange_*`][compare_exchange], when a `null` pointer
        /// needs to be inserted.
        /// These methods accept values of various non-nullable pointer types
        /// ([`Shared`][Shared], [`Owned`][Owned], [`Unlinked`][Unlinked] and
        /// [`Unprotected`][Unprotected]) and `Option` thereof as argument.
        /// However, the compiler is usually not able to infer the concrete type,
        /// when a `None` is inserted, and this function is intended for these
        /// cases.
        ///
        /// [store]: crate::atomic::Atomic::store
        /// [swap]: crate::atomic::Atomic::swap
        /// [compare_exchange]: crate::atomic::Atomic::compare_exchange
        /// [Shared]: crate::Shared
        /// [Owned]: crate::Owned
        /// [Unlinked]: crate::Unlinked
        /// [Unprotected]: crate::Unprotected
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
        #[inline]
        pub unsafe fn try_from_marked(marked: $crate::marked::MarkedPtr<T, N>) -> Option<Self> {
            $crate::marked::MarkedNonNull::new(marked).map(|ptr| Self {
                inner: ptr,
                _marker: PhantomData,
            })
        }

        /// TODO: Doc...
        #[inline]
        pub fn into_marked_non_null(self) -> $crate::marked::MarkedNonNull<T, N> {
            self.inner
        }

        /// TODO: Doc...
        #[inline]
        pub fn with_tag(self, tag: usize) -> Self {
            Self {
                inner: MarkedNonNull::compose(self.inner.decompose_non_null(), tag),
                _marker: PhantomData,
            }
        }
    };
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// Internal (trait)
////////////////////////////////////////////////////////////////////////////////////////////////////

/// Prevents implementation of `MarkedPointer` trait (not exported from crate).
pub trait Internal {}
