macro_rules! impl_trait {
    () => {
        type Item = T;
        type MarkBits = N;
        const MARK_BITS: usize = N::USIZE;

        #[inline]
        fn as_marked_ptr(&self) -> crate::pointer::MarkedPtr<T, N> {
            self.inner.into_marked_ptr()
        }

        #[inline]
        fn tag(&self) -> usize {
            self.as_marked_ptr().decompose_tag()
        }

        #[inline]
        fn clear_tag(self) -> Self {
            let ptr = self.into_marked_ptr().decompose_ptr();
            unsafe { Self::from_marked_ptr(crate::pointer::MarkedPtr::new(ptr)) }
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
    () => {
        type Item = T;
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
        fn tag(&self) -> usize {
            match *self {
                Some(ref ptr) => ptr.tag(),
                None => 0,
            }
        }

        #[inline]
        fn clear_tag(self) -> Self {
            self.map(|ptr| ptr.with_tag(0))
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
            Some(core::mem::transmute(marked))
        }
    };
}

macro_rules! impl_trait_marked {
    ($pointer:ident) => {
        type Item = T;
        type MarkBits = N;
        const MARK_BITS: usize = N::USIZE;

        #[inline]
        fn as_marked_ptr(&self) -> crate::pointer::MarkedPtr<T, N> {
            match *self {
                Marked::Pointer(ref ptr) => ptr.as_marked_ptr(),
                Marked::OnlyTag(ref tag) => crate::pointer::MarkedPtr::compose(
                    core::ptr::null_mut(),
                    *tag
                ),
                Marked::Null => crate::pointer::MarkedPtr::null(),
            }
        }

        #[inline]
        fn tag(&self) -> usize {
            match *self {
                Marked::Pointer(ref ptr) => ptr.tag(),
                Marked::OnlyTag(ref tag) => *tag,
                Marked::Null => 0,
            }
        }

        #[inline]
        fn clear_tag(self) -> Self {
            match self {
                Marked::Pointer(ptr) => Marked::Pointer(ptr.with_tag(0)),
                _ => Marked::Null,
            }
        }

        #[inline]
        fn into_marked_ptr(self) -> crate::pointer::MarkedPtr<T, N> {
            match self {
                Marked::Pointer(ptr) => ptr.into_marked_ptr(),
                Marked::OnlyTag(tag) => crate::pointer::MarkedPtr::compose(
                    core::ptr::null_mut(),
                    tag
                ),
                Marked::Null => crate::pointer::MarkedPtr::null(),
            }
        }

        #[inline]
        unsafe fn from_marked_ptr(marked: crate::pointer::MarkedPtr<T, N>) -> Self {
            crate::pointer::MarkedNonNull::new(marked).map(|ptr| $pointer::from_marked_non_null(ptr))
        }

        #[inline]
        unsafe fn from_marked_non_null(marked: crate::pointer::MarkedNonNull<T, N>) -> Self {
            Marked::Pointer($pointer::from_marked_non_null(marked))
        }
    };
}

macro_rules! impl_inherent {
    () => {
        /// Gets a `None` for an [`Option<Self>`](core::option::Option).
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

        /// Creates a new [`Option<Self>`](std::option::Option) from a marked pointer.
        ///
        /// # Safety
        ///
        /// The caller has to ensure `marked` is either `null` or a valid pointer to a heap
        /// allocated value of the appropriate `Self` type.
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

        /// Consumes `self` and returns the same value but with the specified `tag`.
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
