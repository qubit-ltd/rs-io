# Qubit IO

[![Rust CI](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io/coverage-badge.json)](https://qubit-ltd.github.io/rs-io/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io.svg?color=blue)](https://crates.io/crates/qubit-io)
[![License](https://img.shields.io/crates/l/qubit-io.svg)](LICENSE)

Small `std::io` trait utilities for Rust.

`qubit-io` provides:

- object-safe composition traits such as `ReadSeek`, `ReadWrite`, and
  `ReadWriteSeek`;
- extension traits for recurring `Read`, `BufRead`, `Seek`, `Read + Seek`,
  `Write`, and `Write + Seek` patterns;
- `Streams` utility functions for copy and content comparison operations;
- lightweight reader and writer wrappers such as `CountingReader`,
  `LimitReader`, `PositionGuard`, `TeeReader`, and checksum wrappers.

Binary scalar, LEB128, and ZigZag codecs are no longer part of this crate. Use
`qubit-codec-binary` for buffer-level binary codecs and `qubit-io-binary` for
binary stream readers, writers, and extension traits.

Detailed usage is documented in the [user guide](doc/user_guide.md). API
reference documentation is available on [docs.rs](https://docs.rs/qubit-io).

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
qubit-io = "0.6"
```

## Quick Example

```rust
use std::io::Cursor;

use qubit_io::{
    ReadExt,
    Streams,
};

let mut input = Cursor::new(b"abcdef".to_vec());
let mut prefix = [0_u8; 3];

let read = input.read_exact_or_eof(&mut prefix)?;
assert_eq!(3, read);
assert_eq!(b"abc", &prefix);

let mut source = Cursor::new(b"payload".to_vec());
let mut output = Vec::new();
let copied = Streams::copy_at_most(&mut source, &mut output, 4)?;

assert_eq!(4, copied);
assert_eq!(b"payl", output.as_slice());
# Ok::<(), std::io::Error>(())
```

## Crate Split

The codec and stream stack is intentionally split:

- `qubit-codec`: core byte order, codec, coder, encoder, and decoder traits;
- `qubit-codec-binary`: buffer-level binary, LEB128, and ZigZag codecs;
- `qubit-io`: generic `std::io` helpers;
- `qubit-io-binary`: binary stream readers, writers, and extension traits;
- `qubit-codec-text` and `qubit-io-text`: text codecs and text stream adapters.

Repository: [https://github.com/qubit-ltd/rs-io](https://github.com/qubit-ltd/rs-io)
