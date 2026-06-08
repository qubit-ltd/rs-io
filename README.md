# Qubit IO

[![Rust CI](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io/coverage-badge.json)](https://qubit-ltd.github.io/rs-io/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io.svg?color=blue)](https://crates.io/crates/qubit-io)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Chinese Document](https://img.shields.io/badge/Document-Chinese-blue.svg)](README.zh_CN.md)

Generic unit buffering, byte-stream buffering, and small `std::io` trait
utilities for Rust.

## Overview

`qubit-io` provides:

- minimal indexed I/O traits: `Input` and `Output`, with blanket
  byte implementations for `Read` and `Write`;
- unit-oriented buffering primitives: `Buffer<T>`, `BufferedInput`, and
  `BufferedOutput`;
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

## Design Goals

- **Generic I/O Only**: keep this crate focused on reusable `std::io` helpers.
- **Unit-Oriented Core**: provide low-level indexed input and output contracts
  for hot paths that have already validated ranges.
- **Format-Agnostic Buffering**: provide efficient unit and byte buffers
  without embedding binary codec, text codec, or record-format knowledge.
- **Explicit Low-Level Contracts**: expose hot-path APIs such as `Buffer` and
  unchecked range helpers with clear caller responsibilities.
- **Object-Safe Composition**: make common trait combinations easy to name and
  pass around.
- **Predictable Extension Traits**: provide recurring read, write, seek, and
  copy patterns without hiding allocation or error behavior.
- **Layer Separation**: keep binary and text codec stream adapters in sibling
  crates.
- **Small Dependency Graph**: provide useful I/O tools without runtime
  dependencies.

## Features

### Indexed I/O

- **`Input`**: minimal unchecked indexed read contract with an associated
  `Item` type for copying units into `output[index..index + count]`; every
  `Read` value implements `Input<Item = u8>`.
- **`Output`**: minimal unchecked indexed write contract for copying
  units of its associated `Item` type from `input[index..index + count]` plus
  explicit flushing; every `Write` value implements `Output<Item = u8>`.

### Buffered I/O

- **`Buffer<T>`**: low-level position/limit storage with a readable window and
  spare tail capacity.
- **`BufferedInput<I>`**: buffered unit input over `Input`,
  with unread-window inspection, count-aware refilling, `into_parts`, and
  indexed unchecked reads for validated output ranges. When `I::Item = u8`, it
  implements `Read` and `BufRead`, plus logical `Seek` when the wrapped input
  supports `Seek`.
- **`BufferedOutput<O>`**: buffered unit output over `Output`,
  with spare-window access, checked and unchecked advancing, explicit
  flushing, non-flushing `into_parts`, and large-write bypass paths. When
  `O::Item = u8`, it implements `Write`, plus flushing `Seek` when the wrapped
  output supports `Seek`.
- **`DEFAULT_BUFFER_CAPACITY`**: shared default capacity for input and output
  buffering.

### Composition Traits

- **`ReadSeek`**: names `Read + Seek`.
- **`BufReadSeek`**: names `BufRead + Seek`.
- **`ReadWrite`**: names `Read + Write`.
- **`ReadWriteSeek`**: names `Read + Write + Seek`.
- **`WriteSeek`**: names `Write + Seek`.

### Extension Traits

- **`ReadExt`**: exact reads, partial EOF reads, limited reads, and copy helpers.
- **`BufReadExt`**: bounded line and delimiter reads.
- **`SeekExt`**: stream size helpers that preserve position.
- **`ReadSeekExt`**: peek/read-at helpers that restore position.
- **`WriteExt`**: unchecked write helpers for validated ranges.
- **`WriteSeekExt`**: write-at helpers that preserve position.

### Utility Functions and Wrappers

- **`Streams`**: copy, bounded copy, equality, and lexicographic comparison.
- **Counting wrappers**: `CountingReader` and `CountingWriter`.
- **Limit wrappers**: `LimitReader` and `LimitWriter`.
- **Tee wrappers**: `TeeReader` and `TeeWriter`.
- **Checksum wrappers**: `ChecksumReader` and `ChecksumWriter`.
- **Position guard**: `PositionGuard` restores stream position on drop unless
  dismissed.

## Documentation

- [User Guide](doc/user_guide.md)
- [API Reference](https://docs.rs/qubit-io)
- [Chinese README](README.zh_CN.md)

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
qubit-io = "0.8"
```

## Quick Start

```rust
use std::io::{
    Cursor,
    Write,
};

use qubit_io::{
    BufferedInput,
    BufferedOutput,
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

let mut buffered_input = BufferedInput::with_capacity(
    Cursor::new(b"abcdef".to_vec()),
    3,
);
buffered_input.ensure_available(3)?;
assert_eq!(b"abc", buffered_input.unread_slice());
unsafe {
    buffered_input.consume_unchecked(3);
}

let mut buffered_output =
    BufferedOutput::with_capacity(Cursor::new(Vec::<u8>::new()), 4);
buffered_output.ensure_spare_capacity(3)?;
buffered_output.spare_slice_mut()[0..3].copy_from_slice(b"xyz");
unsafe {
    buffered_output.advance_unchecked(3);
}
buffered_output.flush()?;
let (cursor, pending) = buffered_output.into_parts();
assert!(pending.is_empty());
assert_eq!(b"xyz", cursor.into_inner().as_slice());
# Ok::<(), std::io::Error>(())
```

## API Reference

### Indexed I/O Traits

| Trait | Purpose |
|-------|---------|
| `Input` | Reads units into caller-validated indexed output ranges |
| `Output` | Writes units from caller-validated indexed input ranges and flushes pending units |

### Trait Aliases

| Trait | Equivalent Bounds |
|-------|-------------------|
| `ReadSeek` | `Read + Seek` |
| `BufReadSeek` | `BufRead + Seek` |
| `ReadWrite` | `Read + Write` |
| `ReadWriteSeek` | `Read + Write + Seek` |
| `WriteSeek` | `Write + Seek` |

### Utility Types

| Type | Purpose |
|------|---------|
| `Buffer` | Low-level position/limit storage for hot-path buffering |
| `BufferedInput` | Buffered unit input over an `Input` source |
| `BufferedOutput` | Buffered unit output over an `Output` sink |
| `Streams` | Static helpers for copying and comparing streams |
| `CountingReader` / `CountingWriter` | Count successful bytes read or written |
| `LimitReader` / `LimitWriter` | Cap bytes read or written through a wrapper |
| `TeeReader` / `TeeWriter` | Mirror bytes into a secondary sink |
| `ChecksumReader` / `ChecksumWriter` | Feed successful bytes into a caller-provided hasher |
| `PositionGuard` | Restore a seek position unless explicitly dismissed |

### Constants

| Constant | Purpose |
|----------|---------|
| `DEFAULT_BUFFER_CAPACITY` | Shared default capacity for buffered input and output |

## Crate Split

The codec and stream stack is intentionally split:

- `qubit-codec`: core byte order, codec, transcoder, encoder, and decoder traits;
- `qubit-codec-binary`: buffer-level binary, LEB128, and ZigZag codecs;
- `qubit-io`: generic `std::io` helpers;
- `qubit-io-binary`: binary stream readers, writers, and extension traits;
- `qubit-codec-text` and `qubit-io-text`: text codecs and text stream adapters.

## Performance Considerations

Most helpers operate directly on caller-provided buffers and delegate to the
underlying `Read`, `Write`, or `Seek` implementation. Wrapper types avoid hidden
allocation; any buffering policy remains explicit at the call site.

`Input::read_unchecked`, `Output::write_unchecked`, `Buffer<T>`,
`BufferedInput::unread_raw_parts`, and `BufferedOutput::spare_raw_parts_mut`
are low-level APIs for callers that have already validated ranges. They are
intended for hot paths such as binary and text stream adapters where avoiding
repeated slicing and bounds checks matters. Safe wrapper methods remain
available for general-purpose use.

## Testing & Code Coverage

This project keeps generic I/O behavior covered by integration tests under
`tests/`.

### Running Tests

```bash
# Run all tests
cargo test

# Run with coverage report
./coverage.sh

# Generate text format report
./coverage.sh text

# Align code with CI requirements
./align-ci.sh

# Run CI checks (format, clippy, test, coverage, audit)
RS_CI_SKIP_TOOLCHAIN_UPDATE=1 ./ci-check.sh
```

## Dependencies

`qubit-io` has no runtime dependencies.

## License

Copyright (c) 2026. Haixing Hu.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

See [LICENSE](LICENSE) for the full license text.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### Development Guidelines

- Keep this crate generic and independent of concrete codec formats.
- Maintain deterministic tests for I/O edge cases.
- Document public APIs and error behavior.
- Ensure all checks pass before submitting a PR.

## Author

**Haixing Hu**

## Related Projects

More Rust libraries from Qubit are available under the
[qubit-ltd](https://github.com/qubit-ltd) GitHub organization.

---

Repository: [https://github.com/qubit-ltd/rs-io](https://github.com/qubit-ltd/rs-io)
