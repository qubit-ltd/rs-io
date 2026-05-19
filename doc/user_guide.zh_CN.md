# Qubit IO 用户指南

Qubit IO 是一个标准库优先的小型 I/O 辅助 crate。它不替代 `std::io`，
而是给常用能力组合命名，补充保守的 extension method，并提供少量 stream
instrumentation 与有界 I/O wrapper。

## 本 crate 提供什么

- `ReadSeek`、`ReadWriteSeek` 等 object-safe 组合 trait。
- 面向精确读取、有界读取、有界分隔符读取、二进制标量、LEB128、ZigZag
  和长度前缀 UTF-8 字符串的 extension trait。
- `Files` 命名空间，用于父目录创建、随机临时条目、buffered file helper
  和同目录持久化 atomic write。
- `Streams` 和 `Filenames` 命名空间，用于 stream 复制/比较操作和 lexical
  文件名 helper。
- 用于计数、限制、tee、checksum 和恢复 seek 位置的 wrapper 类型。
- 面向偏好 reader/writer object 调用风格的 codec wrapper 类型。

## 导入方式

当模块只需要少量 API 时，优先显式导入：

```rust
use qubit_io::{
    Filenames,
    Files,
    ReadExt,
    Streams,
    WriteSeekExt,
};
```

当调用点大量使用 extension method 时，可以导入 prelude：

```rust
use qubit_io::prelude::*;
```

prelude 只重导出组合 trait、extension trait 和 `ByteOrder`。wrapper 类型、
`Files`、`Streams` 和 `Filenames` 仍然建议从 crate root 显式导入。

## 组合 Trait

Rust 的 trait alias 尚未稳定，而且 trait object 在很多 API 需要的形式下不能
直接组合多个非 auto trait。Qubit IO 定义具名、object-safe 的组合 trait，并为
所有满足约束的类型提供 blanket implementation：

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

当 API 需要通过 trait object 保存或传递不同 I/O 值时，使用这些组合 trait。
如果调用方的具体类型可以继续保留为泛型，优先使用普通泛型约束。

## 精确读取与有界读取

`ReadExt` 覆盖短读安全 helper 和分配保护：

- `read_exact_or_eof` 填满调用方提供的 buffer，或在 EOF 时返回成功的部分字节数。
- `read_exact_array::<N>` 精确读取 `N` 字节到栈上数组。
- `read_exact_vec_limited` 和 `read_exact_vec_limited_into` 会在分配前拒绝过大的
  精确读取。
- `read_to_end_limited` 及字符串变体通过最多读取一个额外字节来识别超限输入。

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

## 二进制标量、LEB128、ZigZag 与字符串

`BinaryReadExt` 和 `BinaryWriteExt` 通过 `_be`、`_le` 或运行时 `ByteOrder`
方法读写基础标量值。

`Leb128ReadExt` 和 `Leb128WriteExt` 通过 `read_uleb_u32`、`write_sleb_i64`
等整数类型专用方法读写 unsigned / signed LEB128。带 `_strict` 后缀的读取方法
会拒绝 non-canonical 编码。

`ZigZagReadExt` 和 `ZigZagWriteExt` 使用 unsigned LEB128 payload 读写
ZigZag 映射后的有符号整数。严格读取方法要求底层 unsigned LEB128 payload
使用 canonical LEB128 编码。

`StringReadExt` 和 `StringWriteExt` 使用 ULEB128、`u16` 或 `u32` 字节长度前缀
读写 UTF-8 字符串。ULEB 字符串读取包含 `read_utf8_string_uleb_strict`，它会在
读取 payload 前拒绝 non-canonical ULEB 长度前缀。所有字符串读取都要求传入
`max_len`，并在分配前拒绝超限 payload 长度。

## 文件工具

请使用 `Files` associated method，而不是旧的 free function：

