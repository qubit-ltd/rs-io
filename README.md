# Qubit IO

[![Rust CI](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io/coverage-badge.json)](https://qubit-ltd.github.io/rs-io/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io.svg?color=blue)](https://crates.io/crates/qubit-io)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

Qubit IO provides a generic input/output stream system for Rust, built around
`Input` and `Output`. Their associated `Item` type extends stream I/O from bytes
to characters, business records, and other Rust types. Library and application
authors can write transfer and processing algorithms directly over those items,
without serializing them at each processing boundary.

Standard `Read` and `Write` values integrate with this system automatically.
Buffers, limits, counters, checksums, digests, and tee wrappers compose around
the stream interfaces. `AsyncInput` and `AsyncOutput` extend the model to
runtime-neutral asynchronous I/O, with optional Tokio and `futures-io` adapters.

## Core model: streams of items

| Interface | Operation | Item type |
| --- | --- | --- |
| `Input<Item = T>` | Synchronous input | Any `T` supported by the implementation |
| `Output<Item = T>` | Synchronous output | Any `T` supported by the implementation |
| `AsyncInput<Item = T>` | Asynchronous input | Any `T` supported by the implementation |
| `AsyncOutput<Item = T>` | Asynchronous output | Any `T` supported by the implementation |

An input reads items into a slice; an output writes items from a slice. With
`Item = Sale`, a read count of 10 means ten sales records. With `Item = u8`, it
means ten bytes. Limits and counters follow the same item unit.

The core traits do not require `Item: Clone + Default`; individual adapters
and wrappers impose the bounds they need. Generic buffering requires
`Clone + Default`, while checksum and digest wrappers operate on bytes.

## Installation

Requires Rust 1.94 or later. The default feature set enables no async ecosystem
adapter or digest dependency.

```toml
[dependencies]
qubit-io = "0.19"
```

For development against an unpublished checkout, use a local `path` dependency
pointing to this repository and keep the matching `version` requirement, for
example `qubit-io = { path = "../rs-io", version = "0.19" }` from a sibling
checkout. The path form is for workspace development; published consumers
should use the Crates.io dependency above.

## Quick start: process business records

Consider a sales pipeline that emits category revenue records. Encoding every
record into bytes between operators adds a serialization boundary where the
algorithm only needs typed data. Instead, the mapper accepts a sales input and
a revenue output. Its caller selects the source, destination, and policies:

```text
sales source -> limit (2 records) -> buffer -> counter -> mapper -> counter -> revenue sink
```

This complete program uses application-defined in-memory record adapters.
Qubit IO supplies the traits, buffer, limit, counters, and checked read/write
helpers:

<!-- example: sales-pipeline profile=core mode=run -->
```rust
use std::io;
use qubit_io::{
    BufferedInput, CountingInput, CountingOutput, Input, LimitInput, Output,
};

#[derive(Clone, Debug, Default, PartialEq)]
struct Sale {
    category: u16,
    cents: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Revenue {
    category: u16,
    cents: u64,
}

// Application adapters: these transfer whole records, not encoded bytes.
struct RecordInput<T> {
    records: Vec<T>,
    position: usize,
}

impl<T: Clone> Input for RecordInput<T> {
    type Item = T;

    // SAFETY: The caller must provide a valid destination range.
    unsafe fn read_unchecked(
        &mut self, output: &mut [T], index: usize, count: usize,
    ) -> io::Result<usize> {
        let n = count.min(self.records.len() - self.position);
        output[index..index + n]
            .clone_from_slice(&self.records[self.position..self.position + n]);
        self.position += n;
        Ok(n)
    }
}

struct RecordOutput<T>(Vec<T>);

impl<T: Clone> Output for RecordOutput<T> {
    type Item = T;

    // SAFETY: The caller must provide a valid source range.
    unsafe fn write_unchecked(
        &mut self, input: &[T], index: usize, count: usize,
    ) -> io::Result<usize> {
        self.0.extend_from_slice(&input[index..index + count]);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn map_sales<I, O>(input: &mut I, output: &mut O) -> io::Result<()>
where
    I: Input<Item = Sale>,
    O: Output<Item = Revenue>,
{
    let mut batch = [Sale::default(); 1];
    while input.read(&mut batch)? != 0 {
        let sale = &batch[0];
        output.write_fully(&[Revenue {
            category: sale.category,
            cents: sale.cents,
        }])?;
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let source = RecordInput {
        records: vec![
            Sale { category: 7, cents: 300 },
            Sale { category: 7, cents: 500 },
            Sale { category: 9, cents: 900 },
        ],
        position: 0,
    };
    let limited = LimitInput::new(source, 2);
    let buffered = BufferedInput::with_capacity(limited, 2);
    let mut input = CountingInput::new(buffered);
    let mut output = CountingOutput::new(RecordOutput(Vec::new()));

    map_sales(&mut input, &mut output)?;
    output.flush()?;
    assert_eq!(input.items_read(), 2);
    assert_eq!(output.items_written(), 2);
    assert_eq!(output.into_inner().0, vec![
        Revenue { category: 7, cents: 300 },
        Revenue { category: 7, cents: 500 },
    ]);
    Ok(())
}
```

The assertions check both the resulting records and the transfer counts: the
third sale is excluded by the limit. No record serialization occurs in this
pipeline. The mapper uses safe methods; the adapters implement the low-level
indexed transfer contract, whose `unsafe` methods require valid slice ranges.

A real application supplies adapters for its own record sources and sinks;
Qubit IO does not provide a record storage engine. The
[typed-record example](examples/typed_records.rs) also mirrors output records
to two sinks with `TeeOutput`.

## Shorter bounds for byte streams

Use `ByteInput` and `ByteOutput` when an API transfers bytes. They provide
shorter bounds for `Input<Item = u8>` and `Output<Item = u8>`, respectively,
with the same inherited operations:

<!-- example: byte-bounds profile=core mode=run -->
```rust
use std::io::{self, Cursor};
use qubit_io::{ByteInput, ByteOutput};

fn input() -> impl ByteInput {
    Cursor::new(b"QBIT")
}

fn copy_magic<I: ByteInput + ?Sized, O: ByteOutput + ?Sized>(
    input: &mut I, output: &mut O,
) -> io::Result<()> {
    let mut magic = [0; 4];
    input.read_exactly(&mut magic)?;
    output.write_fully(&magic)
}

fn main() -> io::Result<()> {
    let mut bytes = Vec::new();
    copy_magic(&mut input(), &mut bytes)?;
    assert_eq!(bytes, b"QBIT");
    Ok(())
}
```

Existing byte inputs and outputs automatically satisfy these traits, including
standard `Read` and `Write` implementations. Implementers still implement
`Input` or `Output`; no separate byte-trait implementation is needed.
Return `impl ByteInput` / `impl ByteOutput` to hide a concrete type, or use
`dyn ByteInput` / `dyn ByteOutput` behind a reference or box for dynamic dispatch.
See the [byte-stream guide](doc/user_guide.md#4-connect-standard-library-byte-streams)
for borrowed outputs and standard-library conversion.

The same short-bound pattern covers asynchronous bytes and Unicode scalar
values:

| Mode and item type | Input bound | Output bound |
| --- | --- | --- |
| Synchronous `u8` | `ByteInput` | `ByteOutput` |
| Asynchronous `u8` | `AsyncByteInput` | `AsyncByteOutput` |
| Synchronous `char` | `CharInput` | `CharOutput` |
| Asynchronous `char` | `AsyncCharInput` | `AsyncCharOutput` |

Each trait automatically applies to implementations of its corresponding base
trait with the matching `Item`. For example, use `I: CharInput` instead of
`I: Input<Item = char>`, or `O: AsyncByteOutput` instead of
`O: AsyncOutput<Item = u8>`. Opaque return types use `impl CharInput` or
`impl AsyncByteOutput` in the same way. Character streams transfer Rust `char`
values (Unicode scalar values), without encoding or decoding text. Async
polling still uses `Pin` and `Poll`, and does not require `Unpin`.

## Compatible with standard Read and Write

Every `std::io::Read` value already implements `Input<Item = u8>`, and every
`std::io::Write` value implements `Output<Item = u8>`. Files, sockets, `Cursor`,
and `Vec<u8>` can therefore participate without an extra input/output adapter.
In the reverse direction, `StdRead` and `StdWrite` expose Qubit byte streams
through the standard traits:

<!-- example: standard-bridge profile=core mode=run -->
```rust
use std::io::{self, Cursor, Read};
use qubit_io::{CountingInput, Input, Output, StdRead};

fn read_magic<I: Input<Item = u8>>(input: &mut I) -> io::Result<[u8; 4]> {
    let mut magic = [0; 4];
    input.read_exactly(&mut magic)?;
    Ok(magic)
}

fn main() -> io::Result<()> {
    let mut bytes = Vec::new();
    bytes.write_fully(b"QBITpayload")?;
    let mut input = CountingInput::new(Cursor::new(bytes));
    assert_eq!(read_magic(&mut input)?, *b"QBIT");
    assert_eq!(input.bytes_read(), 4);

    // Present the remaining Qubit input to an API that expects std::io::Read.
    let mut reader = StdRead::new(input);
    let mut rest = String::new();
    reader.read_to_string(&mut rest)?;
    assert_eq!(rest, "payload");
    assert_eq!(reader.get_ref().bytes_read(), 11);
    Ok(())
}
```

`InputRef` and `OutputRef` support borrowing an existing Qubit stream, including
a trait object, before conversion. Standard-library compatibility applies to
byte streams; converting typed records to bytes still requires an encoding.

## Compose stream wrappers

A wrapper implements the corresponding stream interface, so algorithms can
accept a wrapped stream without knowing which policies surround it. The quick
start's `CountingInput<BufferedInput<LimitInput<_>>>` is still an
`Input<Item = Sale>`.

| Capability | Synchronous types | Asynchronous types | Item constraints |
| --- | --- | --- | --- |
| Buffering | `BufferedInput`, `BufferedOutput` | `AsyncBufferedInput`, `AsyncBufferedOutput` | `Clone + Default` |
| Limits | `LimitInput`, `LimitOutput` | `AsyncLimitInput`, `AsyncLimitOutput` | Generic items |
| Counting | `CountingInput`, `CountingOutput` | `AsyncCountingInput`, `AsyncCountingOutput` | Generic items |
| Checksums | `ChecksumInput`, `ChecksumOutput` | `AsyncChecksumInput`, `AsyncChecksumOutput` | `u8`; caller-supplied `std::hash::Hasher` |
| Content digests | `DigestInput`, `DigestOutput` | `AsyncDigestInput`, `AsyncDigestOutput` | `u8`; `digest` feature and caller-supplied algorithm |
| Tee | `TeeInput`, `TeeOutput`, `SyncSeekTeeInput` | — | Generic items; synchronized seeking requires seekable source and branch |

Wrapper order determines what is observed. A counter outside an input buffer
counts items consumed by the algorithm; a counter inside it also counts
prefetched items. A limit bounds the items exposed or accepted by its wrapper;
it does not replace application validation such as checking a message's declared
length before allocation. See the [bounded-frame example](examples/bounded_frame.rs).

Flush buffered output explicitly and check the result. Dropping synchronous
buffered output only attempts a best-effort drain; dropping asynchronous
buffered output cannot perform asynchronous I/O. The
[user guide](doc/user_guide.md) explains wrapper order, partial progress, and
recovery of unread or pending items.

Synchronous buffered output exposes recovery as `Pending` or `Indeterminate`.
The latter follows a panic during inner I/O; check external side effects before
deciding whether any retained items can be replayed. See the user guide for the
recovery path and its limits.

### Content digests

For a SHA-256 fingerprint, enable `digest` and add a compatible algorithm:

```toml
[dependencies]
qubit-io = { version = "0.19", features = ["digest"] }
sha2 = "0.11"
```

<!-- example: digest-fingerprint profile=digest mode=run -->
```rust
use std::io::{self, Cursor};
use qubit_io::DigestInput;
use sha2::{Digest, Sha256};

fn main() -> io::Result<()> {
    let mut input = Cursor::new(b"hello world");
    let fingerprint = DigestInput::<_, Sha256>::digest_all(&mut input)?;
    assert_eq!(fingerprint, Sha256::digest(b"hello world"));
    Ok(())
}
```

`digest_all` reads from the current position to EOF using a fixed-size buffer;
it fingerprints the entire original input only when started at its beginning.
An `Interrupted` read is retried without losing the digest accumulated from
earlier reads. Other read errors are returned without a partial digest, and the
input remains at the position already reached.
For incremental tracking, wrap a byte stream with `DigestInput::new` or
`DigestOutput::new`. Algorithms must use the compatible `digest` 0.11 interface;
Qubit IO does not choose the algorithm for the application.

Digest wrappers track successful transfers in order, including repeated reads
or overwriting writes after seeking. Their result is not automatically a
fingerprint of final file contents. `finalize()` does not read to EOF, flush,
or close. See the [digest guide](doc/user_guide.md#content-digests).

## Asynchronous input and output

`AsyncInput` and `AsyncOutput` retain the generic item model and expose readiness
through `Pin` and `Poll`. Callers normally use helpers such as
`read_exactly_async` and `write_fully_async`; pinned `!Unpin` values and trait
objects use `PinnedAsyncInputExt` and `PinnedAsyncOutputExt`.

An asynchronous algorithm can accept either Tokio or `futures-io` sources via
adapters. For example, this complete program runs a generic async reader with
Tokio:

```toml
[dependencies]
qubit-io = { version = "0.19", features = ["tokio"] }
tokio = { version = "1", features = ["macros", "rt", "io-util"] }
```

<!-- example: tokio-reader profile=tokio mode=run -->
```rust
use std::io;
use qubit_io::{AsyncCountingInput, AsyncInput, TokioInput};

async fn read_magic<I>(input: &mut I) -> io::Result<[u8; 4]>
where
    I: AsyncInput<Item = u8> + Unpin,
{
    let mut magic = [0; 4];
    input.read_exactly_async(&mut magic).await?;
    Ok(magic)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let source = TokioInput::new(&b"QBIT"[..]);
    let mut input = AsyncCountingInput::new(source);
    assert_eq!(read_magic(&mut input).await?, *b"QBIT");
    assert_eq!(input.bytes_read(), 4);
    Ok(())
}
```

The algorithm depends on `AsyncInput`, while the application selects Tokio.
A `futures-io` caller can pass `FuturesInput` to the same function. Synchronous
and asynchronous algorithms have separate entry points.

| Conversion | Tokio adapter | futures-io adapter |
| --- | --- | --- |
| Ecosystem `AsyncRead` → Qubit `AsyncInput<Item = u8>` | `TokioInput` | `FuturesInput` |
| Ecosystem `AsyncWrite` → Qubit `AsyncOutput<Item = u8>` | `TokioOutput` | `FuturesOutput` |
| Qubit `AsyncInput<Item = u8>` → ecosystem `AsyncRead` | `TokioAsyncRead` | `FuturesAsyncRead` |
| Qubit `AsyncOutput<Item = u8>` → ecosystem `AsyncWrite` | `TokioAsyncWrite` | `FuturesAsyncWrite` |

`AsyncClose` is a separate capability for graceful output closure, including
any required buffered output and underlying close operation. Flushing alone
does not close a transport. The [user guide](doc/user_guide.md) covers polling,
pinning, and cancellation behavior.

## Other stream tools

| Tool | Purpose |
| --- | --- |
| `Seekable`, `SeekableInput`, `SeekableOutput` | Synchronous positioning in stream item units |
| `PositionGuard` | Restore a saved position; use explicit `restore()` to observe restoration errors |
| `Streams` | Copy generic item streams; copy, compare, and check equality of standard byte streams |
| `std_io::ext` | Extend standard I/O with bounded reads, line reads, copying, and position-preserving operations |
| `std_io` combination traits | Express standard capabilities together, such as `ReadSeek` and `ReadWriteSeek` |
| `Buffer`, `EnsuredBufferedInput`, `EnsuredBufferedOutput` | Low-level buffer storage and conditional synchronous buffering |
| `BoxInput`, `BoxOutput`, `BoxAsyncInput`, `BoxAsyncOutput` | Boxed stream adapters |

## Scope and when to use it

Use Qubit IO when an algorithm should operate on typed item streams, transfer
policies need to compose independently, or an async library must leave runtime
selection to its caller. An application using only ordinary byte streams and
one runtime may need only the native I/O traits.

The crate provides stream transfer and related tools. Binary values and text
encodings belong to [qubit-io-binary](https://github.com/qubit-ltd/rs-io-binary) and
[qubit-io-text](https://github.com/qubit-ltd/rs-io-text), respectively. Paths, file identity,
metadata, commit, abort, and persistence belong to a higher-level file-system
abstraction such as [qubit-fs](https://github.com/qubit-ltd/rs-fs).

## Features and documentation

| Feature | Enables |
| --- | --- |
| Default: none | Core synchronous and asynchronous traits, buffering, standard I/O integration, and wrappers other than digests |
| `digest` | Synchronous/asynchronous content digest wrappers and read-to-EOF digest helpers |
| `tokio` | Bidirectional Tokio byte I/O adapters |
| `futures-io` | Bidirectional futures-io byte I/O adapters |

- [API documentation](https://docs.rs/qubit-io) — built with all declared features enabled.
- [User guide](doc/user_guide.md) | [中文用户指南](doc/user_guide.zh_CN.md)
- [Typed-record example](examples/typed_records.rs) | [bounded-frame example](examples/bounded_frame.rs)
- [中文 README](README.zh_CN.md)

## Testing

```bash
# Run tests with the default feature set
cargo test

# Run tests with all declared features
cargo test --all-features

# Project CI checks
./.infra/bin/ci-check.sh

# Check code coverage
./.infra/bin/coverage.sh
```

## License

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for the
full license text.

## Contributing

Contributions are welcome. Please follow the Rust API guidelines, keep public
API documentation and tests current, and run `./.infra/bin/align-ci.sh` to format code and
`./.infra/bin/ci-check.sh` to satisfy CI requirements before submitting a pull request.

## Author

**Haixing Hu** - *Qubit Co. Ltd.*

Repository: [https://github.com/qubit-ltd/rs-io](https://github.com/qubit-ltd/rs-io)
