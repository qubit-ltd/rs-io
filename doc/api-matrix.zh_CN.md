# Qubit IO API 矩阵

本文档汇总 `qubit_io` 从 crate root 对外重导出的公开 API。

## Prelude

| 模块 | 重导出内容 |
|------|------------|
| `qubit_io::prelude` | `BinaryReadExt`、`BinaryWriteExt`、`BufReadExt`、`BufReadSeek`、`Leb128IntReadExt`、`Leb128IntWriteExt`、`ReadExt`、`ReadSeek`、`ReadSeekExt`、`ReadWrite`、`ReadWriteSeek`、`SeekExt`、`StringReadExt`、`StringWriteExt`、`WriteSeek`、`WriteSeekExt`、`ZigZagIntReadExt`、`ZigZagIntWriteExt` |

## 组合 Trait

| Trait | 标准库约束 | 用途 |
|------|------------|------|
| `ReadSeek` | `Read + Seek` | 可读取的随机访问输入。 |
| `BufReadSeek` | `BufRead + Seek` | 带缓冲的可读取随机访问输入。 |
| `ReadWrite` | `Read + Write` | 双向 stream 或可变缓冲区。 |
| `WriteSeek` | `Write + Seek` | 可写入的随机访问输出。 |
| `ReadWriteSeek` | `Read + Write + Seek` | 完整可变的随机访问 I/O 对象。 |

## 扩展 Trait

| Trait | 方法 | 说明 |
|------|------|------|
| `ReadExt` | `read_exact_or_eof`、`discard_exact_or_eof`、`copy_to`、`copy_to_at_most`、`copy_to_end_limited`、`read_to_end_limited`、`read_to_end_limited_into`、`read_to_string_limited`、`read_to_string_limited_into` | 短读安全读取、有界复制、有界字节读取和有界 UTF-8 文本读取。 |
| `BufReadExt` | `read_until_limited`、`read_until_limited_into`、`read_line_limited`、`read_line_limited_into`、`discard_until_limited` | 面向 buffered reader 的有界分隔符和行读取/丢弃。 |
| `SeekExt` | `stream_size` | 获取 stream 大小并恢复原位置；如果标准库未来加入 `Seek::stream_size`，该方法可能被弃用。 |
| `ReadSeekExt` | `peek_exact_or_eof`、`read_exact_or_eof_at` | 保持位置不变的 peek 和随机 offset 读取。 |
| `WriteSeekExt` | `write_all_at_preserving_position` | 保持位置不变的随机 offset 写入。 |

## 二进制标量

| Trait | 方法 |
|------|------|
| `BinaryReadExt` | `read_u8`、`read_i8`；`read_u16`、`read_u16_be`、`read_u16_le`；`read_i16`、`read_i16_be`、`read_i16_le`；`read_u32`、`read_u32_be`、`read_u32_le`；`read_i32`、`read_i32_be`、`read_i32_le`；`read_u64`、`read_u64_be`、`read_u64_le`；`read_i64`、`read_i64_be`、`read_i64_le`；`read_u128`、`read_u128_be`、`read_u128_le`；`read_i128`、`read_i128_be`、`read_i128_le`；`read_f32`、`read_f32_be`、`read_f32_le`；`read_f64`、`read_f64_be`、`read_f64_le` |
| `BinaryWriteExt` | `write_u8`、`write_i8`；`write_u16`、`write_u16_be`、`write_u16_le`；`write_i16`、`write_i16_be`、`write_i16_le`；`write_u32`、`write_u32_be`、`write_u32_le`；`write_i32`、`write_i32_be`、`write_i32_le`；`write_u64`、`write_u64_be`、`write_u64_le`；`write_i64`、`write_i64_be`、`write_i64_le`；`write_u128`、`write_u128_be`、`write_u128_le`；`write_i128`、`write_i128_be`、`write_i128_le`；`write_f32`、`write_f32_be`、`write_f32_le`；`write_f64`、`write_f64_be`、`write_f64_le` |

多字节运行时字节序方法使用 `ByteOrder::{BigEndian, LittleEndian}`。

## 整数编码

