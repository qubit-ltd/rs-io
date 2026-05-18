# Qubit IO API Matrix

This matrix summarizes the public API re-exported by `qubit_io`.

## Composition Traits

| Trait | Standard-library bounds | Purpose |
|-------|-------------------------|---------|
| `ReadSeek` | `Read + Seek` | Readable random-access inputs. |
| `BufReadSeek` | `BufRead + Seek` | Buffered readable random-access inputs. |
| `ReadWrite` | `Read + Write` | Duplex streams and mutable buffers. |
| `WriteSeek` | `Write + Seek` | Writable random-access outputs. |
| `ReadWriteSeek` | `Read + Write + Seek` | Fully mutable random-access I/O objects. |

## Extension Traits

| Trait | Methods | Notes |
|-------|---------|-------|
| `ReadExt` | `read_exact_or_eof`, `discard_exact_or_eof`, `copy_to`, `copy_to_limited`, `read_to_end_limited`, `read_to_string_limited` | Short-read-safe reads, bounded copies, bounded byte reads, and bounded UTF-8 reads. |
| `BufReadExt` | `read_until_limited`, `read_line_limited`, `discard_until_limited` | Bounded delimiter and line operations for buffered readers. |
| `SeekExt` | `stream_size` | Measures stream size while restoring the original position. May be deprecated if the standard library adds `Seek::stream_size`. |
| `ReadSeekExt` | `peek_exact_or_eof`, `read_exact_or_eof_at` | Position-preserving peek and random-offset reads. |
| `WriteSeekExt` | `write_all_at_preserving_position` | Position-preserving random-offset writes. |

## Binary Scalars

| Trait | Supported values | Byte order support |
|-------|------------------|--------------------|
| `BinaryReadExt` | `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `u64`, `i64`, `u128`, `i128`, `f32`, `f64` | `_be`, `_le`, and runtime `ByteOrder` methods for multi-byte values. |
| `BinaryWriteExt` | `u8`, `i8`, `u16`, `i16`, `u32`, `i32`, `u64`, `i64`, `u128`, `i128`, `f32`, `f64` | `_be`, `_le`, and runtime `ByteOrder` methods for multi-byte values. |

## Integer Encodings

| Trait | Supported values | Strict decoding |
|-------|------------------|-----------------|
| `Leb128IntReadExt` | unsigned `u8`, `u16`, `u32`, `u64`, `u128`, `usize`; signed `i8`, `i16`, `i32`, `i64`, `i128`, `isize` | Every read method also has a `_strict` variant that rejects non-canonical LEB128 encodings. |
| `Leb128IntWriteExt` | unsigned `u8`, `u16`, `u32`, `u64`, `u128`, `usize`; signed `i8`, `i16`, `i32`, `i64`, `i128`, `isize` | Write methods always emit canonical LEB128 encodings. |
| `ZigZagIntReadExt` | `i8`, `i16`, `i32`, `i64`, `i128`, `isize` | Every read method also has a `_strict` variant that requires a canonical unsigned LEB128 payload. |
| `ZigZagIntWriteExt` | `i8`, `i16`, `i32`, `i64`, `i128`, `isize` | Write methods emit ZigZag-mapped canonical unsigned LEB128 payloads. |

LEB128 follows the WebAssembly Core binary value encoding:
<https://webassembly.github.io/spec/core/binary/values.html#integers>.

ZigZag follows the Protocol Buffers signed integer mapping:
<https://protobuf.dev/programming-guides/encoding/#signed-integers>.

## Length-Prefixed UTF-8 Strings

| Trait | Methods | Limit behavior |
|-------|---------|----------------|
| `StringReadExt` | `read_utf8_string_uleb`, `read_utf8_string_u16_be`, `read_utf8_string_u16_le`, `read_utf8_string_u32_be`, `read_utf8_string_u32_le` | Every read method requires `max_len` and rejects encoded payload lengths above that limit before allocating the payload buffer. |
| `StringWriteExt` | `write_utf8_string_uleb`, `write_utf8_string_u16_be`, `write_utf8_string_u16_le`, `write_utf8_string_u32_be`, `write_utf8_string_u32_le` | Fixed-width length methods reject strings whose UTF-8 byte length does not fit the prefix type. |

## Utility Functions

| Function | Purpose |
|----------|---------|
| `copy_limited` | Copies at most `max_bytes` bytes from a reader to a writer. |
| `content_eq` | Compares two readers for byte-for-byte equality. |
| `compare_content` | Lexicographically compares two readers. |
| `open_buffered_reader` | Opens a file as `BufReader<File>`. |
| `create_file_with_parent` | Creates missing parent directories, then creates a file. |
| `create_buffered_writer_with_parent` | Creates missing parent directories, then creates `BufWriter<File>`. |
| `atomic_write` | Writes bytes through a same-directory temporary file, fsyncs the temporary file, replaces the destination, and syncs the parent directory when supported. |
| `atomic_write_with` | Same as `atomic_write`, but accepts caller-provided write logic for the temporary file. |

## Wrapper Types

| Type | Implements | Purpose |
|------|------------|---------|
| `CountingReader` | `Read` | Counts successfully read bytes. |
| `CountingWriter` | `Write` | Counts successfully written bytes. |
| `LimitReader` | `Read` | Caps bytes read from an inner reader. |
| `LimitWriter` | `Write` | Caps bytes written to an inner writer. |
| `TeeReader` | `Read` | Copies successfully read bytes to a branch writer. |
| `TeeWriter` | `Write` | Writes to a primary writer and mirrors bytes to a branch writer. |
| `ChecksumReader` | `Read` | Updates a caller-provided checksum state for successfully read bytes. |
| `ChecksumWriter` | `Write` | Updates a caller-provided checksum state for successfully written bytes. |
| `PositionGuard` | `Seek` guard | Restores the original stream position on drop unless dismissed. |
