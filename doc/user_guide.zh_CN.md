# Qubit IO 用户指南

当代码需要可复用的 `std::io` helper，但不应该绑定到具体二进制或文本编码格式时，
使用 `qubit-io`。本 crate 只保留通用 I/O 层能力。

## 能力地图

| 领域 | API | 适用场景 |
| --- | --- | --- |
| Buffered byte I/O | `Buffer`、`BufferedInput`、`BufferedOutput` | 上层 adapter 需要 format-agnostic 的 byte window |
| 组合 trait | `ReadSeek`、`ReadWrite`、`ReadWriteSeek`、`BufReadSeek`、`WriteSeek` | API 需要组合 I/O 能力的 trait object |
| Read helper | `ReadExt` | exact-or-EOF 读取、有界读取和复制 helper |
| BufRead helper | `BufReadExt` | 有界 delimiter / line 读取 |
| Seek helper | `SeekExt`、`ReadSeekExt`、`WriteSeekExt` | 查询 stream 大小、保留位置的读写 |
| Stream 工具 | `Streams` | 命名空间式复制和内容比较 |
| Wrapper | `CountingReader`、`LimitReader`、`TeeReader`、checksum wrapper、`PositionGuard` | 为现有 stream 组合小型行为 |

## 安装

```toml
[dependencies]
qubit-io = "0.8"
```

## Buffered Byte I/O

`BufferedInput` 和 `BufferedOutput` 是面向字节的缓冲原语。它们不解码
binary value、不解码文本，也不解析 record；这些能力应该由兄弟 crate 基于 byte
window 组合出来。

```rust
use std::io::{
    BufRead,
    Cursor,
};

use qubit_io::BufferedInput;

let mut input = BufferedInput::with_capacity(
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

`BufferedOutput::into_parts` 不执行 I/O，会返回被包装的 writer 和 pending
bytes。成功结束时，先调用 `flush`，再用 `into_parts` 验证 pending bytes 为空。
如果 flush 失败，调用方仍然持有 buffered output，可以自行重试或拆解。

```rust
use std::io::{
    Cursor,
    Write,
};

use qubit_io::BufferedOutput;

let mut output =
    BufferedOutput::with_capacity(Cursor::new(Vec::<u8>::new()), 4);
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

hot path adapter 可以使用 `unread_raw_parts` 和 `spare_raw_parts_mut`，把完整
backing buffer 和 index 传给 unchecked codec API。通用调用应优先使用
`unread_slice`、`spare_buffer_mut`、`consume` 和 `advance`。

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
