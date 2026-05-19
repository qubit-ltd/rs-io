# Qubit IO User Guide

Qubit IO is a small standard-library-first I/O helper crate. It does not try to
replace `std::io`; instead, it gives names to common capability combinations,
adds conservative extension methods, and provides a few wrappers for stream
instrumentation and bounded I/O.

## What This Crate Provides

- Object-safe composition traits such as `ReadSeek` and `ReadWriteSeek`.
- Extension traits for exact reads, bounded reads, bounded delimiter reads,
  binary scalars, LEB128, ZigZag, and length-prefixed UTF-8 strings.
- A `Files` namespace for parent directory creation, random temporary entries
  with safe file-name fragment validation, buffered file helpers, and durable
  same-directory atomic writes.
- `Streams` and `Filenames` namespaces for stream copy/compare operations and
  lexical file-name helpers.
- Wrapper types for counting, limiting, teeing, checksumming, and restoring a
  seekable stream position.
- Codec wrapper types for users who prefer reader/writer objects over
  extension-method calls.

## Import Patterns

Use individual imports when a module only needs a few APIs:

```rust
use qubit_io::{
    Filenames,
    Files,
    ReadExt,
    Streams,
    WriteSeekExt,
};
```

Use the prelude for method-heavy call sites that use several extension traits:

```rust
use qubit_io::prelude::*;
```

The prelude intentionally re-exports composition and extension traits, plus
`ByteOrder`. Wrapper types, `Files`, `Streams`, and `Filenames` remain explicit
root-level imports.

## Composition Traits

Rust trait aliases are not stable, and trait objects cannot directly combine
multiple non-auto traits in every shape APIs need. Qubit IO defines named,
object-safe traits and implements them for every matching type:

```rust
use qubit_io::ReadSeek;
use std::io::{
    Read,
    Seek,
};

fn as_read_seek<T>(value: &mut T) -> &mut dyn ReadSeek
where
    T: Read + Seek,
{
    value
}
```

Use these traits when an API stores or passes heterogeneous I/O values behind a
trait object. Prefer ordinary generic bounds when the concrete type can remain
generic.

## Exact and Bounded Reads

`ReadExt` covers short-read-safe helpers and allocation guards:

- `read_exact_or_eof` fills a caller-provided buffer or returns a successful
  partial byte count at EOF.
- `read_exact_array::<N>` reads exactly `N` bytes into a stack array.
- `read_exact_vec_limited` and `read_exact_vec_limited_into` reject oversized
  exact reads before allocating.
- `read_to_end_limited` and string variants detect oversized inputs by reading
  at most one excess byte.

```rust
use qubit_io::ReadExt;
use std::io::Cursor;

let mut input = Cursor::new(b"abcdef".to_vec());
let header = input.read_exact_array::<2>()?;
let payload = input.read_exact_vec_limited(4, 16)?;

assert_eq!(*b"ab", header);
assert_eq!(b"cdef", payload.as_slice());
# Ok::<(), std::io::Error>(())
```

## Binary Scalars, LEB128, ZigZag, and Strings

`BinaryReadExt` and `BinaryWriteExt` read and write primitive scalar values
with `_be`, `_le`, or runtime `ByteOrder` methods.

`Leb128ReadExt` and `Leb128WriteExt` encode unsigned and signed LEB128 values
through integer-specific methods such as `read_uleb_u32` and
`write_sleb_i64`. Read methods with the `_strict` suffix reject non-canonical
encodings.

`ZigZagReadExt` and `ZigZagWriteExt` read and write ZigZag-mapped signed
integers using unsigned LEB128 payloads. Their strict read methods require the
underlying unsigned LEB128 payload to be canonical.

`StringReadExt` and `StringWriteExt` read and write UTF-8 strings with ULEB128,
`u16`, or `u32` byte-length prefixes. ULEB string reads include
`read_utf8_string_uleb_strict`, which rejects non-canonical ULEB length
prefixes before reading the payload. Every string read requires `max_len` and
rejects oversized payload lengths before allocation.

## File Utilities

Use `Files` associated methods instead of free functions:

- `ensure_dir` and `ensure_parent` create missing directories.
- `open_buffered_reader`, `create_file_with_parent`, and
  `create_buffered_writer_with_parent` handle common file-open patterns.
