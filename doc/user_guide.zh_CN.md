# Qubit IO 用户手册

Qubit IO 是 Qubit Rust crate 家族中的 stream 和字节 I/O crate。它只关注
`std::io` trait、extension method、wrapper 和小型 codec helper。

文件系统相关 helper 已移动到 `qubit-local-fs`。

## 导入方式

具体 wrapper 和工具命名空间可以从 crate root 导入：

```rust
use qubit_io::{
    CountingReader,
    ReadExt,
    ReadSeek,
    Streams,
};
```

如果一个模块主要需要 method-providing extension trait 和组合 trait，可以使用
prelude：

```rust
use qubit_io::prelude::*;
```

## Stream Helper

`Streams` 提供围绕 `std::io::Read` 和 `std::io::Write` 的 associated function：

- `copy` 委托给 `std::io::copy`；
- `copy_at_most` 最多复制指定字节数；
- `copy_to_end_limited` 只有在限制内到达 EOF 时才完成复制；
- `content_eq` 和 `compare_content` 比较 readable stream 内容。

```rust
use std::io::Cursor;

use qubit_io::Streams;

let mut left = Cursor::new(b"abc".to_vec());
let mut right = Cursor::new(Vec::new());

Streams::copy(&mut left, &mut right)?;
assert_eq!(b"abc", right.into_inner().as_slice());

# Ok::<(), std::io::Error>(())
```

## Extension Trait

`ReadExt`、`BufReadExt`、`SeekExt`、`ReadSeekExt` 和 `WriteSeekExt` 提供常用的
底层 I/O 操作，并保持对标准库 I/O trait 的泛型支持。

```rust
use std::io::Cursor;

use qubit_io::{ReadExt, WriteSeekExt};

let mut input = Cursor::new(b"hello".to_vec());
let bytes = input.read_to_end_limited(16)?;
assert_eq!(b"hello", bytes.as_slice());

let mut output = Cursor::new(vec![0; 8]);
output.write_all_at_preserving_position(2, b"rs")?;

# Ok::<(), std::io::Error>(())
```

## Codec Helper

Binary、LEB128、ZigZag 和 length-prefixed string helper 同时提供 extension trait
和 reader/writer wrapper 两种调用方式。

```rust
use std::io::Cursor;

use qubit_io::{BinaryReadExt, BinaryWriteExt};

let mut buffer = Vec::new();
buffer.write_u32_be(0x0102_0304)?;

let mut cursor = Cursor::new(buffer);
assert_eq!(0x0102_0304, cursor.read_u32_be()?);

# Ok::<(), std::io::Error>(())
```

## Wrapper

当 stream 行为应该成为类型语义而不是一次函数调用时，可以使用 wrapper：

- `CountingReader` 和 `CountingWriter` 统计成功读写的字节数；
- `LimitReader` 和 `LimitWriter` 限制字节预算；
- `TeeReader` 和 `TeeWriter` 把流量复制到 branch writer；
- `ChecksumReader` 和 `ChecksumWriter` 更新调用方提供的 checksum 状态；
- `PositionGuard` 在 drop 时恢复 seek 位置，除非主动 dismiss。

## Crate 边界

`qubit-io` 不再包含本地文件系统工具。`Files`、`Filenames`、`TempFile`、
`TempDir`、递归目录复制、清理 helper 和 atomic file write 请使用
`qubit-local-fs`。
