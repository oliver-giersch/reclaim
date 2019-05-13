macro_rules! impl_trait {
    () => {
        type Item = T;
        type Pointer = Self;
        type MarkBits = N;
        const MARK_BITS: usize = N::USIZE;

        #[inline]
        fn as_marked_ptr(&self) -> crate::pointer::MarkedPtr<T, N> {
            self.inner.into_marked_ptr()
        }

        #[inline]
        fn decompose_tag(&self) -> usize {
            self.as_marked_ptr().decompose_tag()
        }

        #[inline]
        fn clear_tag(self) -> Self {
            let ptr = self.into_marked_ptr().decompose_ptr();
            unsafe { Self::from_marked_ptr(crate::pointer::MarkedPtr::new(ptr)) }
        }

        #[inline]
        fn marked_with_tag(self, tag: usize) -> crate::pointer::Marked<Self::Pointer> {
            Marked::Value(self.with_tag(tag))
        }

        #[inline]
        fn into_marked_ptr(self) -> crate::pointer::MarkedPtr<T, N> {
            self.inner.into_marked_ptr()
        }

        #[inline]
        unsafe fn from_marked_ptr(marked: crate::pointer::MarkedPtr<T, N>) -> Self {
            debug_assert!(!marked.is_null());
            Self {
                inner: crate::pointer::MarkedNonNull::new_unchecked(marked),
                _marker: core::marker::PhantomData,
            }
        }

        #[inline]
        unsafe fn from_marked_non_null(marked: crate::pointer::MarkedNonNull<T, N>) -> Self {
            Self {
                inner: marked,
                _marker: core::marker::PhantomData,
            }
        }
    };
}

macro_rules! impl_trait_option {
    ($pointer:ty) => {
        type Item = T;
        type Pointer = $pointer;
        type MarkBits = N;
        const MARK_BITS: usize = N::USIZE;

        #[inline]
        fn as_marked_ptr(&self) -> crate::pointer::MarkedPtr<T, N> {
            match *self {
                Some(ref ptr) => ptr.as_marked_ptr(),
                None => crate::pointer::MarkedPtr::null(),
            }
        }

        #[inline]
        fn decompose_tag(&self) -> usize {
            match *self {
                Some(ref ptr) => ptr.decompose_tag(),
                None => 0,
            }
        }

        #[inline]
        fn clear_tag(self) -> Self {
            self.map(|ptr| ptr.with_tag(0))
        }

        #[inline]
        fn marked_with_tag(self, tag: usize) -> crate::pointer::Marked<Self::Pointer> {
            match self {
                Some(ptr) => Marked::Value(ptr.with_tag(tag)),
                None => Marked::Null(tag),
            }
        }

        #[inline]
        fn into_marked_ptr(self) -> crate::pointer::MarkedPtr<T, N> {
            match self {
                Some(ptr) => ptr.into_marked_ptr(),
                None => crate::pointer::MarkedPtr::null(),
            }
        }

        #[inline]
        unsafe fn from_marked_ptr(marked: crate::pointer::MarkedPtr<T, N>) -> Self {
            core::mem::transmute(marked)
        }

        #[inline]
        unsafe fn from_marked_non_null(marked: crate::pointer::MarkedNonNull<T, N>) -> Self {
            Some(Self::Pointer::from_marked_non_null(marked))
        }
    };
}

