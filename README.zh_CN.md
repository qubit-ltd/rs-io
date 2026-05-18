# Qubit IO

[![Rust CI](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io/coverage-badge.json)](https://qubit-ltd.github.io/rs-io/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io.svg?color=blue)](https://crates.io/crates/qubit-io)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

面向 Rust 的小型 I/O trait 与扩展工具库。

## 概述

Qubit IO 在 `std::io` 之上提供两层很小的能力：

- 为常用 `std::io` 能力组合提供 object-safe 的组合 trait；
- 为标准库留给调用方反复手写的底层 I/O 模式提供 extension trait。

组合 trait 适合 API 需要使用 `&mut dyn ReadSeek` 或
`Box<dyn ReadWriteSeek>` 这类 trait object，而不是 `R: Read + Seek`
这类泛型约束的场景。

extension trait 覆盖的是保守、标准库优先的行为，例如：尽量读满 buffer 但 EOF
正常返回已读长度；从可 seek stream 中 peek 数据但不消费当前位置；在指定 offset
写入后恢复调用方原来的位置。

## 设计目标

- **object-safe 组合**：提供适合 trait object 使用的具名 I/O 约束。
- **标准库优先**：直接基于 `std::io::{Read, Write, Seek}` 构建。
- **不引入包装类型**：基于标准 I/O trait 做 blanket implementation。
- **极小 API 表面积**：只保留跨 crate 复用价值高的通用底层操作。
- **位置安全**：把不消费当前位置的探测和随机访问 patch 写入显式表达出来。
- **便于集成**：可用于 cursor、文件、缓冲区、stream 和自定义 I/O 类型。

## 特性

### Object-Safe I/O Trait 组合

- **`ReadSeek`**：组合 `Read` 与 `Seek`，用于可读取的随机访问输入。
- **`BufReadSeek`**：组合 `BufRead` 与 `Seek`，用于带缓冲的随机访问输入。
- **`ReadWrite`**：组合 `Read` 与 `Write`，用于双向 stream 或缓冲区。
- **`WriteSeek`**：组合 `Write` 与 `Seek`，用于可写入的随机访问输出。
- **`ReadWriteSeek`**：组合 `Read`、`Write` 与 `Seek`，用于完整可变的随机访问 I/O 对象。

### I/O Extension Trait

- **`ReadExt`**：
  - `read_exact_or_eof` 会在短读时继续读取，直到目标 buffer 被填满或遇到 EOF。
  - `discard_exact_or_eof` 不分配内存，最多消费并丢弃指定字节数。
  - `copy_to` 与 `copy_to_limited` 以方法形式把内容复制到 writer。
  - `read_to_end_limited` 在最大长度限制内把剩余输入读入 `Vec<u8>`。
- **`SeekExt`**：
  - `stream_size` 获取 stream 大小并恢复原位置。
- **`ReadSeekExt`**：
  - `peek_exact_or_eof` 从当前位置读取并恢复原位置。
  - `read_exact_or_eof_at` 从绝对 offset 读取并恢复原位置。
- **`WriteSeekExt`**：
  - `write_all_at_preserving_position` 在绝对 offset 写入并恢复原位置。
- **`BinaryReadExt` / `BinaryWriteExt`**：
  - 支持通过 `_be` / `_le` 后缀方法或运行时 `ByteOrder` 读写基础数字标量。

### Blanket Implementation

所有实现了对应标准库 trait 的类型，都会自动实现 Qubit IO 的组合 trait。
对 `std::io::Cursor`、`std::fs::File` 或自定义 I/O 类型，都不需要额外编写
adapter 代码。

## 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
qubit-io = "0.2"
```

## 快速开始

### 读取并定位

当函数既需要从当前位置读取，又需要在输入中移动位置时，使用 `ReadSeek`。

```rust
use qubit_io::ReadSeek;
use std::io::SeekFrom;

fn read_second_byte(input: &mut dyn ReadSeek) -> std::io::Result<u8> {
    input.seek(SeekFrom::Start(1))?;

    let mut byte = [0; 1];
    input.read_exact(&mut byte)?;
    Ok(byte[0])
}

fn main() -> std::io::Result<()> {
    let mut cursor = std::io::Cursor::new(b"abc".to_vec());
    assert_eq!(read_second_byte(&mut cursor)?, b'b');
    Ok(())
}
```

### 精确读取或 EOF 正常返回

当短读需要继续读取，但 EOF 先到时又不希望返回 `UnexpectedEof`，可以使用
`ReadExt::read_exact_or_eof`。

```rust
use qubit_io::ReadExt;

fn read_prefix(input: &mut dyn std::io::Read) -> std::io::Result<Vec<u8>> {
    let mut buffer = vec![0; 8];
    let count = input.read_exact_or_eof(&mut buffer)?;
    buffer.truncate(count);
    Ok(buffer)
}

fn main() -> std::io::Result<()> {
    let mut cursor = std::io::Cursor::new(b"abc".to_vec());
    assert_eq!(read_prefix(&mut cursor)?, b"abc");
    Ok(())
}
```

### 不消费当前位置地探测内容

当需要检查可 seek stream 的前缀或某段内容，但不能改变调用方可见的位置时，
可以使用 `ReadSeekExt::peek_exact_or_eof`。

```rust
use qubit_io::ReadSeekExt;
use std::io::{Seek, SeekFrom};

fn peek_three(input: &mut std::io::Cursor<Vec<u8>>) -> std::io::Result<[u8; 3]> {
    input.seek(SeekFrom::Start(2))?;

    let mut buffer = [0; 3];
    let count = input.peek_exact_or_eof(&mut buffer)?;
    assert_eq!(3, count);
    assert_eq!(2, input.stream_position()?);
    Ok(buffer)
}

fn main() -> std::io::Result<()> {
    let mut cursor = std::io::Cursor::new(b"abcdef".to_vec());
    assert_eq!(peek_three(&mut cursor)?, *b"cde");
    Ok(())
}
```

### 读取、写入并定位

对于需要完整读写随机访问能力的内存缓冲区、文件或自定义句柄，使用
`ReadWriteSeek`。

```rust
use qubit_io::ReadWriteSeek;
use std::io::SeekFrom;

fn overwrite_prefix(io: &mut dyn ReadWriteSeek) -> std::io::Result<String> {
    io.write_all(b"hello")?;
    io.seek(SeekFrom::Start(0))?;
    io.write_all(b"j")?;
    io.seek(SeekFrom::Start(0))?;

    let mut content = String::new();
    io.read_to_string(&mut content)?;
    Ok(content)
}

fn main() -> std::io::Result<()> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    assert_eq!(overwrite_prefix(&mut cursor)?, "jello");
    Ok(())
}
```

### 读取并写入

当值是双向 stream、但不需要定位能力时，使用 `ReadWrite`。

```rust
use qubit_io::ReadWrite;

