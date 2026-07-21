# Qubit IO

[![Rust CI](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/qubit-io.svg?color=blue)](https://crates.io/crates/qubit-io)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

Qubit IO provides runtime-neutral synchronous and asynchronous item streams.
It is the transport layer shared by Qubit filesystem, binary, and text crates.

The central traits are deliberately smaller than a filesystem abstraction:
they move items and report `std::io::Error`; they do not imply file identity,
paths, commit, abort, or persistence semantics.

## Core API

| Concern | Synchronous | Asynchronous |
| --- | --- | --- |
| Input | `Input<Item = T>` | `AsyncInput<Item = T>` |
| Output | `Output<Item = T>` | `AsyncOutput<Item = T>` |
| Convenience | `read_fully`, `write_fully` | Defaults on `AsyncInput`, `AsyncOutput` |
| Buffering | `BufferedInput`, `BufferedOutput` | `AsyncBufferedInput`, `AsyncBufferedOutput` |
| Limits | `LimitReader`, `LimitWriter` for std streams | `AsyncLimitInput`, `AsyncLimitOutput` |
| Counters | `CountingReader`, `CountingWriter` | `AsyncCountingInput`, `AsyncCountingOutput` |
| Checksums | `ChecksumReader`, `ChecksumWriter` | `AsyncChecksumInput`, `AsyncChecksumOutput` |

`AsyncInput` and `AsyncOutput` use `Pin`, `Context`, and `Poll`. They do not
depend on Tokio, `futures-io`, or an executor. Multi-poll operations are named
futures such as `ReadFullyFuture` and `WriteFullyFuture`, so progress survives
`Pending`.

## Synchronous example

All `std::io::Read` byte streams implement `Input<Item = u8>`, and all
`std::io::Write` byte streams implement `Output<Item = u8>`.

```rust
use std::io::Cursor;
use qubit_io::{Input, Output};

let mut input = Cursor::new(b"qubit".to_vec());
let mut bytes = [0_u8; 5];
assert_eq!(5, input.read_fully(&mut bytes)?);

let mut output = Vec::new();
output.write_fully(&bytes)?;
assert_eq!(b"qubit", output.as_slice());
# Ok::<(), std::io::Error>(())
```

`Input` and `Output` remain generic over the item type, so codecs can also use
`u16`, `char`, or another cheap scalar unit without converting through bytes.

## Asynchronous example

The optional Tokio adapter is an explicit newtype. This avoids coherence
conflicts when a stream implements more than one async ecosystem trait.

```rust,ignore
use qubit_io::{AsyncInput, TokioInput};

let socket = /* a tokio::io::AsyncRead value */;
let mut input = TokioInput::new(socket);
let mut header = [0_u8; 16];
let read = input.read_fully_async(&mut header).await?;
```

Reverse adapters are also available: `TokioAsyncRead` and `TokioAsyncWrite`
expose Qubit streams to Tokio, while `FuturesAsyncRead` and
`FuturesAsyncWrite` expose them to `futures-io`.

## Buffering and composition

`Buffer<T>` is a low-level readable-window container. The synchronous and
asynchronous buffered wrappers build on the same position/limit model.

`AsyncBufferedOutput` owns every accepted item until the inner output accepts
it. A partial flush updates retained progress before returning `Pending`.
Dropping an asynchronous buffer cannot perform I/O; call `flush_async()` or use
`into_parts()` to recover pending items.

Limit and counting wrappers are item-oriented. Checksum wrappers are byte-only
because `std::hash::Hasher` consumes bytes.

## Features

```toml
[dependencies]
qubit-io = "0.14"
```

- Default features: runtime-neutral core only.
- `tokio`: adapters in both directions for Tokio I/O traits.
- `futures-io`: adapters in both directions for `futures-io` traits.

## Documentation and checks

- [User guide](doc/user_guide.md)
- [用户指南](doc/user_guide.zh_CN.md)

```bash
cargo test --no-default-features
cargo test --all-features
./align-ci.sh
./ci-check.sh
```

## License

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).