- `random_file_name`, `try_random_file_name`, `temp_dir`, and `temp_path`
  construct random temporary names and paths. `try_random_file_name` reports
  invalid path-like prefix or suffix fragments through `Result`.
- `create_temp_file`, `create_temp_file_with`, `create_temp_file_in`,
  `create_temp_dir_with`, and `create_temp_dir_in` create collision-resistant
  random temporary entries with `getrandom`-backed OS randomness and reject
  path-like name fragments before joining with the target directory.
- `atomic_write` and `atomic_write_with` write through a random same-directory
  temporary file, preserve existing regular-file permissions, flush and sync it,
  replace the destination, and sync the parent directory when the platform
  supports directory syncing.

```rust
use qubit_io::Files;

let dir = Files::create_temp_dir_with(Some("qubit-io-guide-"), 16)?;
let path = dir.join("nested").join("data.bin");

Files::atomic_write(&path, b"payload")?;
assert_eq!(b"payload", std::fs::read(&path)?.as_slice());

std::fs::remove_dir_all(dir)?;
# Ok::<(), std::io::Error>(())
```

## Stream Utilities

Use `Streams` when an operation involves more than one stream or is clearer as
a namespace-level helper:

- `copy` delegates to `std::io::copy`, preserving standard-library optimized
  copy paths.
- `copy_at_most` copies no more than the requested byte count.
- `copy_to_end_limited` requires the remaining input to reach EOF within the
  limit.
- `content_eq` and `compare_content` consume two readers while comparing their
  remaining bytes.

```rust
use qubit_io::Streams;
use std::io::Cursor;

let mut input = Cursor::new(b"abcdef".to_vec());
let mut output = Vec::new();

let copied = Streams::copy_at_most(&mut input, &mut output, 4)?;

assert_eq!(4, copied);
assert_eq!(b"abcd", output.as_slice());
# Ok::<(), std::io::Error>(())
```

## Filename Utilities

Use `Filenames` for lexical file-name operations that do not touch the
filesystem. Public methods that return filename data return UTF-8 string values
(`&str` or `String`), not `OsStr`; invalid UTF-8 path components return `None`:

- `file_name`, `file_stem`, `file_prefix`, and `extension` expose common
  `Path` components as `&str`.
- `dot_extension`, `has_extension`, and `has_extension_ignore_ascii_case`
  cover frequent extension checks.
- `file_name_from_path` extracts the final segment from a string containing
  `/` or `\` separators.
- `file_name_from_url` removes query/fragment suffixes and decodes
  percent-encoded UTF-8 in the final URL path segment.

Path-based helpers follow `std::path::Path` semantics. In particular, dotfiles
such as `.env` do not have an extension unless they contain another dot.

```rust
use qubit_io::Filenames;
use std::path::Path;

let path = Path::new("/tmp/archive.tar.gz");

