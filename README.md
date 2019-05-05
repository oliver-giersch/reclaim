# Reclaim - Concurrent memory reclamation

A unified abstract interface for concurrent memory reclamation that leverages Rust's type system in order to expose a
public API that is (almost) safe.

[![Build Status](https://travis-ci.com/oliver-giersch/reclaim.svg?branch=master)](
https://travis-ci.com/oliver-giersch/reclaim)
[![Latest version](https://img.shields.io/crates/v/reclaim.svg)](
https://crates.io/crates/reclaim)
[![Documentation](https://docs.rs/reclaim/badge.svg)](https://docs.rs/reclaim)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](
https://github.com/oliver-giersch/reclaim)
[![Rust 1.35+](https://img.shields.io/badge/rust-1.35+-lightgray.svg)](
https://www.rust-lang.org)

## Usage

Add the following to your `Cargo.toml`

```
[dependencies]
reclaim = "0.1.0"
```

## Minimum Supported Rust Version (MSRV)

The minimum supported rust version for this crate is 1.35.0

## Features

...

## Reclamation Scheme Implementations

The following lists the currently available reclamation scheme implementations
using this crate:

- hazptr
- arc-reclaim
- debra

## Down the Road

...
