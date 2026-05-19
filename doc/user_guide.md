# Qubit IO User Guide

Qubit IO is the stream and byte-I/O crate in the Qubit Rust family. It focuses on
`std::io` traits, extension methods, wrappers, and small codec helpers.

Filesystem-oriented helpers have moved to `qubit-local-fs`.

## Imports

Use the crate root for concrete wrappers and utility namespaces:

```rust
use qubit_io::{
    CountingReader,
    ReadExt,
    ReadSeek,
    Streams,
};
```

Use the prelude when a module mostly needs method-providing extension traits and
composition traits:

```rust
use qubit_io::prelude::*;
```

## Stream Helpers

`Streams` provides associated functions around `std::io::Read` and
`std::io::Write`:

- `copy` delegates to `std::io::copy`;
- `copy_at_most` copies no more than a specified number of bytes;
- `copy_to_end_limited` copies only if the remaining input reaches EOF within a
  limit;
- `content_eq` and `compare_content` compare readable stream contents.

```rust
use std::io::Cursor;

use qubit_io::Streams;

let mut left = Cursor::new(b"abc".to_vec());
let mut right = Cursor::new(Vec::new());

Streams::copy(&mut left, &mut right)?;
assert_eq!(b"abc", right.into_inner().as_slice());

# Ok::<(), std::io::Error>(())
```

## Extension Traits

`ReadExt`, `BufReadExt`, `SeekExt`, `ReadSeekExt`, and `WriteSeekExt` add common
low-level operations that remain generic over standard-library I/O traits.

```rust
use std::io::Cursor;

use qubit_io::{ReadExt, WriteSeekExt};

let mut input = Cursor::new(b"hello".to_vec());
let bytes = input.read_to_end_limited(16)?;
assert_eq!(b"hello", bytes.as_slice());

let mut output = Cursor::new(vec![0; 8]);
output.write_all_at_preserving_position(2, b"rs")?;

# Ok::<(), std::io::Error>(())
```

## Codec Helpers

Binary, LEB128, ZigZag, and length-prefixed string helpers are available as both
extension traits and reader/writer wrapper types.

```rust
use std::io::Cursor;

use qubit_io::{BinaryReadExt, BinaryWriteExt};

let mut buffer = Vec::new();
buffer.write_u32_be(0x0102_0304)?;

let mut cursor = Cursor::new(buffer);
assert_eq!(0x0102_0304, cursor.read_u32_be()?);

# Ok::<(), std::io::Error>(())
```

## Wrappers

Use wrappers when stream behavior should be part of the type instead of a single
function call:

- `CountingReader` and `CountingWriter` count successful bytes;
- `LimitReader` and `LimitWriter` enforce byte budgets;
- `TeeReader` and `TeeWriter` duplicate traffic to a branch writer;
- `ChecksumReader` and `ChecksumWriter` update caller-provided checksum state;
- `PositionGuard` restores seek position on drop unless dismissed.

## Crate Boundary

`qubit-io` deliberately does not contain local filesystem utilities. Use
`qubit-local-fs` for `Files`, `Filenames`, `TempFile`, `TempDir`, recursive
directory copy, cleanup helpers, and atomic file writes.
