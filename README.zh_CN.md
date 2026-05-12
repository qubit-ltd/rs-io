# Qubit IO

[![Rust CI](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml)
[![Coverage Status](https://coveralls.io/repos/github/qubit-ltd/rs-io/badge.svg?branch=main)](https://coveralls.io/github/qubit-ltd/rs-io?branch=main)
[![Crates.io](https://img.shields.io/crates/v/qubit-io.svg?color=blue)](https://crates.io/crates/qubit-io)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

面向 Rust 的小型 I/O trait 工具库。

## 概述

Qubit IO 为常用 `std::io` trait 组合提供 object-safe 的组合 trait。
当 API 需要使用 `&mut dyn ReadSeek` 或 `Box<dyn ReadWriteSeek>` 这类
trait object，而不是 `R: Read + Seek` 这类泛型约束时，可以使用本 crate。

本 crate 刻意保持极小：它不包装 reader 或 writer，不进行分配，也不引入新的
I/O 行为。它只为标准库 trait 的常用组合命名，使这些组合可以作为 trait object
出现在公开 API 或内部抽象中。

## 设计目标

- **object-safe 组合**：提供适合 trait object 使用的具名 I/O 约束。
- **标准库优先**：直接基于 `std::io::{Read, Write, Seek}` 构建。
- **零运行时开销**：使用 blanket implementation，不引入包装类型。
- **极小 API 表面积**：只保留常用、可复用的 I/O trait 组合。
- **便于集成**：可用于 cursor、文件、缓冲区、stream 和自定义 I/O 类型。

## 特性

### Object-Safe I/O Trait 组合

- **`ReadSeek`**：组合 `Read` 与 `Seek`，用于可读取的随机访问输入。
- **`ReadWrite`**：组合 `Read` 与 `Write`，用于双向 stream 或缓冲区。
- **`WriteSeek`**：组合 `Write` 与 `Seek`，用于可写入的随机访问输出。
- **`ReadWriteSeek`**：组合 `Read`、`Write` 与 `Seek`，用于完整可变的随机访问 I/O 对象。

### Blanket Implementation

所有实现了对应标准库 trait 的类型，都会自动实现 Qubit IO 的组合 trait。
对 `std::io::Cursor`、`std::fs::File` 或自定义 I/O 类型，都不需要额外编写
adapter 代码。

## 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
qubit-io = "0.1.0"
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
| `ReadWrite` | `Read + Write` | 双向 stream 或缓冲区 |
| `WriteSeek` | `Write + Seek` | 可写入的随机访问输出 |
| `ReadWriteSeek` | `Read + Write + Seek` | 完整可变的随机访问 I/O |

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

这些 trait 自身不添加新方法。`read_exact`、`write_all`、`seek` 等方法都来自
标准库 supertrait。

## 测试与代码覆盖率

本项目的测试聚焦于 trait object 支持和 blanket implementation 行为。

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