| Trait | 方法 |
|------|------|
| `Leb128IntReadExt` | `read_uleb_u8`、`read_uleb_u8_strict`；`read_uleb_u16`、`read_uleb_u16_strict`；`read_uleb_u32`、`read_uleb_u32_strict`；`read_uleb_u64`、`read_uleb_u64_strict`；`read_uleb_u128`、`read_uleb_u128_strict`；`read_uleb_usize`、`read_uleb_usize_strict`；`read_sleb_i8`、`read_sleb_i8_strict`；`read_sleb_i16`、`read_sleb_i16_strict`；`read_sleb_i32`、`read_sleb_i32_strict`；`read_sleb_i64`、`read_sleb_i64_strict`；`read_sleb_i128`、`read_sleb_i128_strict`；`read_sleb_isize`、`read_sleb_isize_strict` |
| `Leb128IntWriteExt` | `write_uleb_u8`、`write_uleb_u16`、`write_uleb_u32`、`write_uleb_u64`、`write_uleb_u128`、`write_uleb_usize`、`write_sleb_i8`、`write_sleb_i16`、`write_sleb_i32`、`write_sleb_i64`、`write_sleb_i128`、`write_sleb_isize` |
| `ZigZagIntReadExt` | `read_zigzag_i8`、`read_zigzag_i8_strict`；`read_zigzag_i16`、`read_zigzag_i16_strict`；`read_zigzag_i32`、`read_zigzag_i32_strict`；`read_zigzag_i64`、`read_zigzag_i64_strict`；`read_zigzag_i128`、`read_zigzag_i128_strict`；`read_zigzag_isize`、`read_zigzag_isize_strict` |
| `ZigZagIntWriteExt` | `write_zigzag_i8`、`write_zigzag_i16`、`write_zigzag_i32`、`write_zigzag_i64`、`write_zigzag_i128`、`write_zigzag_isize` |

LEB128 参考 WebAssembly Core binary value encoding：
<https://webassembly.github.io/spec/core/binary/values.html#integers>。

ZigZag 参考 Protocol Buffers signed integer mapping：
<https://protobuf.dev/programming-guides/encoding/#signed-integers>。

## 长度前缀 UTF-8 字符串

| Trait | 方法 | 限制行为 |
|------|------|----------|
| `StringReadExt` | `read_utf8_string_uleb`、`read_utf8_string_u16_be`、`read_utf8_string_u16_le`、`read_utf8_string_u32_be`、`read_utf8_string_u32_le` | 每个读取方法都要求传入 `max_len`，并在分配 payload buffer 前拒绝超过限制的长度。 |
| `StringWriteExt` | `write_utf8_string_uleb`、`write_utf8_string_u16_be`、`write_utf8_string_u16_le`、`write_utf8_string_u32_be`、`write_utf8_string_u32_le` | 固定宽度长度前缀方法会拒绝 UTF-8 字节长度无法放入对应前缀类型的字符串。 |

## 工具函数

| 函数 | 用途 |
|------|------|
| `copy_at_most` | 从 reader 向 writer 最多复制 `max_bytes` 字节。 |
| `copy_to_end_limited` | 一直复制到 EOF；如果输入长度超过 `max_bytes`，返回 `InvalidData`。 |
| `content_eq` | 判断两个 reader 的内容是否逐字节相同。 |
| `compare_content` | 对两个 reader 的内容做字典序比较。 |
| `open_buffered_reader` | 以 `BufReader<File>` 形式打开文件。 |
| `create_file_with_parent` | 创建缺失的父目录后创建文件。 |
| `create_buffered_writer_with_parent` | 创建缺失的父目录后创建 `BufWriter<File>`。 |
| `atomic_write` | 使用同目录临时文件写入，sync 临时文件，替换目标文件，并在支持的平台上 sync 父目录。 |
| `atomic_write_with` | 与 `atomic_write` 相同，但由调用方提供临时文件写入逻辑。 |

## Wrapper 类型

| 类型 | 实现 | 公开方法 |
|------|------|----------|
| `CountingReader` | `Read` | `new`、`bytes_read`、`get_ref`、`get_mut`、`into_inner` |
| `CountingWriter` | `Write` | `new`、`bytes_written`、`get_ref`、`get_mut`、`into_inner` |
| `LimitReader` | `Read` | `new`、`remaining`、`get_ref`、`get_mut`、`into_inner` |
| `LimitWriter` | `Write` | `new`、`remaining`、`get_ref`、`get_mut`、`into_inner` |
| `TeeReader` | `Read` | `new`、`reader_ref`、`reader_mut`、`branch_ref`、`branch_mut`、`into_inner` |
| `TeeWriter` | `Write` | `new`、`primary_ref`、`primary_mut`、`branch_ref`、`branch_mut`、`into_inner` |
| `ChecksumReader` | `Read` | `new`、`checksum`、`get_ref`、`get_mut`、`hasher_ref`、`hasher_mut`、`into_inner` |
| `ChecksumWriter` | `Write` | `new`、`checksum`、`get_ref`、`get_mut`、`hasher_ref`、`hasher_mut`、`into_inner` |
| `PositionGuard` | `Seek` drop guard | `new`、`position`、`get_mut`、`restore`、`dismiss` |
