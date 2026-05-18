# Qubit IO API 矩阵

本文档汇总 `qubit_io` 从 crate root 对外重导出的公开 API。

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
| `ReadExt` | `read_exact_or_eof`、`discard_exact_or_eof`、`copy_to`、`copy_to_limited`、`read_to_end_limited`、`read_to_string_limited` | 短读安全读取、有界复制、有界字节读取和有界 UTF-8 文本读取。 |
| `BufReadExt` | `read_until_limited`、`read_line_limited`、`discard_until_limited` | 面向 buffered reader 的有界分隔符和行读取/丢弃。 |
| `SeekExt` | `stream_size` | 获取 stream 大小并恢复原位置；如果标准库未来加入 `Seek::stream_size`，该方法可能被弃用。 |
| `ReadSeekExt` | `peek_exact_or_eof`、`read_exact_or_eof_at` | 保持位置不变的 peek 和随机 offset 读取。 |
| `WriteSeekExt` | `write_all_at_preserving_position` | 保持位置不变的随机 offset 写入。 |

## 二进制标量

| Trait | 支持的值 | 字节序支持 |
|------|----------|------------|
| `BinaryReadExt` | `u8`、`i8`、`u16`、`i16`、`u32`、`i32`、`u64`、`i64`、`u128`、`i128`、`f32`、`f64` | 多字节值支持 `_be`、`_le` 和运行时 `ByteOrder` 方法。 |
| `BinaryWriteExt` | `u8`、`i8`、`u16`、`i16`、`u32`、`i32`、`u64`、`i64`、`u128`、`i128`、`f32`、`f64` | 多字节值支持 `_be`、`_le` 和运行时 `ByteOrder` 方法。 |

## 整数编码

| Trait | 支持的值 | strict 解码 |
|------|----------|-------------|
| `Leb128IntReadExt` | unsigned `u8`、`u16`、`u32`、`u64`、`u128`、`usize`；signed `i8`、`i16`、`i32`、`i64`、`i128`、`isize` | 每个读取方法都有 `_strict` 变体，用于拒绝非 canonical LEB128 编码。 |
| `Leb128IntWriteExt` | unsigned `u8`、`u16`、`u32`、`u64`、`u128`、`usize`；signed `i8`、`i16`、`i32`、`i64`、`i128`、`isize` | 写入方法始终输出 canonical LEB128 编码。 |
| `ZigZagIntReadExt` | `i8`、`i16`、`i32`、`i64`、`i128`、`isize` | 每个读取方法都有 `_strict` 变体，要求 unsigned LEB128 payload 为 canonical 编码。 |
| `ZigZagIntWriteExt` | `i8`、`i16`、`i32`、`i64`、`i128`、`isize` | 写入方法输出 ZigZag 映射后的 canonical unsigned LEB128 payload。 |

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
| `copy_limited` | 从 reader 向 writer 最多复制 `max_bytes` 字节。 |
| `content_eq` | 判断两个 reader 的内容是否逐字节相同。 |
| `compare_content` | 对两个 reader 的内容做字典序比较。 |
| `open_buffered_reader` | 以 `BufReader<File>` 形式打开文件。 |
| `create_file_with_parent` | 创建缺失的父目录后创建文件。 |
| `create_buffered_writer_with_parent` | 创建缺失的父目录后创建 `BufWriter<File>`。 |
| `atomic_write` | 使用同目录临时文件写入，sync 临时文件，替换目标文件，并在支持的平台上 sync 父目录。 |
| `atomic_write_with` | 与 `atomic_write` 相同，但由调用方提供临时文件写入逻辑。 |

## Wrapper 类型

| 类型 | 实现 | 用途 |
|------|------|------|
| `CountingReader` | `Read` | 统计成功读取的字节数。 |
| `CountingWriter` | `Write` | 统计成功写入的字节数。 |
| `LimitReader` | `Read` | 限制从 inner reader 读取的字节数。 |
| `LimitWriter` | `Write` | 限制写入 inner writer 的字节数。 |
| `TeeReader` | `Read` | 把成功读取的字节复制到 branch writer。 |
| `TeeWriter` | `Write` | 写入 primary writer，并把字节镜像到 branch writer。 |
| `ChecksumReader` | `Read` | 对成功读取的字节更新调用方提供的 checksum 状态。 |
| `ChecksumWriter` | `Write` | 对成功写入的字节更新调用方提供的 checksum 状态。 |
| `PositionGuard` | `Seek` guard | 在 drop 时恢复原 stream 位置，除非被 dismiss。 |
