# Qubit IO

[![Rust CI](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io/coverage-badge.json)](https://qubit-ltd.github.io/rs-io/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io.svg?color=blue)](https://crates.io/crates/qubit-io)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Chinese Document](https://img.shields.io/badge/Document-Chinese-blue.svg)](README.zh_CN.md)

Small I/O trait and extension utilities for Rust.

## Overview

Qubit IO provides a compact set of low-level utilities on top of `std::io`:

- object-safe composition traits for common `std::io` capability combinations;
- extension traits for recurring low-level I/O patterns that the standard
  library leaves to callers;
- utility functions and wrapper types for common stream instrumentation,
  limiting, teeing, checksumming, and position restoration.

The composition traits are useful when an API needs a trait object such as
`&mut dyn ReadSeek` or `Box<dyn ReadWriteSeek>` instead of a generic bound like
`R: Read + Seek`.

The extension traits cover conservative, standard-library-first behavior such
as bounded reads, limited delimiter reads, binary scalar encoding, LEB128 and
ZigZag integer encodings, and position-preserving seek operations.

## Design Goals

- **Object-safe composition**: provide named trait-object-friendly I/O bounds.
- **Standard-library first**: build directly on `std::io::{Read, Write, Seek}`.
- **Practical wrappers**: include only small wrappers with clear stream-level
  behavior.
- **Tiny API surface**: keep only generic, low-level operations that are reused
  across crates.
- **Position safety**: make non-consuming inspection and random-access patching
  explicit.
- **Integration friendly**: work with cursors, files, buffers, streams, and
  custom types implementing the standard I/O traits.

## Features

### Object-Safe I/O Trait Combinations

- **`ReadSeek`**: combines `Read` and `Seek` for readable random-access inputs.
- **`BufReadSeek`**: combines `BufRead` and `Seek` for buffered random-access
  inputs.
- **`ReadWrite`**: combines `Read` and `Write` for duplex streams or buffers.
- **`WriteSeek`**: combines `Write` and `Seek` for writable random-access
  outputs.
- **`ReadWriteSeek`**: combines `Read`, `Write`, and `Seek` for fully mutable
  random-access I/O objects.

### I/O Extension Traits

- **`ReadExt`**:
  - `read_exact_or_eof` retries short reads until the destination buffer is
    full or EOF is reached.
  - `discard_exact_or_eof` consumes and discards up to a requested number of
    bytes without allocating.
  - `copy_to`, `copy_to_at_most`, and `copy_to_end_limited` copy into a writer
    with method-style ergonomics.
  - `read_to_end_limited` and `read_to_end_limited_into` read the remaining
    input with a maximum accepted size.
  - `read_to_string_limited` and `read_to_string_limited_into` read bounded
    UTF-8 text.
- **`BufReadExt`**:
  - `read_until_limited`, `read_until_limited_into`, `read_line_limited`,
    `read_line_limited_into`, and `discard_until_limited` provide bounded
    delimiter-oriented operations.
- **`SeekExt`**:
  - `stream_size` measures stream size and restores the original position.
- **`ReadSeekExt`**:
  - `peek_exact_or_eof` reads from the current position and restores it.
  - `read_exact_or_eof_at` reads from an absolute offset and restores the
    original position.
- **`WriteSeekExt`**:
  - `write_all_at_preserving_position` writes bytes at an absolute offset and
    restores the original position.
- **`BinaryReadExt` / `BinaryWriteExt`**:
  - read and write primitive numeric scalars through `u128` / `i128` with
    `_be` / `_le` suffix methods or a runtime `ByteOrder`.
- **`Leb128IntReadExt` / `Leb128IntWriteExt`**:
  - read and write unsigned and signed LEB128 integers through 128-bit values;
    read methods also provide `_strict` canonical-decoding variants.
- **`ZigZagIntReadExt` / `ZigZagIntWriteExt`**:
  - read and write ZigZag-mapped signed integers through 128-bit values using
    unsigned LEB128 payloads; read methods also provide `_strict` variants.
- **`StringReadExt` / `StringWriteExt`**:
  - read and write length-prefixed UTF-8 strings with ULEB128, `u16`, or `u32`
    byte-length prefixes.

### Utilities and Wrappers

- file helpers create missing parent directories and provide durable
  same-directory atomic writes;
- content helpers compare readers and copy bounded byte ranges;
- wrapper types provide counting, limiting, teeing, checksum updating, and
  position-guard behavior.
- `qubit_io::prelude` re-exports the extension traits and composition traits
  for method-oriented call sites.

### Blanket Implementations

Every type that implements the corresponding standard-library traits
automatically implements the Qubit IO composition and extension traits. You do
not need to write adapter code for `std::io::Cursor`, `std::fs::File`, or your
own I/O types.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
qubit-io = "0.2"
```

## Quick Start

### Read and Seek

Use `ReadSeek` when a function needs to read from the current position and also
move around in the input.

```rust
use qubit_io::ReadSeek;
use std::io::SeekFrom;

fn read_second_byte(input: &mut dyn ReadSeek) -> std::io::Result<u8> {
    input.seek(SeekFrom::Start(1))?;

    let mut byte = [0; 1];
    input.read_exact(&mut byte)?;
    Ok(byte[0])
}

fn main() -> std::io::Result<()> {
    let mut cursor = std::io::Cursor::new(b"abc".to_vec());
    assert_eq!(read_second_byte(&mut cursor)?, b'b');
    Ok(())
}
```

### Read Exact or EOF

Use `ReadExt::read_exact_or_eof` when short reads should be retried, but EOF
before the buffer is full should return a partial byte count instead of
`UnexpectedEof`.

```rust
use qubit_io::ReadExt;

fn read_prefix(input: &mut dyn std::io::Read) -> std::io::Result<Vec<u8>> {
    let mut buffer = vec![0; 8];
    let count = input.read_exact_or_eof(&mut buffer)?;
    buffer.truncate(count);
    Ok(buffer)
}

fn main() -> std::io::Result<()> {
    let mut cursor = std::io::Cursor::new(b"abc".to_vec());
    assert_eq!(read_prefix(&mut cursor)?, b"abc");
    Ok(())
}
```

### Peek Without Consuming Position

Use `ReadSeekExt::peek_exact_or_eof` when inspecting a seekable stream should
not change the caller-visible position.

```rust
use qubit_io::ReadSeekExt;
use std::io::{Seek, SeekFrom};

fn peek_three(input: &mut std::io::Cursor<Vec<u8>>) -> std::io::Result<[u8; 3]> {
    input.seek(SeekFrom::Start(2))?;

    let mut buffer = [0; 3];
    let count = input.peek_exact_or_eof(&mut buffer)?;
    assert_eq!(3, count);
    assert_eq!(2, input.stream_position()?);
    Ok(buffer)
}

fn main() -> std::io::Result<()> {
    let mut cursor = std::io::Cursor::new(b"abcdef".to_vec());
    assert_eq!(peek_three(&mut cursor)?, *b"cde");
    Ok(())
}
```

### Read, Write, and Seek

Use `ReadWriteSeek` for in-memory buffers, files, or custom handles that need
full read-write random access.

```rust
use qubit_io::ReadWriteSeek;
use std::io::SeekFrom;

fn overwrite_prefix(io: &mut dyn ReadWriteSeek) -> std::io::Result<String> {
    io.write_all(b"hello")?;
    io.seek(SeekFrom::Start(0))?;
    io.write_all(b"j")?;
    io.seek(SeekFrom::Start(0))?;

    let mut content = String::new();
    io.read_to_string(&mut content)?;
    Ok(content)
}

fn main() -> std::io::Result<()> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    assert_eq!(overwrite_prefix(&mut cursor)?, "jello");
    Ok(())
}
```

### Read and Write

Use `ReadWrite` when the value is a bidirectional stream but does not need
seeking.

```rust
use qubit_io::ReadWrite;

