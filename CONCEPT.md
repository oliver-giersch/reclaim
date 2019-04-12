# Future Work

Notes on redesigning the crate's interface for future development.

## Prerequisites

The redesign outlined in this document hinges upon several language
or `stdlib` features to be come available (at least in nightly):

- `const generics`
- generic associated types (`GAT`)
- custom allocator support (at least for `Box`)
- more `const fn` capabilities (esp trait bounds and assertions)

The inclusion of an intrinsic `offset!` macro would also be desirable,
as it would allow removing the dependency on the `memoffset` crate.

## Goals

- replace `typenum` dependency with `const generics` once available
- allow fully generic application of the reclaim trait(s) once `GAT` are available
- enable (ergonomic) use in `#[no_std]` environments
- integrate with custom allocators
- the ordering of generic parameters will be so, that `N` always appears last
  (for aesthetic reasons)

## Future API

### Primary Traits

#### Reclaim

The `Reclaim` trait will remain largely unchanged, but most of its requirements
are moved into the `LocalReclaim` trait, which it has to extend. 
If the implemented reclamation scheme requires management of thread local
state, it will be only usable in `std` environments, where thread local
variables can be accessed like statics (with the `thread_local` macro).

Most reclamation schemes do not strictly require thread local state in order
to function, but frequently make use of thread local storage for the sake of
performance and efficiency, as allows costly writes to shared memory to be
delayed or avoided.

```rust
pub trait Reclaim: LocalReclaim + Sync {
    // allows using the trait in a fully generic manner, but requires GAT
    fn guarded<T, const N: usize>() -> Self::Guarded<T, N>;
    unsafe fn retire<T: 'static, const N: usize>(unlinked: Unlinked<T, Self, N>);
    unsafe fn retire_unchecked<T, const N: usize>(unlinked: Unlinked<T, Self, N>);
}
```

Most implementations of this trait will use the underlying methods of the `LocalReclaim`
trait and supply the `&Self::Local` argument from thread local state.
Otherwise, they can e.g. just supply a default `&()` argument, if no local state needs
to be managed.

#### LocalReclaim

The `LocalReclaim` trait will be the `#[no_std]` compatible foundational trait for `Reclaim`.
Custom allocators can be specified with the additional `Allocator` associated type.
It is not yet clear, exactly which trait the `Allocator` type must be bound by.
The current assumption/premise is, that an allocator that is usable for concurrent memory
reclamation can **impossibly** be stateful and must function like a global allocator, but
must not necessarily be registered as such.

The `Local` and `RecordHeader` types are both optional, in which case they should be defined
to be `()`.
This is applicable, if a reclamation scheme does not require any thread local state to be
managed.

```rust
pub trait LocalReclaim: Sized  {
    type Allocator: GlobalAlloc; // or some other trait
    // needs GAT
    type Guarded<T, const N: usize>: Protect<Item = T, Reclaimer = Self, MARK_BITS = N>;
    type Local: Sized;
    type RecordHeader: Default + Sized;
    
    // allows using the trait in a fully generic manner but requires GAT
    fn guarded<T, const N: usize>(local: &Self::Local) -> Self::Guarded<T, N>;
    unsafe fn retire<T: 'static, const N: usize>(
        local: &Self::Local,
        unlinked: Unlinked<T, Self, N>
    );
    unsafe fn retire_unchecked<T, const N: usize>(
        local: &Self::Local,
        unlinked: Unlinked<T, Self, N>
    );
}
```

Any reclamation scheme can and must implement this trait.
Implementing the `Reclaim` trait is strictly optional but is the correct thing to do, if a reclamation
scheme does either not require any actual thread local state or exists in an environment, which can
use thread local statics.

#### Protect

The `Protect` trait remains largely unchanged, however it loses the `new` method and is thus itself
no longer required to be generically constructable.
Construction is achieved through exclusively the `guarded` method of the `Reclaim` trait.
Implementations for `#[no_std]` environments will likely have to store a reference/pointer to the
thread local state in the `struct` implementing the trait.

```rust
pub trait Protect: Clone + Sized {
    type Item: Sized;
    type Reclaimer: Reclaim;
    const MARK_BITS: usize;
    
    fn shared<'a>(
        &'a self
    ) -> Option<Shared<'a, Self::Item, Self::Reclaimer, Self::MARK_BITS>>;
    
    fn acquire<'a>(
        &'a mut self,
        atomic: &Atomic<Self::Item, Self::Reclaimer, Self::MARK_BITS>,
        order: Ordering
    ) -> Option<Shared<'a, Self::Item, Self::Reclaimer, Self::MARK_BITS>>;
    
    fn acquire_if_equal<'a>(
        &'a mut self,
        atomic: &Atomic<Self::Item, Self::Reclaimer, Self::MARK_BITS>,
        expected: MarkedPtr<Self::Item, Self::MARK_BITS>,
        order: Ordering,
    ) -> AcquireResult<Self::Item, Self::Reclaimer, Self::MARK_BITS>;
    
    fn release(&mut self);
}
```
### Types

The types currently exported by the crate will likely remain unchanged and will continue
to consist of the following:

#### Marked Pointers

- `AtomicMarkedPtr`
- `MarkedPtr`
- `MarkedNonNull`

These types will remain largely unchanged, but their definition will be adjusted to use
`const generics` for the generic type parameter `N` like so:

```rust
pub struct MarkedPtr<T, const N: usize = 0> {
    ...
}
````

The implicit trait bound `T: Sized` will not be relaxed primarily for two reasons:

- fat pointers can not be read/written on 64-bit architectures
- unsized types have undetermined alignment, which is required to determine the
  number of mark bits
  
#### Atomic

The primary `Atomic` type will remain largely unchanged:

```rust
pub struct Atomic<T, R, const N: usize = 0> {
    ...
}
```

Its API will continue to be oriented on the API of `Option` and `AtomicPtr`.

#### Owned

The `Box` like but markable `Owned` type must be defined as follows:

```rust
pub struct Owned<T, R: LocalReclaim, const N: usize> {
    ...
}
```

The `LocalReclaim` trait bound must be part of the type definition because it is required for
correctly dropping and correctly deallocating an `Owned`.


#### Non-nullable Pointers

- `Shared`
- `Unlinked`
- `Unprotected`
