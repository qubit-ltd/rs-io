# Qubit IO User Guide

Use `qubit-io` when code needs reusable `std::io` helpers without choosing a
binary or text encoding format. The crate stays at the generic I/O layer.

## Capability Map

| Area | API | Use when |
| --- | --- | --- |
| Buffered byte I/O | `Buffer`, `BufferedByteInput`, `BufferedByteOutput` | higher-level adapters need format-agnostic byte windows |
| Composition traits | `ReadSeek`, `ReadWrite`, `ReadWriteSeek`, `BufReadSeek`, `WriteSeek` | an API needs a trait object for combined I/O capabilities |
| Read helpers | `ReadExt` | exact-or-EOF reads, bounded reads, and copy helpers |
| Buffered helpers | `BufReadExt` | bounded delimiter and line reads |
| Seek helpers | `SeekExt`, `ReadSeekExt`, `WriteSeekExt` | stream-size queries and position-preserving reads or writes |
| Stream utilities | `Streams` | namespaced copy and content comparison |
| Wrappers | `CountingReader`, `LimitReader`, `TeeReader`, checksum wrappers, `PositionGuard` | small behavior adapters around existing streams |

## Installation

```toml
[dependencies]
qubit-io = "0.7"
```

## Buffered Byte I/O

`BufferedByteInput` and `BufferedByteOutput` are byte-oriented buffering
primitives. They do not decode binary values, decode text, or parse records.
Those layers should be built in sibling crates on top of the byte windows.

```rust
use std::io::{
    BufRead,
    Cursor,
};

use qubit_io::BufferedByteInput;

let mut input = BufferedByteInput::with_capacity(
    Cursor::new(b"abcdef".to_vec()),
    4,
);

assert_eq!(b"abcd", input.fill_buf()?);
input.consume(2);

let (inner, unread) = input.into_parts();
assert_eq!(4, inner.position());
assert_eq!(b"cd", unread.as_slice());
# Ok::<(), std::io::Error>(())
```

`BufferedByteOutput::into_parts` performs no I/O and returns the wrapped writer
plus any pending bytes. To finish successfully, call `flush` first and then
verify that `into_parts` returns an empty pending byte vector. If flushing
fails, the caller still owns the buffered output and can retry or dismantle it.

```rust
use std::io::{
    Cursor,
    Write,
};

use qubit_io::BufferedByteOutput;

let mut output =
    BufferedByteOutput::with_capacity(Cursor::new(Vec::<u8>::new()), 4);
output.ensure_spare_capacity(3)?;
output.spare_buffer_mut()[0..3].copy_from_slice(b"xyz");
unsafe {
    output.advance_unchecked(3);
}

output.flush()?;
let (cursor, pending) = output.into_parts();
assert!(pending.is_empty());
assert_eq!(b"xyz", cursor.into_inner().as_slice());
# Ok::<(), std::io::Error>(())
```

Hot-path adapters can use `unread_raw_parts` and `spare_raw_parts_mut` to pass
the full backing buffer plus an index to unchecked codec APIs. General-purpose
callers should prefer `unread_slice`, `spare_buffer_mut`, `consume`, and
`advance`.

## Extension Traits

Import the trait whose methods you want to call.

```rust
use std::io::Cursor;

use qubit_io::ReadExt;

let mut input = Cursor::new(b"abc".to_vec());
let mut bytes = [0_u8; 8];

let read = input.read_exact_or_eof(&mut bytes)?;

assert_eq!(3, read);
assert_eq!(b"abc", &bytes[..read]);
# Ok::<(), std::io::Error>(())
```

`ReadExt` includes bounded helpers such as `read_to_end_limited` and
`read_exact_vec_limited`. These are useful at protocol and file-format
boundaries where unbounded allocation would be a bug.

`BufReadExt` provides bounded delimiter operations:

```rust
use std::io::Cursor;

use qubit_io::BufReadExt;

let mut input = Cursor::new(b"first\nsecond".to_vec());
let line = input.read_line_limited(16)?;

assert_eq!("first\n", line);
# Ok::<(), std::io::Error>(())
```

## Position-Preserving I/O

`ReadSeekExt` and `WriteSeekExt` are for APIs that need temporary random access
without changing the caller-visible position.

```rust
use std::io::Cursor;

use qubit_io::ReadSeekExt;

let mut input = Cursor::new(b"abcdef".to_vec());
let mut header = [0_u8; 2];

input.read_exact_or_eof_at(2, &mut header)?;

assert_eq!(b"cd", &header);
assert_eq!(0, input.position());
# Ok::<(), std::io::Error>(())
```

## Streams

`Streams` is an uninstantiable namespace for operations involving one or more
streams.

```rust
use std::io::Cursor;

use qubit_io::Streams;

let mut input = Cursor::new(b"abcdef".to_vec());
let mut output = Vec::new();

let copied = Streams::copy_to_end_limited(&mut input, &mut output, 8)?;

assert_eq!(6, copied);
assert_eq!(b"abcdef", output.as_slice());
# Ok::<(), std::io::Error>(())
```

Use `Streams::content_eq` or `Streams::compare_content` when comparing remaining
stream contents from their current positions.

## Wrappers

Wrappers compose small stream behaviors without owning the underlying resource
type.

```rust
use std::io::Read;

use qubit_io::CountingReader;

let inner = std::io::Cursor::new(b"abc".to_vec());
let mut reader = CountingReader::new(inner);
let mut bytes = [0_u8; 2];

reader.read_exact(&mut bytes)?;

assert_eq!(2, reader.count());
# Ok::<(), std::io::Error>(())
```

Common wrappers include:

| Wrapper | Purpose |
| --- | --- |
| `CountingReader`, `CountingWriter` | count successfully read or written bytes |
| `LimitReader`, `LimitWriter` | cap how many bytes can pass through |
| `TeeReader`, `TeeWriter` | copy successful reads or writes to a branch sink |
| `ChecksumReader`, `ChecksumWriter` | hash successful bytes through a caller-supplied hasher |
| `PositionGuard` | restore a seek position unless dismissed |

## What This Crate Does Not Contain

`qubit-io` deliberately does not contain binary scalar codecs, LEB128, ZigZag,
or text charset adapters. Use the sibling crates for those layers:

- `qubit-codec-binary` for buffer-level binary codecs;
- `qubit-io-binary` for binary stream extension traits and wrappers;
- `qubit-codec-text` for text codecs;
- `qubit-io-text` for text stream adapters.
