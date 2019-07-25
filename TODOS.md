# TODOS

- make `Atomic::from_raw` safe, make `Atomic::take` unsafe(?)
- rename `GlobalReclaim::try_flush` to `try_reclaim`
- add `UnwrapUnchecked` trait, impl for `Option`, `Result`, `Marked`
- add `unwrap_ptr` method to `NonNullable`
- add `Owned::extract() -> T`
