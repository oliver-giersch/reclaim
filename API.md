# API Design

## Traits

### NonNullable

```rust
trait NonNullable: Sized {
    type Item: Sized;
    const MARK_BITS: usize;
    
    fn into_marked_non_null(ptr: Self) -> MarkedNonNull<Self::Item, Self::MARK_BITS>;
}
```

implemented by:
- `&'a T`
- `&'a mut T`
- `NonNull`
- `MarkedNonNull`
- `Owned`
- `Shared`
- `Unlinked`
- `Unprotected`

### MarkedPointer

```rust
pub trait MarkedPointer: Sized + Internal {
    type Pointer: NonNullable<Item = Self::Item, MARK_BITS = Self::MARK_BITS>;
    type Item: Sized;
    const MARK_BITS: usize;
    
    fn as_marked_ptr(_: &Self) -> MarkedPtr<Self::Item, Self::MARK_BITS>;
    fn into_marked_ptr(_: Self) -> MarkedPtr<Self::Item, Self::MARK_BITS>;
    fn with_tag(_: Self, tag: usize) -> Marked<Self::Pointer>;
    fn clear_tag(_: Self) -> Self;
    unsafe fn from_marked_ptr(ptr: MarkedPtr<Self::Item, Self::MARK_BITS>) -> Self;
    unsafe fn from_marked_non_null(ptr: MarkedNonNull<Self::Item, Self::MARK_BITS>) -> Self;
}
```

implemented by:
- `Owned`
- `Shared`
- `Unlinked`
- `Unprotected`
- `Option`/`Marked`

blanket implementation for `Option` and `Marked`:

```
impl<U, T, const N: usize> MarkedPointer for Marked<U>
where
    U: NonNullable<Item = T, MARK_BITS = N> +
       MarkedPointer<Item = T, MARK_BITS = N>
{
    ...
}
```

(likewise for `Option`)

### Enums

## Marked

```rust
pub enum Marked<T: NonNullable> {
    Value(T),
    Null(usize)
}
```

### Method Naming Guidelines

- `decompose` returns a tuple `(*mut T, tag)`
- `decompose_ptr` returns a `*mut T`
- `decompose_non_null` returns a `NonNull<T>`
- `decompose_tag` returns a `usize`
- `decompose_ref` returns a tuple `(&T, tag)`
- `decompose_mut` returns a tuple `(&mut T, tag)`

# Old API doc

## Non-nullable Pointer/Reference Types

(sealed) trait:
- `NonNullable` (marker)

move-only:
- `Owned` (`Clone` + `Drop`)
- `Unlinked`

copy:
- `Shared`
- `Unprotected` 

## Wrappers

`enum Marked<T: NonNullable>`

blanket implementation:

```
impl<T: NonNullable + MarkedPointer> MarkedPointer for Marked<T> { ... }
```

## `MarkedPointer` Trait

```
pub trait MarkedPointer: Sized + Internal {
    type Item: Sized;
    const MARK_BITS: usize;

    fn as_marked_ptr(_: &Self) -> MarkedPtr<Self::Item, Self::MARK_BITS>;
    fn into_marked_ptr(_: Self) -> MarkedPtr<Self::Item, Self::MARK_BITS>;
    fn decompose_tag(_: &Self) -> usize;
    fn clear_tag(_: Self) -> Self;
    

    fn tag(&self) -> usize { ... }
    fn strip_tag(self) -> Self { ... }

    fn as_marked(&self) -> MarkedPtr<Self::Item, Self::MarkBits>;
    fn into_marked(self) -> MarkedPtr<Self::Item, Self::MarkBits>;
    unsafe fn from_marked(marked: MarkedPtr<Self::Item, Self::MarkBits>) -> Self;
    unsafe fn from_marked_non_null(marked: MarkedNonNull<Self::Item, Self::MarkBits>) -> Self;
}
```

Implementors:
- `Owned`
- `Option<Owned>`
- `Unlinked`
- `Option<Unlinked>`
- `Shared`
- `Option<Shared>`
- `Unprotected`
- `Option<Unprotected>`

## For `Owned`, `Shared`, `Unlinked`, `Unprotected` (all)

```
pub fn none() -> Option<Self> { ... }
pub fn try_from_marked(marked: MarkedPtr<T, N>) -> Option<Self> { ... }
pub fn into_marked_non_null(this: Self) -> MarkedNonNull<T, N> { ... }
pub fn with_tag(self, tag: usize) -> Self { ... }
```

## For `Owned` (`Deref` + `DerefMut` + ...)

```
pub fn new(owned: T) -> Self;
pub fn compose(owned: T, tag: usize) -> Self;
pub fn decompose_ref(&self) -> (&T, usize);
pub fn decompose_mut(&mut self) -> (&mut T, usize);
pub fn leak<'a>(owned: Self) -> &'a mut T;
pub fn leak_shared<'a>(owned: Self) -> Shared<'a, T, N, R>;
```

## For `Shared<'g, ...>`

```
pub unsafe fn deref(self) -> &'g T;
pub unsafe fn decompose_ref(self) -> (&'g T, usize);
```

## For `Unlinked`

```
pub unsafe fn deref(&self) -> &T;
pub unsafe fn decompose_ref(&self) -> (&T, usize);
pub unsafe fn retire(self) where T: 'static;
pub unsafe fn retire_unchecked(self);
```

## For `Unprotected`

```
pub unsafe fn deref_unprotected<'a>(self) -> &'a T;
```
