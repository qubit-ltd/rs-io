# Qubit IO

[![Rust CI](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io/coverage-badge.json)](https://qubit-ltd.github.io/rs-io/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io.svg?color=blue)](https://crates.io/crates/qubit-io)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![中文文档](https://img.shields.io/badge/文档-中文版-blue.svg)](README.zh_CN.md)

Qubit IO provides runtime-neutral synchronous and asynchronous item streams.
It is the transport layer shared by Qubit filesystem, binary, and text crates.

The central traits deliberately stop at transfer: they move items and report
`std::io::Error`; they do not model file identity, paths, commit, abort, or
persistence. Use the [user guide](doc/user_guide.md) for the complete contract,
buffer ownership rules, wrapper composition, and adapter guidance.

## Core API

| Concern | Synchronous | Asynchronous |
| --- | --- | --- |
| Transfer | `Input<Item = T>`, `Output<Item = T>` | `AsyncInput<Item = T>`, `AsyncOutput<Item = T>` |
| Buffering | `BufferedInput`, `BufferedOutput` | `AsyncBufferedInput`, `AsyncBufferedOutput` |
| Wrappers | limit, counting, checksum, tee | limit, counting, checksum |
| Ecosystem bridges | `std::io` blanket implementations | optional Tokio and `futures-io` adapters |

## Synchronous example

All `std::io::Read` byte streams implement `Input<Item = u8>`, and all
`std::io::Write` byte streams implement `Output<Item = u8>`.

```rust
use std::io::Cursor;
use qubit_io::{Input, Output};

let mut input = Cursor::new(b"qubit".to_vec());
let mut bytes = [0_u8; 5];
input.read_exactly(&mut bytes)?;

let mut output = Vec::new();
output.write_fully(&bytes)?;
assert_eq!(b"qubit", output.as_slice());
# Ok::<(), std::io::Error>(())
```

`Input` and `Output` remain generic over the item type, so codecs can also use
`u16`, `char`, or another cheap scalar unit without converting through bytes.

## Features

```toml
[dependencies]
qubit-io = "0.14"
```

- Default features: runtime-neutral core only.
- `tokio`: adapters in both directions for Tokio I/O traits.
- `futures-io`: adapters in both directions for `futures-io` traits.

## Documentation

- [User guide](doc/user_guide.md)
- [中文用户指南](doc/user_guide.zh_CN.md)
- [Changelog](CHANGELOG.md)

API documentation on docs.rs is built with all declared features enabled.

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
