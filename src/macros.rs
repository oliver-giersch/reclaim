macro_rules! impl_trait {
    ($self:ident) => {
        type Pointer = Self;
        type Item = T;
        type MarkBits = N;

        #[inline]
        fn as_marked_ptr(&self) -> crate::pointer::MarkedPtr<T, N> {
            self.inner.into_marked_ptr()
        }

        #[inline]
        fn into_marked_ptr(self) -> crate::pointer::MarkedPtr<Self::Item, Self::MarkBits> {
            self.into_marked_non_null().into_marked_ptr()
        }

        #[inline]
        fn marked($self: Self, tag: usize) -> crate::pointer::Marked<Self::Pointer> {
            let inner = $self.inner.with_tag(tag);
            crate::pointer::Marked::Value(Self { inner, _marker: PhantomData })
        }

        #[inline]
        fn unmarked($self: Self) -> Self {
            let inner = $self.inner.clear_tag();
            Self { inner, _marker: PhantomData }
        }

        #[inline]
        unsafe fn from_marked_ptr(
            marked: crate::pointer::MarkedPtr<Self::Item, Self::MarkBits>
        ) -> Self
        {
            debug_assert!(!marked.is_null());
            Self { inner: MarkedNonNull::new_unchecked(marked), _marker: PhantomData}
        }

        #[inline]
        unsafe fn from_marked_non_null(
            marked: crate::pointer::MarkedNonNull<Self::Item, Self::MarkBits>
        ) -> Self
        {
            Self { inner: marked, _marker: PhantomData }
        }
    };
}

macro_rules! impl_inherent {
    ($self:ident) => {
        /// Creates a `None` variant for an [`Option<Self>`][Option].
        ///
        /// This is useful for calls to [`store`][store], [`swap`][swap] or
        /// [`compare_exchange_*`][compare_exchange], when a `null` pointer
        /// needs to be inserted.
        /// These methods accept values of various non-nullable pointer types
        /// ([`Shared`][Shared], [`Owned`][Owned], [`Unlinked`][Unlinked] and
        /// [`Unprotected`][Unprotected]) and [`Option`] types thereof as
        /// argument.
        /// However, the compiler is usually not able to infer the concrete
        /// type, when a [`None`] is inserted, and this function is intended for
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
        /// type Unlinked<T> = reclaim::leak::Unlinked<T, reclaim::typenum::U0>;
        ///
        /// let atomic = Atomic::new(1);
        /// let swap = atomic.swap(Owned::none(), Ordering::Relaxed).unwrap();
        ///
        /// assert_eq!(swap.as_ref(), &1);
        /// unsafe { Unlinked::retire(swap) }; // leaks memory
        /// ```
        #[inline]
        pub fn none() -> Option<Self> {
            None
        }

        /// Creates an unmarked [`Null`][crate::pointer::Marked::Null] variant
        /// for a [`Marked<Self>`][crate::pointer::Marked].
        #[inline]
        pub fn null() -> crate::pointer::Marked<Self> {
            Marked::Null(0)
        }

        /// Creates a marked [`Null`][crate::pointer::Marked::Null] variant for
        /// a [`Marked<Self>`][crate::pointer::Marked] with the given `tag`.
        #[inline]
        pub fn null_with_tag(tag: usize) -> crate::pointer::Marked<Self> {
            Marked::Null(tag)
        }

        /// Consumes the given `Self` and returns the same value but with the
        /// specified `tag`.
        #[inline]
        pub fn with_tag($self: Self, tag: usize) -> Self {
            let inner = $self.inner;
            core::mem::forget($self);
            Self { inner: inner.with_tag(tag), _marker: PhantomData }
        }

        /// Decomposes the given `Self`, returning the original value without
        /// its previous tag and the separated tag.
        #[inline]
        pub fn decompose($self: Self) -> (Self, usize) {
            let (inner, tag) = $self.inner.decompose();
            core::mem::forget($self);
            ( Self { inner: crate::pointer::MarkedNonNull::from(inner), _marker: PhantomData }, tag)
        }
    };
}