macro_rules! impl_trait_marked {
    ($pointer:ty) => {
        type Item = T;
        type Pointer = $pointer;
        type MarkBits = N;
        const MARK_BITS: usize = N::USIZE;

        #[inline]
        fn as_marked_ptr(&self) -> crate::pointer::MarkedPtr<T, N> {
            match *self {
                Marked::Value(ref ptr) => ptr.as_marked_ptr(),
                Marked::Null(ref tag) => crate::pointer::MarkedPtr::compose(
                    core::ptr::null_mut(),
                    *tag
                ),
            }
        }

        #[inline]
        fn decompose_tag(&self) -> usize {
            match *self {
                Marked::Value(ref ptr) => ptr.decompose_tag(),
                Marked::Null(ref tag) => *tag,
            }
        }

        #[inline]
        fn clear_tag(self) -> Self {
            match self {
                Marked::Value(ptr) => Marked::Value(ptr.with_tag(0)),
                Marked::Null(_) => Marked::Null(0),
            }
        }

        #[inline]
        fn marked_with_tag(self, tag: usize) -> crate::pointer::Marked<Self::Pointer> {
            match self {
                Marked::Value(ptr) => Marked::Value(ptr.with_tag(tag)),
                Marked::Null(_) => Marked::Null(tag),
            }
        }

        #[inline]
        fn into_marked_ptr(self) -> crate::pointer::MarkedPtr<T, N> {
            match self {
                Marked::Value(ptr) => ptr.into_marked_ptr(),
                Marked::Null(tag) => crate::pointer::MarkedPtr::compose(
                    core::ptr::null_mut(),
                    tag
                ),
            }
        }

        #[inline]
        unsafe fn from_marked_ptr(marked: crate::pointer::MarkedPtr<T, N>) -> Self {
            crate::pointer::MarkedNonNull::new(marked)
                .map(|ptr| Self::Pointer::from_marked_non_null(ptr))
        }

        #[inline]
        unsafe fn from_marked_non_null(marked: crate::pointer::MarkedNonNull<T, N>) -> Self {
            Marked::Value(Self::Pointer::from_marked_non_null(marked))
        }
    };
}

macro_rules! impl_inherent {
    () => {
        /// Creates a `None` variant for an
        /// [`Option<Self>`](core::option::Option).
        ///
        /// This is useful for calls to [`store`][store], [`swap`][swap] or
        /// [`compare_exchange_*`][compare_exchange], when a `null` pointer
        /// needs to be inserted.
        /// These methods accept values of various non-nullable pointer types
        /// ([`Shared`][Shared], [`Owned`][Owned], [`Unlinked`][Unlinked] and
        /// [`Unprotected`][Unprotected]) and `Option` thereof as argument.
        /// However, the compiler is usually not able to infer the concrete
        /// type, when a `None` is inserted, and this function is intended for
        /// these cases.
        ///
        /// [store]: crate::Atomic::store
        /// [swap]: crate::Atomic::swap
        /// [compare_exchange]: crate::Atomic::compare_exchange
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
        /// assert_eq!(swap.as_ref(), &1);
        /// unsafe { swap.retire() }; // leaks memory
        /// ```
        #[inline]
        pub fn none() -> Option<Self> {
            None
        }

        /// Creates a `Null` variant for a [`Marked<Self>`][crate::Marked]
        #[inline]
        pub fn null() -> crate::pointer::Marked<Self> {
            Marked::Null(0)
        }

        /// Creates an `OnlyTag` variant for a [`Marked<Self>`][crate::Marked]
        /// with the given `tag`.
        #[inline]
        pub fn null_with_tag(tag: usize) -> crate::pointer::Marked<Self> {
            Marked::Null(tag)
        }

        /// Creates a new [`Option<Self>`](core::option::Option) from a marked
        /// pointer.
        ///
        /// # Safety
        ///
        /// The caller has to ensure `marked` is either `null` or a valid
        /// pointer to a heap allocated value of the appropriate `Self` type.
        #[inline]
        pub unsafe fn try_from_marked(
            marked: crate::pointer::MarkedPtr<T, N>
        ) -> crate::pointer::Marked<Self> {
            crate::pointer::MarkedNonNull::new(marked).map(|ptr| Self::from_marked_non_null(ptr))
        }

        /// Consumes `self` and returns the raw inner non-null marked pointer.
        #[inline]
        pub fn into_marked_non_null(self) -> crate::pointer::MarkedNonNull<T, N> {
            self.inner
        }

        /// Consumes `self` and returns the same value but with the
        /// specified `tag`.
        #[inline]
        pub fn with_tag(self, tag: usize) -> Self {
            debug_assert!(!self.as_marked_ptr().is_null());
            Self {
                inner: MarkedNonNull::compose(self.inner.decompose_non_null(), tag),
                _marker: PhantomData,
            }
        }
    };
}
