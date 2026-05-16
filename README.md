# Qubit IO

[![Rust CI](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io/coverage-badge.json)](https://qubit-ltd.github.io/rs-io/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io.svg?color=blue)](https://crates.io/crates/qubit-io)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Chinese Document](https://img.shields.io/badge/Document-Chinese-blue.svg)](README.zh_CN.md)

Small I/O trait utilities for Rust.

## Overview

Qubit IO provides object-safe composition traits for common `std::io` trait
combinations. It is useful when an API needs a trait object such as
`&mut dyn ReadSeek` or `Box<dyn ReadWriteSeek>` instead of a generic bound like
`R: Read + Seek`.

The crate intentionally stays tiny: it does not wrap readers or writers, does
not allocate, and does not introduce new I/O behavior. It only names common
combinations of standard-library traits so they can be used as trait objects.

## Design Goals

- **Object-safe composition**: provide named trait-object-friendly I/O bounds.
- **Standard-library first**: build directly on `std::io::{Read, Write, Seek}`.
- **Zero runtime overhead**: use blanket implementations with no wrapper type.
- **Tiny API surface**: keep only the combinations that are commonly reused.
- **Integration friendly**: work with cursors, files, buffers, streams, and
  custom types implementing the standard I/O traits.

## Features

### Object-Safe I/O Trait Combinations

- **`ReadSeek`**: combines `Read` and `Seek` for readable random-access inputs.
- **`ReadWrite`**: combines `Read` and `Write` for duplex streams or buffers.
- **`WriteSeek`**: combines `Write` and `Seek` for writable random-access
  outputs.
- **`ReadWriteSeek`**: combines `Read`, `Write`, and `Seek` for fully mutable
  random-access I/O objects.

### Blanket Implementations

Every type that implements the corresponding standard-library traits
automatically implements the Qubit IO composition trait. You do not need to
write adapter code for `std::io::Cursor`, `std::fs::File`, or your own I/O
types.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
qubit-io = "0.1"
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

| Trait | Standard-library bounds | Typical use |
|-------|-------------------------|-------------|
| `ReadSeek` | `Read + Seek` | readable random-access input |
| `ReadWrite` | `Read + Write` | bidirectional stream or buffer |
| `WriteSeek` | `Write + Seek` | writable random-access output |
| `ReadWriteSeek` | `Read + Write + Seek` | fully mutable random-access I/O |

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

The traits do not add methods of their own. Method calls such as `read_exact`,
`write_all`, and `seek` come from the standard-library supertraits.

## Testing & Code Coverage

This project keeps tests focused on trait-object support and blanket
implementation behavior.

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