fn write_ping(stream: &mut dyn ReadWrite) -> std::io::Result<()> {
    stream.write_all(b"ping")
}

fn main() -> std::io::Result<()> {
    let mut buffer = std::io::Cursor::new(Vec::new());
    write_ping(&mut buffer)?;
    assert_eq!(buffer.into_inner(), b"ping");
    Ok(())
}
```

### 写入并定位

当输出需要在写入后回填先前位置时，使用 `WriteSeek`。典型例子是在序列化
payload 后回写 header 中的长度字段。

```rust
use qubit_io::WriteSeek;
use std::io::SeekFrom;

fn write_with_header(output: &mut dyn WriteSeek) -> std::io::Result<()> {
    output.write_all(&[0])?;
    output.write_all(b"payload")?;
    output.seek(SeekFrom::Start(0))?;
    output.write_all(&[7])?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    write_with_header(&mut cursor)?;
    assert_eq!(cursor.into_inner(), b"\x07payload");
    Ok(())
}
```

### 在指定 offset 写入

当需要回填 header、offset 表或长度字段，但不想打乱调用方当前写入位置时，可以使用
`WriteSeekExt::write_all_at_preserving_position`。

```rust
use qubit_io::WriteSeekExt;
use std::io::{Seek, Write};

fn patch_length(output: &mut std::io::Cursor<Vec<u8>>) -> std::io::Result<()> {
    output.write_all(&[0, 0])?;
    output.write_all(b"payload")?;
    let end = output.stream_position()?;

    output.write_all_at_preserving_position(0, &[0, 7])?;
    assert_eq!(end, output.stream_position()?);
    Ok(())
}

fn main() -> std::io::Result<()> {
    let mut cursor = std::io::Cursor::new(Vec::new());
    patch_length(&mut cursor)?;
    assert_eq!(cursor.into_inner(), b"\x00\x07payload");
    Ok(())
}
```

## 何时使用这些 Trait

适合使用 Qubit IO 组合 trait 的场景包括：

- API 需要在 trait object 中保存或传递不同的 I/O 对象。
- 希望参数类型保持简洁，例如 `&mut dyn ReadWriteSeek`。
- 需要 object safety，且不能依赖尚未稳定的 trait alias。
- 希望公开签名直接表达一种常用 I/O 能力组合。

如果调用方的具体类型可以继续保持泛型，优先使用普通泛型约束：

```rust
use std::io::{Read, Seek};

