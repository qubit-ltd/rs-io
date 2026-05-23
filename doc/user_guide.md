# Qubit IO User Guide

Qubit IO is the stream and byte-I/O crate in the Qubit Rust family. It focuses
on `std::io` traits, extension methods, stream wrappers, and codec helpers. It
is intentionally not a local filesystem utility crate.

For local filesystem capabilities, see
[qubit-local-files](https://github.com/qubit-ltd/rs-local-files).

## When to Use This Crate

Use `qubit-io` when your code works with byte streams rather than filesystem
paths. Typical examples include parsers, binary codecs, protocol adapters,
archive readers, in-memory buffers, network streams, and APIs that need to
accept flexible `Read` / `Write` / `Seek` implementations.

Good fits:

- Reading exactly up to EOF without losing the number of bytes read.
- Reading a bounded amount of bytes or text from untrusted input.
- Copying a stream only if EOF appears within a size limit.
- Comparing stream contents without first loading everything into memory.
- Writing or reading binary scalar values with explicit byte order.
- Encoding compact integers with LEB128 or ZigZag.
- Exposing trait-object-friendly I/O capabilities such as `dyn ReadSeek`.
- Wrapping streams to count, limit, tee, checksum, or restore positions.

Not a fit:

- Creating temporary files or directories.
- Recursively copying or cleaning local directories.
- Validating local file names or generating local temporary names.
- Atomic replacement writes to local filesystem paths.
- Abstracting local, FTP, object storage, or remote filesystems.

For those local filesystem concerns, use
[qubit-local-files](https://github.com/qubit-ltd/rs-local-files).

## Buffer Codecs

Use the root-level buffer codec types when data already lives in caller-managed
slices and no `std::io::Read` or `std::io::Write` adapter is needed.

| Type | Use when |
| --- | --- |
| `Coder` | implementing a progress-oriented conversion over input and output buffers |
| `CoderProgress`, `CoderStatus` | reporting how far conversion advanced and why it stopped |
| `BinaryCodec` | reading or writing fixed-width scalars at explicit byte indexes |
| `Leb128Codec` | encoding unsigned and signed LEB128 values in byte slices |
| `ZigZagCodec` | encoding signed integers through ZigZag plus unsigned LEB128 |

These codecs are low-level static namespaces. They intentionally expose only
`unsafe` unchecked buffer operations, so the caller must validate the accessible
range before calling them. Use `REQUIRED_MIN_BUFFER_LEN` from the concrete
specialization to size a temporary buffer that can hold one value.

```rust
use qubit_io::{BinaryCodec, BigEndian, Leb128Codec, NonStrict};

let mut fixed = [0_u8; BinaryCodec::<u32, BigEndian>::REQUIRED_MIN_BUFFER_LEN];
unsafe {
    BinaryCodec::<u32, BigEndian>::write_unchecked(&mut fixed, 0, 0x0102_0304);
}

let mut compact = [0_u8; Leb128Codec::<u64, NonStrict>::REQUIRED_MIN_BUFFER_LEN];
let written = unsafe { Leb128Codec::<u64, NonStrict>::write_unchecked(&mut compact, 0, 300) };
assert_eq!(&[0xac, 0x02], &compact[..written]);
```

Stream-oriented reader/writer wrappers live under the separate stream wrapper
surface: `BinaryReader`, `BinaryWriter`, `Leb128Reader`, `Leb128Writer`,
`ZigZagReader`, and `ZigZagWriter`. Reader wrappers also implement `Read`,
writer wrappers implement `Write`, and both sides pass through `Seek` when the
wrapped stream supports it.

## Installation

```toml
[dependencies]
qubit-io = "0.2"
```

## Import Patterns

Import concrete wrappers and namespaces from the crate root:

```rust
use qubit_io::{
    CountingReader,
    LimitWriter,
    Streams,
};
```

Import extension traits explicitly when you want methods to appear on standard
I/O values:

```rust
use qubit_io::{
    ReadExt,
    SeekExt,
    WriteSeekExt,
};
```

Use the prelude when a module mostly needs extension traits, composition traits,
or buffer codec types:

```rust
use qubit_io::prelude::*;
```

The prelude intentionally does not import stream wrapper types. This keeps
concrete runtime behavior explicit at call sites.

## Object-Safe Composition Traits

Rust does not allow all trait combinations to be written directly as trait
objects in every ergonomic form. `qubit-io` provides named traits for common
`std::io` capability sets:

| Trait | Supertraits | Use when |
| --- | --- | --- |
| `ReadSeek` | `Read + Seek` | a consumer needs readable random access |
| `BufReadSeek` | `BufRead + Seek` | a consumer needs buffered readable random access |
| `ReadWrite` | `Read + Write` | a stream or buffer is both readable and writable |
| `WriteSeek` | `Write + Seek` | output can be patched by absolute position |
| `ReadWriteSeek` | `Read + Write + Seek` | input and output share one random-access object |

Example:

```rust
use std::io::{Read, SeekFrom};

use qubit_io::{ReadSeek, SeekExt};

fn read_header(input: &mut dyn ReadSeek) -> std::io::Result<Vec<u8>> {
    let size = input.stream_size()?;
    let mut header = vec![0; size.min(8) as usize];
    input.seek(SeekFrom::Start(0))?;
    input.read_exact(&mut header)?;
    Ok(header)
}
```

## Streams Namespace

`Streams` provides associated functions for generic stream operations.

### Copy Everything

Use `Streams::copy` when you want the standard `std::io::copy` behavior through
the Qubit IO namespace:

```rust
use std::io::Cursor;

use qubit_io::Streams;

let mut input = Cursor::new(b"payload".to_vec());
let mut output = Vec::new();

let copied = Streams::copy(&mut input, &mut output)?;

assert_eq!(7, copied);
assert_eq!(b"payload", output.as_slice());
# Ok::<(), std::io::Error>(())
```

### Copy at Most N Bytes

Use `copy_at_most` when the caller decides how much data may be consumed:

```rust
use std::io::Cursor;

use qubit_io::Streams;

let mut input = Cursor::new(b"abcdef".to_vec());
let mut output = Vec::new();

let copied = Streams::copy_at_most(&mut input, &mut output, 3)?;

assert_eq!(3, copied);
assert_eq!(b"abc", output.as_slice());
# Ok::<(), std::io::Error>(())
```

### Copy Only If EOF Is Within a Limit

Use `copy_to_end_limited` for defensive reads from untrusted streams. It fails if
more data remains after the allowed size:

```rust
use std::io::Cursor;

use qubit_io::Streams;

let mut input = Cursor::new(b"small".to_vec());
let mut output = Vec::new();

let copied = Streams::copy_to_end_limited(&mut input, &mut output, 16)?;

assert_eq!(5, copied);
assert_eq!(b"small", output.as_slice());
# Ok::<(), std::io::Error>(())
```

### Compare Streams

Use `content_eq` for equality and `compare_content` for lexicographic ordering.
Both work incrementally and do not require loading full streams into memory.

```rust
use std::io::Cursor;

use qubit_io::Streams;

let mut left = Cursor::new(b"abc".to_vec());
let mut right = Cursor::new(b"abc".to_vec());

assert!(Streams::content_eq(&mut left, &mut right)?);
# Ok::<(), std::io::Error>(())
```

## Read Extension Methods

`ReadExt` contains exact, bounded, and copy-oriented helpers.

### Exact Read or EOF

`read_exact_or_eof` fills the destination buffer unless EOF appears first. It
returns the number of bytes actually read instead of turning early EOF into an
error.

```rust
use std::io::Cursor;

use qubit_io::ReadExt;

let mut input = Cursor::new(b"abc".to_vec());
let mut buffer = [0_u8; 8];

let count = input.read_exact_or_eof(&mut buffer)?;

assert_eq!(3, count);
assert_eq!(b"abc", &buffer[..count]);
# Ok::<(), std::io::Error>(())
```

### Bounded Read to Memory

`read_to_end_limited` and `read_to_string_limited` are useful when input size is
not fully trusted:

```rust
use std::io::Cursor;

use qubit_io::ReadExt;

let mut input = Cursor::new(b"hello".to_vec());
let bytes = input.read_to_end_limited(16)?;

assert_eq!(b"hello", bytes.as_slice());
# Ok::<(), std::io::Error>(())
```

The `_into` variants append into a caller-provided buffer and roll back appended
bytes on errors where the method promises rollback behavior.

### Method-Style Copy

`copy_to`, `copy_to_at_most`, and `copy_to_end_limited` mirror the `Streams`
namespace as methods on a reader.

```rust
use std::io::Cursor;

use qubit_io::ReadExt;

let mut input = Cursor::new(b"abcdef".to_vec());
let mut output = Vec::new();

let copied = input.copy_to_at_most(&mut output, 4)?;

assert_eq!(4, copied);
assert_eq!(b"abcd", output.as_slice());
# Ok::<(), std::io::Error>(())
```

## Buffered Read Extension Methods

`BufReadExt` adds bounded delimiter-oriented operations. These are useful when
processing line-based or delimiter-based protocols where unbounded line growth
must be rejected.

```rust
use std::io::Cursor;

use qubit_io::BufReadExt;

let mut input = Cursor::new(b"name=value\nrest".to_vec());
let line = input.read_line_limited(32)?;

assert_eq!("name=value\n", line);
# Ok::<(), std::io::Error>(())
```

Use `discard_until_limited` when you want to skip a bounded field without
allocating a buffer for it.

## Seek Extension Methods

`SeekExt::stream_size` measures the stream length and restores the original
position.

```rust
use std::io::{Cursor, Seek, SeekFrom};

use qubit_io::SeekExt;

let mut cursor = Cursor::new(b"abcdef".to_vec());
cursor.seek(SeekFrom::Start(2))?;

let size = cursor.stream_size()?;

assert_eq!(6, size);
assert_eq!(2, cursor.stream_position()?);
# Ok::<(), std::io::Error>(())
```

## Read + Seek Extension Methods

`ReadSeekExt` provides non-consuming reads and absolute-offset reads that restore
the original position.

```rust
use std::io::{Cursor, Seek};

use qubit_io::ReadSeekExt;

let mut cursor = Cursor::new(b"abcdef".to_vec());
let mut buffer = [0_u8; 3];

let count = cursor.peek_exact_or_eof(&mut buffer)?;

assert_eq!(3, count);
assert_eq!(b"abc", &buffer);
assert_eq!(0, cursor.stream_position()?);
# Ok::<(), std::io::Error>(())
```

Use `read_exact_or_eof_at` when you need to inspect a fixed offset but leave the
caller-visible cursor position untouched.

## Write + Seek Extension Methods

`WriteSeekExt::write_all_at_preserving_position` writes at an absolute offset
and restores the original position.

```rust
use std::io::{Cursor, Seek, SeekFrom};

use qubit_io::WriteSeekExt;

let mut cursor = Cursor::new(vec![0; 8]);
cursor.seek(SeekFrom::Start(7))?;

cursor.write_all_at_preserving_position(2, b"rs")?;

assert_eq!(7, cursor.stream_position()?);
assert_eq!(&[0, 0, b'r', b's', 0, 0, 0, 0], cursor.get_ref().as_slice());
# Ok::<(), std::io::Error>(())
```

## Binary Scalar Encoding

`BinaryReadExt` and `BinaryWriteExt` read and write fixed-width numeric scalars
with explicit byte order.

```rust
use std::io::Cursor;

use qubit_io::{BinaryReadExt, BinaryWriteExt};

let mut buffer = Vec::new();
buffer.write_u32_be(0x0102_0304)?;
buffer.write_i16_le(-2)?;

let mut input = Cursor::new(buffer);
assert_eq!(0x0102_0304, input.read_u32_be()?);
assert_eq!(-2, input.read_i16_le()?);
# Ok::<(), std::io::Error>(())
```

Use the runtime `ByteOrder` APIs when byte order is selected by format metadata
rather than by code structure.

## LEB128 and ZigZag Encoding

`Leb128ReadExt` and `Leb128WriteExt` support unsigned and signed LEB128 integer
encoding through 128-bit values. Strict read variants reject non-canonical
encodings.

`ZigZagReadExt` and `ZigZagWriteExt` encode signed values through an unsigned
LEB128 payload. Use ZigZag when small negative values should remain compact.

```rust
use std::io::Cursor;

use qubit_io::{Leb128ReadExt, Leb128WriteExt, ZigZagReadExt, ZigZagWriteExt};

let mut buffer = Vec::new();
buffer.write_uleb_u64(300)?;
buffer.write_zig_zag_i64(-42)?;

let mut input = Cursor::new(buffer);
assert_eq!(300, input.read_uleb_u64()?);
assert_eq!(-42, input.read_zig_zag_i64()?);
# Ok::<(), std::io::Error>(())
```

## Length-Prefixed UTF-8 Strings

`StringReadExt` and `StringWriteExt` read and write UTF-8 strings with ULEB128,
`u16`, or `u32` byte-length prefixes. Reads are bounded by caller-provided size
limits.

```rust
use std::io::Cursor;

use qubit_io::{StringReadExt, StringWriteExt};

let mut buffer = Vec::new();
buffer.write_utf8_string_uleb("hello")?;

let mut input = Cursor::new(buffer);
let value = input.read_utf8_string_uleb(32)?;

assert_eq!("hello", value);
# Ok::<(), std::io::Error>(())
```

## Wrapper Types

### CountingReader and CountingWriter

Use counting wrappers when metrics or validation need to know how many bytes
were successfully read or written.

```rust
use std::io::{Cursor, Read};

use qubit_io::CountingReader;

let inner = Cursor::new(b"abc".to_vec());
let mut reader = CountingReader::new(inner);
let mut output = Vec::new();

reader.read_to_end(&mut output)?;

assert_eq!(3, reader.count());
# Ok::<(), std::io::Error>(())
```

### LimitReader and LimitWriter

Use limit wrappers when a downstream API should see EOF or write failure after a
fixed byte budget.

```rust
use std::io::Read;

use qubit_io::LimitReader;

let inner = std::io::Cursor::new(b"abcdef".to_vec());
let mut reader = LimitReader::new(inner, 3);
let mut output = Vec::new();

reader.read_to_end(&mut output)?;

assert_eq!(b"abc", output.as_slice());
# Ok::<(), std::io::Error>(())
```

### TeeReader and TeeWriter

Use tee wrappers to duplicate traffic to a branch writer while preserving the
normal read or write flow.

### ChecksumReader and ChecksumWriter

Checksum wrappers update a caller-owned checksum state for bytes that are
successfully read or written. They do not prescribe a checksum algorithm.

### PositionGuard

`PositionGuard` records the current stream position and restores it on drop
unless dismissed. It is useful when a function needs to inspect a header, peek at
format metadata, probe a magic number, or run a speculative parser without
changing the caller-visible cursor position.

Use `dismiss` when the inspected operation intentionally becomes the new visible
position. Otherwise, letting the guard drop restores the original position.

```rust
use std::io::{Cursor, Read};

use qubit_io::PositionGuard;

fn looks_like_qubit(input: &mut Cursor<Vec<u8>>) -> std::io::Result<bool> {
    let _guard = PositionGuard::new(input)?;
    let mut magic = [0_u8; 4];
    input.read_exact(&mut magic)?;
    Ok(&magic == b"QBIT")
}

let mut input = Cursor::new(b"QBITpayload".to_vec());

assert!(looks_like_qubit(&mut input)?);
assert_eq!(0, input.position());

# Ok::<(), std::io::Error>(())
```

## Codec Reader and Writer Objects

If you prefer object-style codec APIs over extension traits, use reader and
writer wrappers. These wrappers own the underlying stream and delegate to the
same encoding logic as the extension traits. They are convenient when codec
configuration should live in the reader or writer object. They remain normal
stream wrappers: readers implement `Read`, writers implement `Write`, and both
pass through `Seek` when the wrapped stream supports seeking.

| Wrapper | Purpose |
| --- | --- |
| `BinaryReader`, `BinaryWriter` | fixed-width scalar values |
| `Leb128Reader`, `Leb128Writer` | LEB128 integers |
| `ZigZagReader`, `ZigZagWriter` | ZigZag signed integers |

### BinaryReader and BinaryWriter

Use binary wrappers when parsing or writing fixed-width scalar formats and you
want byte-order selection to live in the wrapper type.

```rust
use std::io::Cursor;

use qubit_io::{BigEndian, BinaryReader, BinaryWriter};

let mut writer = BinaryWriter::<_, BigEndian>::new(Vec::new());
writer.write_u16(0x0102)?;
writer.write_i32(-7)?;
let bytes = writer.into_inner();

let mut reader = BinaryReader::<_, BigEndian>::new(Cursor::new(bytes));
assert_eq!(0x0102, reader.read_u16()?);
assert_eq!(-7, reader.read_i32()?);

# Ok::<(), std::io::Error>(())
```

### Leb128Reader and Leb128Writer

Use LEB128 wrappers for compact integer fields. The reader can be configured for
strict canonical decoding, which is useful when non-canonical encodings should
be rejected at format boundaries.

```rust
use std::io::Cursor;

use qubit_io::{Leb128Reader, Leb128Writer, Strict};

let mut writer = Leb128Writer::new(Vec::new());
writer.write_u64(300)?;
writer.write_i64(-42)?;
let bytes = writer.into_inner();

let mut reader = Leb128Reader::<_, Strict>::new(Cursor::new(bytes));
assert_eq!(300, reader.read_u64()?);
assert_eq!(-42, reader.read_i64()?);

# Ok::<(), std::io::Error>(())
```

### ZigZagReader and ZigZagWriter

Use ZigZag wrappers when signed integers are expected to be small around zero,
including negative values. ZigZag maps signed values to unsigned LEB128 payloads
so `-1`, `0`, and `1` remain compact.

```rust
use std::io::Cursor;

use qubit_io::{Strict, ZigZagReader, ZigZagWriter};

let mut writer = ZigZagWriter::new(Vec::new());
writer.write_i64(-1)?;
writer.write_i64(42)?;
let bytes = writer.into_inner();

let mut reader = ZigZagReader::<_, Strict>::new(Cursor::new(bytes));
assert_eq!(-1, reader.read_i64()?);
assert_eq!(42, reader.read_i64()?);

# Ok::<(), std::io::Error>(())
```

## Error Model

Most APIs return `std::io::Result`. The crate preserves standard I/O behavior
where possible and uses `std::io::ErrorKind` for validation failures such as
size limits or non-canonical encodings. Methods that restore positions document
which error is returned when both the original operation and the restore attempt
can fail.

## Crate Boundary

`qubit-io` deliberately does not contain local filesystem utilities. Use
[qubit-local-files](https://github.com/qubit-ltd/rs-local-files) when you need local
path utilities, temporary files and directories, recursive directory operations,
directory cleanup, or atomic file writes.