- `ensure_dir` 和 `ensure_parent` 创建缺失目录。
- `open_buffered_reader`、`create_file_with_parent` 和
  `create_buffered_writer_with_parent` 处理常见文件打开模式。
- `random_file_name`、`temp_dir` 和 `temp_path` 构造随机临时名称和路径。
- `create_temp_file`、`create_temp_file_with`、`create_temp_file_in`、
  `create_temp_dir_with` 和 `create_temp_dir_in` 使用 `getrandom` 支持的
  OS 随机源创建抗碰撞随机临时条目。
- `atomic_write` 和 `atomic_write_with` 通过随机同目录临时文件写入，flush
  并 sync 临时文件，替换目标文件，并在平台支持时 sync 父目录。

```rust
use qubit_io::Files;

let dir = Files::create_temp_dir_with(Some("qubit-io-guide-"), 16)?;
let path = dir.join("nested").join("data.bin");

Files::atomic_write(&path, b"payload")?;
assert_eq!(b"payload", std::fs::read(&path)?.as_slice());

std::fs::remove_dir_all(dir)?;
# Ok::<(), std::io::Error>(())
```

## Stream 工具

当一个操作涉及多个 stream，或作为命名空间级 helper 更清晰时，使用 `Streams`：

- `copy` 转发到 `std::io::copy`，保留标准库的优化复制路径。
- `copy_at_most` 最多复制指定字节数。
- `copy_to_end_limited` 要求剩余输入必须在长度限制内到达 EOF。
- `content_eq` 和 `compare_content` 在比较两个 reader 剩余字节时消费它们。

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

## 文件名工具

`Filenames` 用于不访问文件系统的 lexical 文件名操作：

- `file_name`、`file_name_str`、`file_stem_str`、`file_prefix_str` 和
  `extension_str` 暴露常用 `Path` component。
- `dot_extension`、`has_extension` 和 `has_extension_ignore_ascii_case`
  覆盖常见扩展名判断。
- `file_name_from_path` 从包含 `/` 或 `\` 分隔符的字符串中提取最后一段。
- `file_name_from_url` 移除 query/fragment 后缀，并解码 URL 最后一段中的
  percent-encoded UTF-8。

基于 `Path` 的 helper 遵循 `std::path::Path` 语义。尤其是 `.env` 这类
dotfile 没有扩展名，除非文件名中还有另一个点号。

```rust
use qubit_io::Filenames;
use std::path::Path;

let path = Path::new("/tmp/archive.tar.gz");