fn generic_reader<R>(reader: &mut R)
where
    R: Read + Seek,
{
    // 当调用方具体类型可以保持泛型时，使用这种写法。
}
```

## API 参考

| Trait | 标准库约束 | 典型用途 |
|------|------------|----------|
| `ReadSeek` | `Read + Seek` | 可读取的随机访问输入 |
| `BufReadSeek` | `BufRead + Seek` | 带缓冲的随机访问输入 |
| `ReadWrite` | `Read + Write` | 双向 stream 或缓冲区 |
| `WriteSeek` | `Write + Seek` | 可写入的随机访问输出 |
| `ReadWriteSeek` | `Read + Write + Seek` | 完整可变的随机访问 I/O |

| Extension trait | 方法 | 典型用途 |
|-----------------|------|----------|
| `ReadExt` | `read_exact_or_eof`、`discard_exact_or_eof`、`copy_to`、`copy_to_limited`、`read_to_end_limited` | 短读安全读取、有界复制和有界读取 |
| `SeekExt` | `stream_size` | 获取大小但保持原 cursor |
| `ReadSeekExt` | `peek_exact_or_eof`、`read_exact_or_eof_at` | 不消费位置的探测和随机 offset 读取 |
| `WriteSeekExt` | `write_all_at_preserving_position` | 随机访问 patch 写入 |
| `BinaryReadExt` | `read_u16_be`、`read_u16_le`、`read_u16(order)` 等标量方法 | 二进制标量解码 |
| `BinaryWriteExt` | `write_u16_be`、`write_u16_le`、`write_u16(value, order)` 等标量方法 | 二进制标量编码 |

每个 trait 都通过 blanket implementation 自动实现：

```rust
use std::io::{Read, Seek};

use qubit_io::ReadSeek;

fn accepts_read_seek<T>(value: &mut T) -> &mut dyn ReadSeek
where
    T: Read + Seek,
{
    value
}
```

## Object Safety 说明

Rust 的 trait alias 尚未稳定，而且类似 `dyn Read + Seek` 的多非 auto trait
组合不能按很多 API 需要的方式直接使用。Qubit IO 通过定义带有目标 supertrait
的具名 trait，并为所有满足约束的类型提供 blanket implementation 来解决这个问题。

组合 trait 自身不添加新方法。`read_exact`、`write_all`、`seek` 等方法都来自
标准库 supertrait。

## Extension Trait 说明

使用扩展方法前，需要把对应 trait import 到当前作用域：

```rust
use qubit_io::ReadExt;
```

extension trait 使用 blanket implementation，所以任何实现了对应标准库 trait 的类型
都会自动获得这些方法。这也适用于 `&mut dyn std::io::Read` 这类 trait object。

## 测试与代码覆盖率

本项目的测试聚焦于 trait object 支持、blanket implementation 行为、短读处理、
EOF 语义、interrupted I/O 重试以及位置恢复语义。

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行覆盖率报告
./coverage.sh

# 生成文本格式报告
./coverage.sh text

# 运行 CI 检查（格式化、clippy、测试、覆盖率）
./ci-check.sh
```

## 依赖项

本 crate 除 Rust 标准库外没有运行时依赖。

## 许可证

Copyright (c) 2026. Haixing Hu, Qubit Co. Ltd. All rights reserved.

根据 Apache 许可证 2.0 版（"许可证"）授权；
除非遵守许可证，否则您不得使用此文件。
您可以在以下位置获取许可证副本：

    http://www.apache.org/licenses/LICENSE-2.0

除非适用法律要求或书面同意，否则根据许可证分发的软件
按"原样"分发，不附带任何明示或暗示的担保或条件。
有关许可证下的特定语言管理权限和限制，请参阅许可证。

完整的许可证文本请参阅 [LICENSE](LICENSE)。

## 贡献

欢迎贡献！请随时提交 Pull Request。

### 开发指南

- 遵循 Rust API 指南。
- 保持工具的通用性，避免依赖领域 crate。
- 在文档能帮助理解时，为公共 API 提供示例。
- 提交 PR 前运行 `./ci-check.sh`。

## 作者

**胡海星** - *Qubit Co. Ltd.*

## 相关项目

Qubit 旗下的更多 Rust 库发布在 GitHub 组织
[qubit-ltd](https://github.com/qubit-ltd)。

---

仓库地址：[https://github.com/qubit-ltd/rs-io](https://github.com/qubit-ltd/rs-io)
