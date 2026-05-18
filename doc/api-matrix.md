# Qubit IO API Matrix

This matrix summarizes the public API re-exported by `qubit_io`.

## Prelude

| Module | Re-exports |
|--------|------------|
| `qubit_io::prelude` | `BinaryReadExt`, `BinaryWriteExt`, `BufReadExt`, `BufReadSeek`, `Leb128IntReadExt`, `Leb128IntWriteExt`, `ReadExt`, `ReadSeek`, `ReadSeekExt`, `ReadWrite`, `ReadWriteSeek`, `SeekExt`, `StringReadExt`, `StringWriteExt`, `WriteSeek`, `WriteSeekExt`, `ZigZagIntReadExt`, `ZigZagIntWriteExt` |

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
| `ReadExt` | `read_exact_or_eof`, `discard_exact_or_eof`, `copy_to`, `copy_to_at_most`, `copy_to_end_limited`, `read_to_end_limited`, `read_to_end_limited_into`, `read_to_string_limited`, `read_to_string_limited_into` | Short-read-safe reads, bounded copies, bounded byte reads, and bounded UTF-8 reads. |
| `BufReadExt` | `read_until_limited`, `read_until_limited_into`, `read_line_limited`, `read_line_limited_into`, `discard_until_limited` | Bounded delimiter and line operations for buffered readers. |
| `SeekExt` | `stream_size` | Measures stream size while restoring the original position. May be deprecated if the standard library adds `Seek::stream_size`. |
| `ReadSeekExt` | `peek_exact_or_eof`, `read_exact_or_eof_at` | Position-preserving peek and random-offset reads. |
| `WriteSeekExt` | `write_all_at_preserving_position` | Position-preserving random-offset writes. |

## Binary Scalars

| Trait | Methods |
|-------|---------|
| `BinaryReadExt` | `read_u8`, `read_i8`; `read_u16`, `read_u16_be`, `read_u16_le`; `read_i16`, `read_i16_be`, `read_i16_le`; `read_u32`, `read_u32_be`, `read_u32_le`; `read_i32`, `read_i32_be`, `read_i32_le`; `read_u64`, `read_u64_be`, `read_u64_le`; `read_i64`, `read_i64_be`, `read_i64_le`; `read_u128`, `read_u128_be`, `read_u128_le`; `read_i128`, `read_i128_be`, `read_i128_le`; `read_f32`, `read_f32_be`, `read_f32_le`; `read_f64`, `read_f64_be`, `read_f64_le` |
| `BinaryWriteExt` | `write_u8`, `write_i8`; `write_u16`, `write_u16_be`, `write_u16_le`; `write_i16`, `write_i16_be`, `write_i16_le`; `write_u32`, `write_u32_be`, `write_u32_le`; `write_i32`, `write_i32_be`, `write_i32_le`; `write_u64`, `write_u64_be`, `write_u64_le`; `write_i64`, `write_i64_be`, `write_i64_le`; `write_u128`, `write_u128_be`, `write_u128_le`; `write_i128`, `write_i128_be`, `write_i128_le`; `write_f32`, `write_f32_be`, `write_f32_le`; `write_f64`, `write_f64_be`, `write_f64_le` |

Multi-byte runtime-order methods use `ByteOrder::{BigEndian, LittleEndian}`.

## Integer Encodings

| Trait | Methods |
|-------|---------|
| `Leb128IntReadExt` | `read_uleb_u8`, `read_uleb_u8_strict`; `read_uleb_u16`, `read_uleb_u16_strict`; `read_uleb_u32`, `read_uleb_u32_strict`; `read_uleb_u64`, `read_uleb_u64_strict`; `read_uleb_u128`, `read_uleb_u128_strict`; `read_uleb_usize`, `read_uleb_usize_strict`; `read_sleb_i8`, `read_sleb_i8_strict`; `read_sleb_i16`, `read_sleb_i16_strict`; `read_sleb_i32`, `read_sleb_i32_strict`; `read_sleb_i64`, `read_sleb_i64_strict`; `read_sleb_i128`, `read_sleb_i128_strict`; `read_sleb_isize`, `read_sleb_isize_strict` |
| `Leb128IntWriteExt` | `write_uleb_u8`, `write_uleb_u16`, `write_uleb_u32`, `write_uleb_u64`, `write_uleb_u128`, `write_uleb_usize`, `write_sleb_i8`, `write_sleb_i16`, `write_sleb_i32`, `write_sleb_i64`, `write_sleb_i128`, `write_sleb_isize` |
| `ZigZagIntReadExt` | `read_zigzag_i8`, `read_zigzag_i8_strict`; `read_zigzag_i16`, `read_zigzag_i16_strict`; `read_zigzag_i32`, `read_zigzag_i32_strict`; `read_zigzag_i64`, `read_zigzag_i64_strict`; `read_zigzag_i128`, `read_zigzag_i128_strict`; `read_zigzag_isize`, `read_zigzag_isize_strict` |
| `ZigZagIntWriteExt` | `write_zigzag_i8`, `write_zigzag_i16`, `write_zigzag_i32`, `write_zigzag_i64`, `write_zigzag_i128`, `write_zigzag_isize` |

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
| `copy_at_most` | Copies at most `max_bytes` bytes from a reader to a writer. |
| `copy_to_end_limited` | Copies until EOF, returning `InvalidData` if input is longer than `max_bytes`. |
| `content_eq` | Compares two readers for byte-for-byte equality. |
| `compare_content` | Lexicographically compares two readers. |
| `open_buffered_reader` | Opens a file as `BufReader<File>`. |
| `create_file_with_parent` | Creates missing parent directories, then creates a file. |
| `create_buffered_writer_with_parent` | Creates missing parent directories, then creates `BufWriter<File>`. |
| `atomic_write` | Writes bytes through a same-directory temporary file, syncs the temporary file, replaces the destination, and syncs the parent directory when supported. |
| `atomic_write_with` | Same as `atomic_write`, but accepts caller-provided write logic for the temporary file. |

## Wrapper Types

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