assert_eq!(Some("archive.tar"), Filenames::file_stem(path));
assert_eq!(Some("gz"), Filenames::extension(path));
assert!(Filenames::has_extension(path, ".gz"));
assert_eq!(
    "my file.txt",
    Filenames::file_name_from_url("https://example.com/my%20file.txt")
);
```

## Stream Wrappers

Wrappers are transparent around the wrapped reader or writer and implement the
corresponding standard-library I/O trait:

- `CountingReader` and `CountingWriter` track successfully transferred bytes.
- `LimitReader` and `LimitWriter` expose or accept at most a fixed number of
  bytes.
- `TeeReader` and `TeeWriter` mirror accepted bytes into a branch writer.
- `ChecksumReader` and `ChecksumWriter` update a caller-provided `Hasher`.
- `PositionGuard` restores a seekable stream to its captured position on drop
  unless restored or dismissed explicitly.

## Codec Wrappers

Use the root-level wrapper types when explicit reader/writer objects are clearer
than importing extension traits at the call site:

- `BinaryReader` and `BinaryWriter`
- `Leb128Reader` and `Leb128Writer`
- `ZigZagReader` and `ZigZagWriter`

These wrappers own the underlying stream, expose `get_ref`, `get_mut`, and
`into_inner`, and delegate to the same encoding implementations as the extension
traits. `BinaryReader` and `BinaryWriter` also store a runtime `ByteOrder`.
`Leb128Reader` and `ZigZagReader` store a runtime strictness flag, so their
object-style APIs use concise names such as `read_u16` and `read_i32`; create
them with `with_strict` or change the flag with `set_strict` when canonical
LEB128 validation is required.

## API Matrix

This matrix summarizes the root-level public API.

### Prelude

| Module | Re-exports |
|--------|------------|
| `qubit_io::prelude` | `BinaryReadExt`, `BinaryWriteExt`, `BufReadExt`, `BufReadSeek`, `ByteOrder`, `Leb128ReadExt`, `Leb128WriteExt`, `ReadExt`, `ReadSeek`, `ReadSeekExt`, `ReadWrite`, `ReadWriteSeek`, `SeekExt`, `StringReadExt`, `StringWriteExt`, `WriteSeek`, `WriteSeekExt`, `ZigZagReadExt`, `ZigZagWriteExt` |

### Composition Traits

| Trait | Standard-library bounds | Purpose |
|-------|-------------------------|---------|
| `ReadSeek` | `Read + Seek` | Readable random-access inputs. |
| `BufReadSeek` | `BufRead + Seek` | Buffered readable random-access inputs. |
| `ReadWrite` | `Read + Write` | Duplex streams and mutable buffers. |
| `WriteSeek` | `Write + Seek` | Writable random-access outputs. |
| `ReadWriteSeek` | `Read + Write + Seek` | Fully mutable random-access I/O objects. |

### Extension Traits

| Trait | Methods | Notes |
|-------|---------|-------|
| `ReadExt` | `read_exact_or_eof`, `read_exact_array`, `read_exact_vec_limited`, `read_exact_vec_limited_into`, `discard_exact_or_eof`, `copy_to`, `copy_to_at_most`, `copy_to_end_limited`, `read_to_end_limited`, `read_to_end_limited_into`, `read_to_string_limited`, `read_to_string_limited_into` | Short-read-safe reads, exact reads, bounded copies, bounded byte reads, and bounded UTF-8 reads. |
| `BufReadExt` | `read_until_limited`, `read_until_limited_into`, `read_line_limited`, `read_line_limited_into`, `discard_until_limited` | Bounded delimiter and line operations for buffered readers. |
| `SeekExt` | `stream_size` | Measures stream size while restoring the original position. |
| `ReadSeekExt` | `peek_exact_or_eof`, `read_exact_or_eof_at` | Position-preserving peek and random-offset reads. |
| `WriteSeekExt` | `write_all_at_preserving_position` | Position-preserving random-offset writes. |

### Binary Scalars

| Trait | Methods |
|-------|---------|
| `BinaryReadExt` | `read_u8`, `read_i8`; `read_u16`, `read_u16_be`, `read_u16_le`; `read_i16`, `read_i16_be`, `read_i16_le`; `read_u32`, `read_u32_be`, `read_u32_le`; `read_i32`, `read_i32_be`, `read_i32_le`; `read_u64`, `read_u64_be`, `read_u64_le`; `read_i64`, `read_i64_be`, `read_i64_le`; `read_u128`, `read_u128_be`, `read_u128_le`; `read_i128`, `read_i128_be`, `read_i128_le`; `read_f32`, `read_f32_be`, `read_f32_le`; `read_f64`, `read_f64_be`, `read_f64_le` |
| `BinaryWriteExt` | `write_u8`, `write_i8`; `write_u16`, `write_u16_be`, `write_u16_le`; `write_i16`, `write_i16_be`, `write_i16_le`; `write_u32`, `write_u32_be`, `write_u32_le`; `write_i32`, `write_i32_be`, `write_i32_le`; `write_u64`, `write_u64_be`, `write_u64_le`; `write_i64`, `write_i64_be`, `write_i64_le`; `write_u128`, `write_u128_be`, `write_u128_le`; `write_i128`, `write_i128_be`, `write_i128_le`; `write_f32`, `write_f32_be`, `write_f32_le`; `write_f64`, `write_f64_be`, `write_f64_le` |

Multi-byte runtime-order methods use `ByteOrder::{BigEndian, LittleEndian}`.

### Integer Encodings

| Trait | Methods |
|-------|---------|
| `Leb128ReadExt` | `read_uleb_u8`, `read_uleb_u8_strict`; `read_uleb_u16`, `read_uleb_u16_strict`; `read_uleb_u32`, `read_uleb_u32_strict`; `read_uleb_u64`, `read_uleb_u64_strict`; `read_uleb_u128`, `read_uleb_u128_strict`; `read_uleb_usize`, `read_uleb_usize_strict`; `read_sleb_i8`, `read_sleb_i8_strict`; `read_sleb_i16`, `read_sleb_i16_strict`; `read_sleb_i32`, `read_sleb_i32_strict`; `read_sleb_i64`, `read_sleb_i64_strict`; `read_sleb_i128`, `read_sleb_i128_strict`; `read_sleb_isize`, `read_sleb_isize_strict` |
| `Leb128WriteExt` | `write_uleb_u8`, `write_uleb_u16`, `write_uleb_u32`, `write_uleb_u64`, `write_uleb_u128`, `write_uleb_usize`, `write_sleb_i8`, `write_sleb_i16`, `write_sleb_i32`, `write_sleb_i64`, `write_sleb_i128`, `write_sleb_isize` |
| `ZigZagReadExt` | `read_zigzag_i8`, `read_zigzag_i8_strict`; `read_zigzag_i16`, `read_zigzag_i16_strict`; `read_zigzag_i32`, `read_zigzag_i32_strict`; `read_zigzag_i64`, `read_zigzag_i64_strict`; `read_zigzag_i128`, `read_zigzag_i128_strict`; `read_zigzag_isize`, `read_zigzag_isize_strict` |
| `ZigZagWriteExt` | `write_zigzag_i8`, `write_zigzag_i16`, `write_zigzag_i32`, `write_zigzag_i64`, `write_zigzag_i128`, `write_zigzag_isize` |

LEB128 follows the WebAssembly Core binary value encoding:
<https://webassembly.github.io/spec/core/binary/values.html#integers>.

ZigZag follows the Protocol Buffers signed integer mapping:
<https://protobuf.dev/programming-guides/encoding/#signed-integers>.

### Length-Prefixed UTF-8 Strings

| Trait | Methods | Limit behavior |
|-------|---------|----------------|
| `StringReadExt` | `read_utf8_string_uleb`, `read_utf8_string_uleb_strict`, `read_utf8_string_u16_be`, `read_utf8_string_u16_le`, `read_utf8_string_u32_be`, `read_utf8_string_u32_le` | Every read method requires `max_len` and rejects encoded payload lengths above that limit before allocating the payload buffer. The strict ULEB variant also rejects non-canonical length prefixes. |
| `StringWriteExt` | `write_utf8_string_uleb`, `write_utf8_string_u16_be`, `write_utf8_string_u16_le`, `write_utf8_string_u32_be`, `write_utf8_string_u32_le` | Fixed-width length methods reject strings whose UTF-8 byte length does not fit the prefix type. |

### File Utilities

| API | Purpose |
|-----|---------|
| `Files::DEFAULT_TEMP_FILE_PREFIX` | Default prefix for random temporary file names. |
| `Files::DEFAULT_TEMP_FILE_RETRIES` | Default retry count for random temporary entry creation. |
| `Files::open_buffered_reader` | Opens a file as `BufReader<File>`. |
| `Files::ensure_dir` | Creates a directory and missing ancestors. |
| `Files::ensure_parent` | Creates missing parent directories for a file path. |
| `Files::create_file_with_parent` | Creates missing parent directories, then creates a file. |
| `Files::create_buffered_writer_with_parent` | Creates missing parent directories, then creates `BufWriter<File>`. |
| `Files::random_file_name` | Builds a random name from an optional prefix and suffix. |
| `Files::try_random_file_name` | Builds a random name through a `Result`-returning API and rejects path-like fragments. |
| `Files::temp_dir` | Returns the process temporary directory. |
| `Files::temp_path` | Builds a random path under the process temporary directory. |
| `Files::create_temp_file` | Creates a random temporary file under the process temporary directory. |
| `Files::create_temp_file_with` | Creates a random temporary file under the process temporary directory with caller-provided naming and retry options. |
| `Files::create_temp_file_in` | Creates a random temporary file in a caller-provided directory. |
| `Files::create_temp_dir_with` | Creates a random temporary directory under the process temporary directory. |
| `Files::create_temp_dir_in` | Creates a random temporary directory in a caller-provided directory. |
| `Files::atomic_write` | Writes bytes through a same-directory temporary file, preserves existing regular-file permissions, syncs the temporary file, replaces the destination, and syncs the parent directory when supported. |
| `Files::atomic_write_with` | Same as `atomic_write`, but accepts caller-provided write logic for the temporary file. |

### Stream Utilities

| API | Purpose |
|-----|---------|
| `Streams::copy` | Namespace-style wrapper around `std::io::copy`. |
| `Streams::copy_at_most` | Copies at most `max_bytes` bytes from a reader to a writer. |
| `Streams::copy_to_end_limited` | Copies until EOF, returning `InvalidData` if input is longer than `max_bytes`. |
| `Streams::content_eq` | Compares two readers for byte-for-byte equality. |
| `Streams::compare_content` | Lexicographically compares two readers. |

### Filename Utilities

| API | Purpose |
|-----|---------|
| `Filenames::file_name` | Returns the final file-name component as `&str`. |
| `Filenames::file_stem` | Returns the file stem as `&str` using `Path::file_stem` semantics. |
| `Filenames::file_prefix` | Returns the file prefix as `&str` using `Path::file_prefix` semantics. |
| `Filenames::extension` | Returns the final extension as `&str` using `Path::extension` semantics. |
| `Filenames::dot_extension` | Returns the final extension with a leading dot as `String`. |
| `Filenames::has_extension` | Performs a case-sensitive final-extension check. |
| `Filenames::has_extension_ignore_ascii_case` | Performs an ASCII-case-insensitive final-extension check. |
| `Filenames::file_name_from_path` | Extracts the final segment from a string with `/` or `\` separators. |
| `Filenames::file_name_from_url` | Extracts and percent-decodes the final URL path segment. |

### Wrapper Types

| Type | Implements | Public methods |
|------|------------|----------------|
| `CountingReader` | `Read` | `new`, `bytes_read`, `get_ref`, `get_mut`, `into_inner` |
| `CountingWriter` | `Write` | `new`, `bytes_written`, `get_ref`, `get_mut`, `into_inner` |
| `LimitReader` | `Read` | `new`, `remaining`, `get_ref`, `get_mut`, `into_inner` |
| `LimitWriter` | `Write` | `new`, `remaining`, `get_ref`, `get_mut`, `into_inner` |
| `TeeReader` | `Read` | `new`, `reader_ref`, `reader_mut`, `branch_ref`, `branch_mut`, `into_inner` |
| `TeeWriter` | `Write` | `new`, `primary_ref`, `primary_mut`, `branch_ref`, `branch_mut`, `into_inner` |
| `ChecksumReader` | `Read` | `new`, `checksum`, `get_ref`, `get_mut`, `hasher_ref`, `hasher_mut`, `into_inner` |
| `ChecksumWriter` | `Write` | `new`, `checksum`, `get_ref`, `get_mut`, `hasher_ref`, `hasher_mut`, `into_inner` |
| `PositionGuard` | drop guard for `Seek` | `new`, `position`, `get_mut`, `restore`, `dismiss` |

### Codec Wrapper Types

| Type | Purpose |
|------|-----------------|
| `BinaryReader` | Reader object for binary scalar and fixed-width length-prefixed string decoding. |
| `BinaryWriter` | Writer object for binary scalar and fixed-width length-prefixed string encoding. |
| `Leb128Reader` | Reader object for LEB128 integer and ULEB128 length-prefixed string decoding, with configurable strict canonical decoding. |
| `Leb128Writer` | Writer object for LEB128 integer and ULEB128 length-prefixed string encoding. |
| `ZigZagReader` | Reader object for ZigZag signed integer decoding, with configurable strict validation of the underlying ULEB128 integer. |
| `ZigZagWriter` | Writer object for ZigZag signed integer encoding. |

## Dependencies

Qubit IO depends on the Rust standard library and `getrandom` at runtime. The
`getrandom` dependency is used to generate random temporary file and directory
names for the `Files` helpers.
