# Reclaim - Concurrent memory reclamation

A unified abstract interface for concurrent memory reclamation that leverages
Rust's type system in order to expose a public API that is largely safe.

[![Build Status](https://travis-ci.com/oliver-giersch/reclaim.svg?branch=master)](
https://travis-ci.com/oliver-giersch/reclaim)
[![Latest version](https://img.shields.io/crates/v/reclaim.svg)](
https://crates.io/crates/reclaim)
[![Documentation](https://docs.rs/reclaim/badge.svg)](https://docs.rs/reclaim)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](
https://github.com/oliver-giersch/reclaim)
[![Rust 1.36+](https://img.shields.io/badge/rust-1.36+-lightgray.svg)](
https://www.rust-lang.org)

## Usage

Add the following to your `Cargo.toml`

```
[dependencies]
reclaim = "0.1.0"
```

## Minimum Supported Rust Version (MSRV)

The minimum supported rust version for this crate is 1.36.0.

## Features

This crate is `no_std` + `alloc` compatible. The `std` feature (enabled by
default) must be disabled when this crate is intended for use in a `#[no_std]`
environment.

## Reclamation Scheme Implementations

The following list contains the currently available reclamation scheme
implementations based on this crate's API and interface:

- [debra (work in progress)](https://github.com/oliver-giersch/debra)
- [hazptr](https://github.com/oliver-giersch/hazptr)

## Down the Road

The ultimate goal of this crate is to allow fully generic memory reclamation
based only on the traits `Reclaim`/`LocalReclaim` and `Protect`.
This will allow writers of lock-free data structures to parametrize their code
over the reclamation scheme they use, making it easily exchangeable.
This is currently not possible due to the lack of GAT (generic associated
types).

Likewise, since `const generics` are currently not available in stable Rust, the
crate's type safe pointer tagging mechanism has to rely on the `typenum` crate.
This is also bound change in the future.

## License

Reclaim is distributed under the terms of both the MIT license and the
Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.
