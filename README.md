# Qubit IO

[![Rust CI](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io/coverage-badge.json)](https://qubit-ltd.github.io/rs-io/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io.svg?color=blue)](https://crates.io/crates/qubit-io)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

Qubit IO lets codecs, protocols, and storage libraries expose I/O without choosing their caller's async runtime. Callers can add buffering, limits, observability, and checksums outside the codec instead of embedding those policies in every transport implementation.

It is a transfer layer, not a file-system abstraction: it does not model paths, file identity, commit, abort, or persistence. Use a higher-level crate such as `qubit-fs` when those lifecycle semantics are part of the contract.

## Quick start

```toml
[dependencies]
qubit-io = "0.15"
```

All standard `Read` and `Write` values already implement the synchronous Qubit
traits. A library can therefore accept `Input<Item = u8>` without coupling its
algorithm to a file, socket, or async runtime:

```rust
use std::io::{self, Cursor};
use qubit_io::Input;

fn read_magic<I: Input<Item = u8>>(input: &mut I) -> io::Result<[u8; 4]> {
    let mut magic = [0_u8; 4];
    input.read_exactly(&mut magic)?;
    Ok(magic)
}

let mut input = Cursor::new(*b"QBIT");
assert_eq!(*b"QBIT", read_magic(&mut input)?);
# Ok::<(), io::Error>(())
```

For complete, checked programs, see the [bounded-frame example](examples/bounded_frame.rs)
and the [typed-record example](examples/typed_records.rs).

## Why this abstraction exists

Native I/O traits are the simplest choice for a single application using one runtime. A library has a different boundary: choosing Tokio in its public API excludes `futures-io` users, while choosing byte-only I/O forces text and data pipelines to encode their logical items before the algorithm can process them.

Qubit IO keeps that boundary small:

- `Input` and `Output` move synchronous items.
- `AsyncInput` and `AsyncOutput` move asynchronous items without selecting an executor.
- Buffers and wrappers add one transfer policy at a time.
- Explicit adapters connect the boundary to `std::io`, Tokio, and `futures-io`.

The result is not an attempt to replace native I/O. It is a stable library boundary when the transport, runtime, or item type belongs to the caller.

## One async API, caller-chosen runtime

A library writes its asynchronous algorithm against `AsyncInput` or
`AsyncOutput` once. A Tokio caller supplies `TokioInput` or `TokioOutput`; a
`futures-io` caller supplies the corresponding `Futures*` adapter. The blocking
driver remains a separate function, but protocol limits and validation rules
stay the same. The [user guide](doc/user_guide.md) includes the adapter
direction table and a bounded-frame case study.

## Policies stay outside the codec

The bounded-frame decoder only knows the wire format. Its caller chooses the transfer policy:

```text
transport adapter -> limit -> buffer -> counting -> frame decoder
```

`AsyncLimitInput` caps a connection's total exposed bytes. `AsyncBufferedInput` batches transport reads. `AsyncCountingInput` reports the bytes the decoder actually consumed. The protocol's 64 KiB frame check remains necessary: a transport budget is not a format-validation rule.

The synchronous side has the same vocabulary: limit, counting, checksum, tee, and buffers. Wrapper order is meaningful. A counter outside a buffer measures what the decoder consumed; a counter inside the buffer measures what the transport supplied, including prefetched bytes. The [user guide](doc/user_guide.md) walks through those choices and their recovery implications.

## Typed streams, not only bytes

`Item` is generic. A data-processing operator can consume business records and
emit mapped records without serializing them at every operator boundary. Limits
and counters are then measured in records rather than bytes, and `TeeOutput`
can mirror those records to a shuffle sink and an audit sink. The
[typed-record example](examples/typed_records.rs) demonstrates the full
pipeline and the `Clone + Default` requirement of generic buffering.

## When to use Qubit IO

| Situation | Recommendation |
| --- | --- |
| A library must support Tokio and `futures-io` callers | Expose Qubit async traits and let callers use adapters. |
| Transfer policies must be composed independently of a codec | Use Qubit buffers and wrappers. |
| The stream carries `char`, typed business records, or another logical item | Use generic `Input` and `Output`. |
| One application uses one runtime and ordinary byte streams | Prefer native I/O traits. |
| The contract includes paths, file identity, commit, or persistence | Use a higher-level file-system abstraction. |

## API map

| Concern | Synchronous | Asynchronous |
| --- | --- | --- |
| Transfer | `Input<Item = T>`, `Output<Item = T>` | `AsyncInput<Item = T>`, `AsyncOutput<Item = T>` |
| Buffering | `BufferedInput`, `BufferedOutput` | `AsyncBufferedInput`, `AsyncBufferedOutput` |
| Observability and limits | limit, counting, checksum, tee | async limit, counting, checksum |
| Ecosystem bridges | `qubit_io::std_io` | optional Tokio and `futures-io` adapters |

`AsyncClose` represents a real transport shutdown. It is deliberately separate from flushing buffered output.

## Features

- Default features: runtime-neutral core only.
- `tokio`: adapters in both directions for Tokio I/O traits.
- `futures-io`: adapters in both directions for `futures-io` traits.

## Documentation

- [API documentation](https://docs.rs/qubit-io)
- [User guide](doc/user_guide.md) | [中文用户指南](doc/user_guide.zh_CN.md)
- [Bounded-frame example](examples/bounded_frame.rs) | [typed-record example](examples/typed_records.rs)

The API documentation is built with all declared features enabled.

## Testing

```bash
# Run tests with the default feature set
cargo test

# Run tests with all declared features
cargo test --all-features

# Project CI checks
./ci-check.sh

# Check code coverage
./coverage.sh
```

## License

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for the
full license text.

## Contributing

Contributions are welcome. Please follow the Rust API guidelines, keep public
API documentation and tests current, and run `./align-ci.sh` to format code and
`./ci-check.sh` to satisfy CI requirements before submitting a pull request.

## Author

**Haixing Hu** - *Qubit Co. Ltd.*

Repository: [https://github.com/qubit-ltd/rs-io](https://github.com/qubit-ltd/rs-io)
