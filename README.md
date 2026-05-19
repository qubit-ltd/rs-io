# Qubit IO

[![Rust CI](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io/coverage-badge.json)](https://qubit-ltd.github.io/rs-io/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io.svg?color=blue)](https://crates.io/crates/qubit-io)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Chinese Document](https://img.shields.io/badge/Document-Chinese-blue.svg)](README.zh_CN.md)

Small stream I/O trait and extension utilities for Rust.

## Overview

Qubit IO provides a compact set of low-level utilities on top of `std::io`:

- object-safe composition traits for common `std::io` capability combinations;
- extension traits for recurring low-level I/O patterns that the standard
  library leaves to callers;
- the `Streams` namespace for stream-level copy and compare operations;
- wrapper types for stream instrumentation, limiting, teeing, checksumming, and
  position restoration;
- codec wrappers and extension traits for binary, LEB128, ZigZag, and
  length-prefixed UTF-8 data.

Local filesystem helpers such as `Files`, `Filenames`, `TempFile`, and `TempDir`
now live in `qubit-local-fs`.

## Installation

```toml
[dependencies]
qubit-io = "0.2"
```

## Quick Example

```rust
use std::io::Cursor;

use qubit_io::{
    ReadExt,
    Streams,
    WriteSeekExt,
};

let mut input = Cursor::new(b"hello".to_vec());
let mut output = Vec::new();

Streams::copy(&mut input, &mut output)?;
assert_eq!(b"hello", output.as_slice());

let mut cursor = Cursor::new(vec![0; 8]);
cursor.write_all_at_preserving_position(2, b"rs")?;

# Ok::<(), std::io::Error>(())
```

## Main APIs

| API | Purpose |
| --- | --- |
| `ReadSeek`, `BufReadSeek`, `ReadWrite`, `WriteSeek`, `ReadWriteSeek` | object-safe composition traits for common `std::io` capability sets |
| `ReadExt`, `BufReadExt`, `SeekExt`, `ReadSeekExt`, `WriteSeekExt` | extension traits for exact, bounded, delimiter-oriented, and position-preserving I/O |
| `BinaryReadExt`, `BinaryWriteExt` | primitive numeric scalar encoding and decoding |
| `Leb128ReadExt`, `Leb128WriteExt` | unsigned and signed LEB128 encoding and decoding |
| `ZigZagReadExt`, `ZigZagWriteExt` | ZigZag-mapped signed integer encoding and decoding |
| `StringReadExt`, `StringWriteExt` | length-prefixed UTF-8 string encoding and decoding |
| `Streams` | stream copy, bounded copy, EOF-limited copy, and content comparison |
| `CountingReader`, `CountingWriter` | byte counting wrappers |
| `LimitReader`, `LimitWriter` | byte limit wrappers |
| `TeeReader`, `TeeWriter` | duplicate reads or writes to a branch writer |
| `ChecksumReader`, `ChecksumWriter` | update a caller-provided checksum state while reading or writing |
| `PositionGuard` | restore seek position unless dismissed |

## Crate Boundary

`qubit-io` is intentionally limited to stream and byte I/O. It does not expose
local filesystem helpers. Use `qubit-local-fs` for local path utilities,
temporary files and directories, directory copy, directory cleanup, and atomic
file writes.

## Runtime Dependencies

This crate depends only on the Rust standard library at runtime.
