# Reclaim - Concurrent memory reclamation

A unified abstract interface for concurrent memory reclamation that leverages Rust's type system in order to expose a
public API that is (almost) safe.

## Usage

Add the following to your `Cargo.toml`

```
[dependencies]
reclaim = "0.1.0"
```

## Memory Management in Rust

Rust's ownership model in combination with the standard library's smart pointer types `Box`, `Rc` and `Arc` are
perfectly well suited for a wide range of the most common use cases. Consequently, there is usually little need for
automated memory management like *Garbage Collection* (GC). However, ...

## Reclaim API

The abstract interface exposed by this crate is formed by a number of traits and concrete but generic types. The most
important of these traits is the `Reclaim` trait, which provides functionality to **retire**

```rust
pub struct Atomic<T, const N: usize, R: Reclaim> { /* ... */ } 
```

## Unified Concurrent Memory Reclamation Interface

- How does Rust manage memory
- why is GC necessary for lock-free concurrent data structures

## Approaches for Protecting Memory from Reclamation

## Memory Allocation and Custom Allocators