fn write_ping(stream: &mut dyn ReadWrite) -> std::io::Result<()> {
    stream.write_all(b"ping")
}

fn main() -> std::io::Result<()> {
    let mut buffer = std::io::Cursor::new(Vec::new());
    write_ping(&mut buffer)?;
    assert_eq!(buffer.into_inner(), b"ping");
    Ok(())
}
```

### Write and Seek

Use `WriteSeek` when output must be patched after earlier bytes are written,
such as writing a header length after serializing a payload.

```rust
use qubit_io::WriteSeek;
use std::io::SeekFrom;

fn write_with_header(output: &mut dyn WriteSeek) -> std::io::Result<()> {
    output.write_all(&[0])?;
    output.write_all(b"payload")?;
    output.seek(SeekFrom::Start(0))?;
    output.write_all(&[7])?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    write_with_header(&mut cursor)?;
    assert_eq!(cursor.into_inner(), b"\x07payload");
    Ok(())
}
```

### Write at an Offset

Use `WriteSeekExt::write_all_at_preserving_position` when patching a header,
offset table, or length field should not disturb the caller's current write
position.

```rust
use qubit_io::WriteSeekExt;
use std::io::{Seek, Write};

fn patch_length(output: &mut std::io::Cursor<Vec<u8>>) -> std::io::Result<()> {
    output.write_all(&[0, 0])?;
    output.write_all(b"payload")?;
    let end = output.stream_position()?;

    output.write_all_at_preserving_position(0, &[0, 7])?;
    assert_eq!(end, output.stream_position()?);
    Ok(())
}

fn main() -> std::io::Result<()> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    patch_length(&mut cursor)?;
    assert_eq!(cursor.into_inner(), b"\x00\x07payload");
    Ok(())
}
```

## When to Use These Traits

Use Qubit IO composition traits when:

- Your API stores or passes heterogeneous I/O objects behind trait objects.
- You want a concise parameter type such as `&mut dyn ReadWriteSeek`.
- You need object safety and cannot use unstable trait aliases.
- You want public signatures to communicate a common I/O capability directly.

Prefer ordinary generic bounds when the concrete type can remain generic:

```rust
use std::io::{Read, Seek};

