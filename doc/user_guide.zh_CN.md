# Qubit IO 用户指南

当代码需要可复用的 `std::io` helper，但不应该绑定到具体二进制或文本编码格式时，
使用 `qubit-io`。本 crate 只保留通用 I/O 层能力。

## 能力地图

| 领域 | API | 适用场景 |
| --- | --- | --- |
| 组合 trait | `ReadSeek`、`ReadWrite`、`ReadWriteSeek`、`BufReadSeek`、`WriteSeek` | API 需要组合 I/O 能力的 trait object |
| Read helper | `ReadExt` | exact-or-EOF 读取、有界读取和复制 helper |
| BufRead helper | `BufReadExt` | 有界 delimiter / line 读取 |
| Seek helper | `SeekExt`、`ReadSeekExt`、`WriteSeekExt` | 查询 stream 大小、保留位置的读写 |
| Stream 工具 | `Streams` | 命名空间式复制和内容比较 |
| Wrapper | `CountingReader`、`LimitReader`、`TeeReader`、checksum wrapper、`PositionGuard` | 为现有 stream 组合小型行为 |

## 安装

```toml
[dependencies]
qubit-io = "0.6"
```

## Extension Trait

导入需要的方法 trait 后即可调用扩展方法。

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

`ReadExt` 包含 `read_to_end_limited` 和 `read_exact_vec_limited` 等有界 helper。
在协议和文件格式边界，使用这些方法可以避免不受控分配。

`BufReadExt` 提供有界 delimiter 操作：

```rust
use std::io::Cursor;

use qubit_io::BufReadExt;

let mut input = Cursor::new(b"first\nsecond".to_vec());
let line = input.read_line_limited(16)?;

assert_eq!("first\n", line);
# Ok::<(), std::io::Error>(())
```

## 保留位置的 I/O

`ReadSeekExt` 和 `WriteSeekExt` 适合临时随机访问，但不希望改变调用方可见当前位置的场景。

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

`Streams` 是不可实例化的 stream 操作命名空间。

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

比较两个 stream 从当前位置开始的剩余内容时，使用 `Streams::content_eq` 或
`Streams::compare_content`。

## Wrapper

Wrapper 可以在不改变底层资源类型的情况下组合小型 stream 行为。

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

常用 wrapper：

| Wrapper | 用途 |
| --- | --- |
| `CountingReader`、`CountingWriter` | 统计成功读写的字节数 |
| `LimitReader`、`LimitWriter` | 限制可通过的字节数 |
| `TeeReader`、`TeeWriter` | 将成功读写的字节复制到分支 sink |
| `ChecksumReader`、`ChecksumWriter` | 用调用方提供的 hasher 统计成功字节 |
| `PositionGuard` | 除非 dismiss，否则恢复 seek 位置 |

## 不包含的能力

`qubit-io` 不再包含 binary scalar codec、LEB128、ZigZag 或 text charset adapter。
这些能力由兄弟 crate 提供：

- `qubit-codec-binary`：缓冲区级 binary codec；
- `qubit-io-binary`：二进制 stream extension trait 和 wrapper；
- `qubit-codec-text`：文本 codec；
- `qubit-io-text`：文本 stream adapter。
