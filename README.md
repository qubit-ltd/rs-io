# Qubit IO

[![Rust CI](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io/coverage-badge.json)](https://qubit-ltd.github.io/rs-io/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io.svg?color=blue)](https://crates.io/crates/qubit-io)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Chinese Document](https://img.shields.io/badge/Document-Chinese-blue.svg)](README.zh_CN.md)

Small stream I/O trait and extension utilities for Rust.

## Overview

Qubit IO is focused on stream and byte I/O built on top of `std::io`. It does
not try to become a filesystem abstraction layer. The crate provides reusable
building blocks for code that reads, writes, seeks, buffers, encodes, decodes,
limits, tees, counts, or compares byte streams.

Use this crate when you need:

- object-safe trait aliases for common `std::io` capability combinations;
- extension methods for exact reads, bounded reads, bounded delimiter reads, and
  position-preserving seek operations;
- small stream helper functions such as bounded copy and content comparison;
- binary, LEB128, ZigZag, and length-prefixed UTF-8 encoding helpers;
- wrapper types for counting, limiting, teeing, checksumming, and seek-position
  restoration;
- reader/writer codec objects, including buffered variants for high-throughput
  scalar decoding and encoding.

For detailed usage, examples, and API selection guidance, see the [User Guide](doc/user_guide.md).
API reference documentation is available on [docs.rs](https://docs.rs/qubit-io).

For local filesystem capabilities, see [qubit-local-files](https://github.com/qubit-ltd/rs-local-files).

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

let data = Cursor::new(b"bounded".to_vec()).read_to_end_limited(16)?;
assert_eq!(b"bounded", data.as_slice());

# Ok::<(), std::io::Error>(())
```

## Main Capabilities

### Object-Safe I/O Trait Combinations

`qubit-io` provides named composition traits that can be used as trait objects:

| Trait | Combines | Typical use |
| --- | --- | --- |
| `ReadSeek` | `Read + Seek` | readable random-access input |
| `BufReadSeek` | `BufRead + Seek` | buffered random-access input |
| `ReadWrite` | `Read + Write` | duplex stream or mutable buffer |
| `WriteSeek` | `Write + Seek` | writable random-access output |
| `ReadWriteSeek` | `Read + Write + Seek` | fully mutable random-access I/O object |

These traits are useful when an API should accept `&mut dyn ReadSeek` instead
of being generic over `R: Read + Seek`.

### Buffer Codecs

The `codec` module contains buffer-level helpers that do not depend on
`std::io::Read` or `std::io::Write`:

| Type | Purpose |
| --- | --- |
| `Coder` | progress-oriented conversion trait for caller-managed buffers |
| `CoderProgress`, `CoderStatus` | conversion progress and stop reason reporting |
| `BinaryCodec` | unchecked fixed-width scalar encoding and decoding on caller-validated byte buffers |
| `Leb128Codec`, `ZigZagCodec` | unchecked compact integer encoding on caller-validated byte buffers |
| `Leb128DecodeError` | structured LEB128 decode error reporting |

`BinaryCodec`, `Leb128Codec`, and `ZigZagCodec` expose static `unsafe`
`read_unchecked` and `write_unchecked` functions for hot paths where the caller
has already validated buffer capacity. Each concrete codec specialization
provides `REQUIRED_MIN_BUFFER_LEN`, the minimum caller-provided buffer capacity
that can hold one value for that specialization.

### Extension Traits

The extension traits keep common low-level I/O patterns close to the standard
library while avoiding repeated boilerplate:

| Extension trait | Examples |
| --- | --- |
| `ReadExt` | `read_exact_or_eof`, `discard_exact_or_eof`, `copy_to`, `read_to_end_limited`, `read_to_string_limited` |
| `BufReadExt` | `read_until_limited`, `read_line_limited`, `discard_until_limited` |
| `SeekExt` | `stream_size` |
| `ReadSeekExt` | `peek_exact_or_eof`, `read_exact_or_eof_at` |
| `WriteSeekExt` | `write_all_at_preserving_position` |
| `BinaryReadExt` / `BinaryWriteExt` | fixed-width integer and floating-point scalar encoding |
| `Leb128ReadExt` / `Leb128WriteExt` | unsigned and signed LEB128 integer encoding |
| `ZigZagReadExt` / `ZigZagWriteExt` | ZigZag-mapped signed integer encoding |
| `StringReadExt` / `StringWriteExt` | length-prefixed UTF-8 strings |

`usize` and `isize` LEB128/ZigZag methods use the current Rust target's pointer
width. They are convenient for in-process Rust data, but fixed-width integer
methods such as `u64` or `i64` are the safer choice for persistent files and
cross-platform protocols. The ULEB string helpers encode the byte length as
`usize`; use the `u16` or `u32` string helpers when the wire format must be
target-independent.

### Streams Namespace

`Streams` contains stream-level associated functions:

| Method | Purpose |
| --- | --- |
| `copy` | delegates to `std::io::copy` |
| `copy_at_most` | copies no more than a specified number of bytes |
| `copy_to_end_limited` | copies only if EOF is reached within a size limit |
| `content_eq` | compares two readable streams for byte equality |
| `compare_content` | lexicographically compares two readable streams |

### Wrappers

Wrapper types make stream behavior part of the type instead of a one-off call:

| Wrapper | Purpose |
| --- | --- |
| `CountingReader`, `CountingWriter` | count successfully read or written bytes |
| `LimitReader`, `LimitWriter` | enforce read or write byte budgets |
| `TeeReader`, `TeeWriter` | duplicate data to a branch writer |
| `ChecksumReader`, `ChecksumWriter` | update caller-provided checksum state while reading or writing |
| `PositionGuard` | restore a seek position on drop unless dismissed |

### Codec Wrappers

Callers who prefer reader/writer objects over extension-method calls can use
root-level codec wrappers. These wrappers keep codec configuration in the
wrapper object while still behaving like normal streams: readers implement
`Read`, writers implement `Write`, and both pass through `Seek` when the inner
stream supports seeking.

| Wrapper | Purpose |
| --- | --- |
| `BinaryReader`, `BinaryWriter` | fixed-width scalar encoding and decoding with type-level byte order |
| `Leb128Reader`, `Leb128Writer` | LEB128 integers and ULEB length-prefixed UTF-8 strings |
| `ZigZagReader`, `ZigZagWriter` | ZigZag over unsigned LEB128 payloads |
| `BufferedBinaryReader`, `BufferedBinaryWriter` | buffered fixed-width scalar encoding and decoding |
| `BufferedLeb128Reader`, `BufferedLeb128Writer` | buffered LEB128 integers and ULEB length-prefixed UTF-8 strings |
| `BufferedZigZagReader`, `BufferedZigZagWriter` | buffered ZigZag over unsigned LEB128 payloads |

Buffered readers may prefetch bytes, so the wrapped reader's physical position
can be ahead of the logical position exposed by the wrapper. Calling
`into_inner` on a buffered reader discards unread prefetched bytes.

Buffered writers do not flush pending bytes from `Drop`. Call `flush()` or
`into_inner()` to guarantee delivery to the wrapped writer. Buffered writers
flush pending bytes before `Seek`.

## Prelude

`qubit_io::prelude` re-exports the method-providing extension traits,
object-safe composition traits, byte order, and buffer codec types. It
intentionally does not re-export stream wrapper types.

```rust
use qubit_io::prelude::*;
```

## Crate Boundary

`qubit-io` is intentionally limited to stream and byte I/O. It does not expose
local path helpers, temporary files, directory copy helpers, directory cleanup,
or atomic file writes. For local filesystem capabilities, see
[qubit-local-files](https://github.com/qubit-ltd/rs-local-files).

## Runtime Dependencies

This crate depends on the Rust standard library and `thiserror`.

## Testing & Code Coverage

This project maintains test coverage for the stream traits, extension methods,
codec helpers, and wrapper types.

### Running Tests

```bash
# Run all tests
cargo test

# Run with coverage report
./coverage.sh

# Generate text format report
./coverage.sh text

# Run CI checks (format, clippy, test, coverage, audit)
./ci-check.sh
```

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

Contributions are welcome. Please feel free to submit a Pull Request.

### Development Guidelines

- Follow the Rust API guidelines.
- Keep stream and byte-I/O concerns in `qubit-io`.
- Use [qubit-local-files](https://github.com/qubit-ltd/rs-local-files) for local filesystem utilities.
- Maintain comprehensive test coverage.
- Document public APIs with examples when they clarify behavior.
- Ensure `./ci-check.sh` passes before submitting a PR.

## Author

**Haixing Hu**

## Related Projects

- [qubit-local-files](https://github.com/qubit-ltd/rs-local-files): local filesystem utilities for Rust.
- More Rust libraries from Qubit are published under the [qubit-ltd](https://github.com/qubit-ltd) organization on GitHub.

---

Repository: [https://github.com/qubit-ltd/rs-io](https://github.com/qubit-ltd/rs-io)