fn generic_reader<R>(reader: &mut R)
where
    R: Read + Seek,
{
    // Use this style when the caller's concrete type can stay generic.
}
```

## API Reference

For a complete method-level overview, see the [API matrix](doc/api-matrix.md).

| Trait | Standard-library bounds | Typical use |
|-------|-------------------------|-------------|
| `ReadSeek` | `Read + Seek` | readable random-access input |
| `BufReadSeek` | `BufRead + Seek` | buffered random-access input |
| `ReadWrite` | `Read + Write` | bidirectional stream or buffer |
| `WriteSeek` | `Write + Seek` | writable random-access output |
| `ReadWriteSeek` | `Read + Write + Seek` | fully mutable random-access I/O |

| Extension trait | Methods | Typical use |
|-----------------|---------|-------------|
| `ReadExt` | `read_exact_or_eof`, `discard_exact_or_eof`, `copy_to`, `copy_to_at_most`, `copy_to_end_limited`, `read_to_end_limited`, `read_to_end_limited_into`, `read_to_string_limited`, `read_to_string_limited_into` | short-read-safe reads, bounded copies, and bounded reads |
| `BufReadExt` | `read_until_limited`, `read_until_limited_into`, `read_line_limited`, `read_line_limited_into`, `discard_until_limited` | bounded delimiter and line operations |
| `SeekExt` | `stream_size` | size checks that keep the original cursor |
| `ReadSeekExt` | `peek_exact_or_eof`, `read_exact_or_eof_at` | non-consuming inspection and random-offset reads |
| `WriteSeekExt` | `write_all_at_preserving_position` | random-access patch writes |
| `BinaryReadExt` | `read_u16_be`, `read_u16_le`, `read_u16(order)`, and scalar variants through `u128` / `i128` | binary scalar decoding |
| `BinaryWriteExt` | `write_u16_be`, `write_u16_le`, `write_u16(value, order)`, and scalar variants through `u128` / `i128` | binary scalar encoding |
| `Leb128IntReadExt` | `read_uleb_u32`, `read_sleb_i32`, `_strict` variants, and other integer variants through 128-bit values | LEB128 integer decoding |
| `Leb128IntWriteExt` | `write_uleb_u32`, `write_sleb_i32`, and other integer variants through 128-bit values | LEB128 integer encoding |
| `ZigZagIntReadExt` | `read_zigzag_i32`, `read_zigzag_i128`, `_strict` variants, and other signed variants | ZigZag signed integer decoding |
| `ZigZagIntWriteExt` | `write_zigzag_i32`, `write_zigzag_i128`, and other signed variants | ZigZag signed integer encoding |
| `StringReadExt` | `read_utf8_string_uleb`, `read_utf8_string_u16_be`, `read_utf8_string_u16_le`, `read_utf8_string_u32_be`, `read_utf8_string_u32_le` | bounded length-prefixed UTF-8 decoding |
| `StringWriteExt` | `write_utf8_string_uleb`, `write_utf8_string_u16_be`, `write_utf8_string_u16_le`, `write_utf8_string_u32_be`, `write_utf8_string_u32_le` | length-prefixed UTF-8 encoding |

Each trait is implemented with a blanket implementation:

```rust
use std::io::{Read, Seek};

use qubit_io::ReadSeek;

fn accepts_read_seek<T>(value: &mut T) -> &mut dyn ReadSeek
where
    T: Read + Seek,
{
    value
}
```

## Object Safety Notes

Rust trait aliases are not stable, and a direct expression such as
`dyn Read + Seek` is not available for multiple non-auto traits in the way many
APIs need. Qubit IO solves this by defining a named trait with the desired
supertraits and implementing it for every matching type.

The composition traits do not add methods of their own. Method calls such as
`read_exact`, `write_all`, and `seek` come from the standard-library
supertraits.

## Extension Trait Notes

Extension methods are available after importing the corresponding trait:

```rust
use qubit_io::ReadExt;
```

The extension traits use `?Sized` blanket implementations, so any type
implementing the matching standard-library trait automatically receives the
methods. This also works for trait objects such as `&mut dyn std::io::Read`.

## Testing & Code Coverage

This project keeps tests focused on trait-object support, blanket
implementation behavior, short-read handling, EOF behavior, interrupted I/O
retry behavior, and position restoration semantics.

### Running Tests

```bash
# Run all tests
cargo test

# Run with coverage report
./coverage.sh

# Generate text format report
./coverage.sh text

# Run CI checks (format, clippy, test, coverage)
./ci-check.sh
```

## Dependencies

This crate has no runtime dependencies outside the Rust standard library.

## License

Copyright (c) 2026. Haixing Hu, Qubit Co. Ltd. All rights reserved.

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

- Follow the Rust API guidelines.
- Keep utilities generic and independent from domain crates.
- Document public APIs with examples when they clarify usage.
- Run `./ci-check.sh` before submitting PRs.

## Author

**Haixing Hu** - *Qubit Co. Ltd.*

## Related Projects

More Rust libraries from Qubit are published under the
[qubit-ltd](https://github.com/qubit-ltd) organization on GitHub.

---

Repository: [https://github.com/qubit-ltd/rs-io](https://github.com/qubit-ltd/rs-io)