assert_eq!(Some("archive.tar"), Filenames::file_stem_str(path));
assert_eq!(Some("gz"), Filenames::extension_str(path));
assert!(Filenames::has_extension(path, ".gz"));
assert_eq!(
    "my file.txt",
    Filenames::file_name_from_url("https://example.com/my%20file.txt")
);
```

## Stream Wrapper

wrapper 会透明包裹底层 reader 或 writer，并实现对应标准库 I/O trait：

- `CountingReader` 和 `CountingWriter` 统计成功传输的字节数。
- `LimitReader` 和 `LimitWriter` 最多暴露或接受固定数量的字节。
- `TeeReader` 和 `TeeWriter` 把已接受的字节镜像到 branch writer。
- `ChecksumReader` 和 `ChecksumWriter` 更新调用方提供的 `Hasher`。
- `PositionGuard` 在 drop 时把 seekable stream 恢复到捕获的位置，除非显式
  restore 或 dismiss。

## Codec Wrapper

当显式 reader/writer object 比在调用点导入 extension trait 更清晰时，可以使用
这些 root-level wrapper 类型：

- `BinaryReader` 和 `BinaryWriter`
- `Leb128Reader` 和 `Leb128Writer`
- `ZigZagReader` 和 `ZigZagWriter`

这些 wrapper 持有底层 stream，提供 `get_ref`、`get_mut` 和 `into_inner`，
并复用与 extension trait 相同的编码实现。`BinaryReader` 和 `BinaryWriter`
还保存运行时 `ByteOrder`。`Leb128Reader` 和 `ZigZagReader` 保存运行时
strictness flag，因此 object 风格 API 使用 `read_u16`、`read_i32` 这类短
方法名；需要 canonical LEB128 校验时，用 `with_strict` 创建，或通过
`set_strict` 切换。

## API 矩阵

本矩阵汇总 crate root 公开 API。

### Prelude

| 模块 | 重导出内容 |
|------|------------|
| `qubit_io::prelude` | `BinaryReadExt`、`BinaryWriteExt`、`BufReadExt`、`BufReadSeek`、`ByteOrder`、`Leb128ReadExt`、`Leb128WriteExt`、`ReadExt`、`ReadSeek`、`ReadSeekExt`、`ReadWrite`、`ReadWriteSeek`、`SeekExt`、`StringReadExt`、`StringWriteExt`、`WriteSeek`、`WriteSeekExt`、`ZigZagReadExt`、`ZigZagWriteExt` |

### 组合 Trait

| Trait | 标准库约束 | 用途 |
|------|------------|------|
| `ReadSeek` | `Read + Seek` | 可读取的随机访问输入。 |
| `BufReadSeek` | `BufRead + Seek` | 带缓冲的可读取随机访问输入。 |
| `ReadWrite` | `Read + Write` | 双向 stream 或可变缓冲区。 |
| `WriteSeek` | `Write + Seek` | 可写入的随机访问输出。 |
| `ReadWriteSeek` | `Read + Write + Seek` | 完整可变的随机访问 I/O 对象。 |

### Extension Trait

| Trait | 方法 | 说明 |
|------|------|------|
| `ReadExt` | `read_exact_or_eof`、`read_exact_array`、`read_exact_vec_limited`、`read_exact_vec_limited_into`、`discard_exact_or_eof`、`copy_to`、`copy_to_at_most`、`copy_to_end_limited`、`read_to_end_limited`、`read_to_end_limited_into`、`read_to_string_limited`、`read_to_string_limited_into` | 短读安全读取、精确读取、有界复制、有界字节读取和有界 UTF-8 文本读取。 |
| `BufReadExt` | `read_until_limited`、`read_until_limited_into`、`read_line_limited`、`read_line_limited_into`、`discard_until_limited` | 面向 buffered reader 的有界分隔符和行读取/丢弃。 |
| `SeekExt` | `stream_size` | 获取 stream 大小并恢复原位置。 |
| `ReadSeekExt` | `peek_exact_or_eof`、`read_exact_or_eof_at` | 保持位置不变的 peek 和随机 offset 读取。 |
| `WriteSeekExt` | `write_all_at_preserving_position` | 保持位置不变的随机 offset 写入。 |

### 二进制标量

| Trait | 方法 |
|------|------|
| `BinaryReadExt` | `read_u8`、`read_i8`；`read_u16`、`read_u16_be`、`read_u16_le`；`read_i16`、`read_i16_be`、`read_i16_le`；`read_u32`、`read_u32_be`、`read_u32_le`；`read_i32`、`read_i32_be`、`read_i32_le`；`read_u64`、`read_u64_be`、`read_u64_le`；`read_i64`、`read_i64_be`、`read_i64_le`；`read_u128`、`read_u128_be`、`read_u128_le`；`read_i128`、`read_i128_be`、`read_i128_le`；`read_f32`、`read_f32_be`、`read_f32_le`；`read_f64`、`read_f64_be`、`read_f64_le` |
| `BinaryWriteExt` | `write_u8`、`write_i8`；`write_u16`、`write_u16_be`、`write_u16_le`；`write_i16`、`write_i16_be`、`write_i16_le`；`write_u32`、`write_u32_be`、`write_u32_le`；`write_i32`、`write_i32_be`、`write_i32_le`；`write_u64`、`write_u64_be`、`write_u64_le`；`write_i64`、`write_i64_be`、`write_i64_le`；`write_u128`、`write_u128_be`、`write_u128_le`；`write_i128`、`write_i128_be`、`write_i128_le`；`write_f32`、`write_f32_be`、`write_f32_le`；`write_f64`、`write_f64_be`、`write_f64_le` |

多字节运行时字节序方法使用 `ByteOrder::{BigEndian, LittleEndian}`。

### 整数编码

| Trait | 方法 |
|------|------|
| `Leb128ReadExt` | `read_uleb_u8`、`read_uleb_u8_strict`；`read_uleb_u16`、`read_uleb_u16_strict`；`read_uleb_u32`、`read_uleb_u32_strict`；`read_uleb_u64`、`read_uleb_u64_strict`；`read_uleb_u128`、`read_uleb_u128_strict`；`read_uleb_usize`、`read_uleb_usize_strict`；`read_sleb_i8`、`read_sleb_i8_strict`；`read_sleb_i16`、`read_sleb_i16_strict`；`read_sleb_i32`、`read_sleb_i32_strict`；`read_sleb_i64`、`read_sleb_i64_strict`；`read_sleb_i128`、`read_sleb_i128_strict`；`read_sleb_isize`、`read_sleb_isize_strict` |
| `Leb128WriteExt` | `write_uleb_u8`、`write_uleb_u16`、`write_uleb_u32`、`write_uleb_u64`、`write_uleb_u128`、`write_uleb_usize`、`write_sleb_i8`、`write_sleb_i16`、`write_sleb_i32`、`write_sleb_i64`、`write_sleb_i128`、`write_sleb_isize` |
| `ZigZagReadExt` | `read_zigzag_i8`、`read_zigzag_i8_strict`；`read_zigzag_i16`、`read_zigzag_i16_strict`；`read_zigzag_i32`、`read_zigzag_i32_strict`；`read_zigzag_i64`、`read_zigzag_i64_strict`；`read_zigzag_i128`、`read_zigzag_i128_strict`；`read_zigzag_isize`、`read_zigzag_isize_strict` |
| `ZigZagWriteExt` | `write_zigzag_i8`、`write_zigzag_i16`、`write_zigzag_i32`、`write_zigzag_i64`、`write_zigzag_i128`、`write_zigzag_isize` |

LEB128 参考 WebAssembly Core binary value encoding：
<https://webassembly.github.io/spec/core/binary/values.html#integers>。

ZigZag 参考 Protocol Buffers signed integer mapping：
<https://protobuf.dev/programming-guides/encoding/#signed-integers>。

### 长度前缀 UTF-8 字符串

| Trait | 方法 | 限制行为 |
|------|------|----------|
| `StringReadExt` | `read_utf8_string_uleb`、`read_utf8_string_uleb_strict`、`read_utf8_string_u16_be`、`read_utf8_string_u16_le`、`read_utf8_string_u32_be`、`read_utf8_string_u32_le` | 每个读取方法都要求传入 `max_len`，并在分配 payload buffer 前拒绝超过限制的长度。严格 ULEB 变体还会拒绝 non-canonical 长度前缀。 |
| `StringWriteExt` | `write_utf8_string_uleb`、`write_utf8_string_u16_be`、`write_utf8_string_u16_le`、`write_utf8_string_u32_be`、`write_utf8_string_u32_le` | 固定宽度长度前缀方法会拒绝 UTF-8 字节长度无法放入对应前缀类型的字符串。 |

### 文件工具

| API | 用途 |
|-----|------|
| `Files::DEFAULT_TEMP_FILE_PREFIX` | 随机临时文件名的默认前缀。 |
| `Files::DEFAULT_TEMP_FILE_RETRIES` | 随机临时条目创建的默认重试次数。 |
| `Files::open_buffered_reader` | 以 `BufReader<File>` 形式打开文件。 |
| `Files::ensure_dir` | 创建目录及缺失祖先目录。 |
| `Files::ensure_parent` | 为文件路径创建缺失父目录。 |
| `Files::create_file_with_parent` | 创建缺失父目录后创建文件。 |
| `Files::create_buffered_writer_with_parent` | 创建缺失父目录后创建 `BufWriter<File>`。 |
| `Files::random_file_name` | 基于可选前缀和后缀生成随机名称。 |
| `Files::temp_dir` | 返回进程临时目录。 |
| `Files::temp_path` | 在进程临时目录下构造随机路径。 |
| `Files::create_temp_file` | 在进程临时目录下创建随机临时文件。 |
| `Files::create_temp_file_with` | 使用调用方提供的命名和重试参数，在进程临时目录下创建随机临时文件。 |
| `Files::create_temp_file_in` | 在调用方提供的目录下创建随机临时文件。 |
| `Files::create_temp_dir_with` | 在进程临时目录下创建随机临时目录。 |
| `Files::create_temp_dir_in` | 在调用方提供的目录下创建随机临时目录。 |
| `Files::atomic_write` | 使用同目录临时文件写入，sync 临时文件，替换目标文件，并在支持的平台上 sync 父目录。 |
| `Files::atomic_write_with` | 与 `atomic_write` 相同，但由调用方提供临时文件写入逻辑。 |

### Stream 工具

| API | 用途 |
|-----|------|
| `Streams::copy` | `std::io::copy` 的命名空间式包装。 |
| `Streams::copy_at_most` | 从 reader 向 writer 最多复制 `max_bytes` 字节。 |
| `Streams::copy_to_end_limited` | 一直复制到 EOF；如果输入长度超过 `max_bytes`，返回 `InvalidData`。 |
| `Streams::content_eq` | 判断两个 reader 的内容是否逐字节相同。 |
| `Streams::compare_content` | 对两个 reader 的内容做字典序比较。 |

### 文件名工具

| API | 用途 |
|-----|------|
| `Filenames::file_name` | 返回最终文件名 component，类型为 `OsStr`。 |
| `Filenames::file_name_str` | 返回 UTF-8 最终文件名 component。 |
| `Filenames::file_stem_str` | 按 `Path::file_stem` 语义返回 UTF-8 file stem。 |
| `Filenames::file_prefix_str` | 按 `Path::file_prefix` 语义返回 UTF-8 file prefix。 |
| `Filenames::extension_str` | 按 `Path::extension` 语义返回 UTF-8 最终扩展名。 |
| `Filenames::dot_extension` | 返回带点号前缀的最终扩展名。 |
| `Filenames::has_extension` | 做大小写敏感的最终扩展名判断。 |
| `Filenames::has_extension_ignore_ascii_case` | 做 ASCII 大小写不敏感的最终扩展名判断。 |
| `Filenames::file_name_from_path` | 从包含 `/` 或 `\` 分隔符的字符串中提取最后一段。 |
| `Filenames::file_name_from_url` | 提取并 percent-decode URL 最后一段。 |

### Wrapper 类型

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

### Codec Wrapper 类型

| 类型 | 用途 |
|------|----------|
| `BinaryReader` | 用于二进制标量和固定宽度长度前缀字符串解码的 reader object。 |
| `BinaryWriter` | 用于二进制标量和固定宽度长度前缀字符串编码的 writer object。 |
| `Leb128Reader` | 用于 LEB128 整数和 ULEB128 长度前缀字符串解码的 reader object，可配置 strict canonical 解码。 |
| `Leb128Writer` | 用于 LEB128 整数和 ULEB128 长度前缀字符串编码的 writer object。 |
| `ZigZagReader` | 用于 ZigZag 有符号整数解码的 reader object，可配置底层 ULEB128 strict 校验。 |
| `ZigZagWriter` | 用于 ZigZag 有符号整数编码的 writer object。 |

## 依赖项

Qubit IO 运行时依赖 Rust 标准库和 `getrandom`。`getrandom` 用于为 `Files`
helper 生成随机临时文件名和目录名。
