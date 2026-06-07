# Qubit IO

[![Rust CI](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io/coverage-badge.json)](https://qubit-ltd.github.io/rs-io/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io.svg?color=blue)](https://crates.io/crates/qubit-io)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

面向 Rust 的字节流缓冲与轻量 `std::io` trait 工具库。

## 概述

`qubit-io` 提供：

- 面向字节的缓冲原语：`Buffer`、`BufferedByteInput` 和
  `BufferedByteOutput`；
- 可作为 trait object 使用的组合 trait，例如 `ReadSeek`、`ReadWrite` 和
  `ReadWriteSeek`；
- `Read`、`BufRead`、`Seek`、`Read + Seek`、`Write` 和 `Write + Seek` 的常用
  extension trait；
- 用于复制和内容比较的 `Streams` 工具函数；
- `CountingReader`、`LimitReader`、`PositionGuard`、`TeeReader` 和 checksum
  wrapper 等轻量 reader / writer wrapper。

binary scalar、LEB128 和 ZigZag 能力已经不在本 crate 中。缓冲区级二进制 codec
请使用 `qubit-codec-binary`，二进制 stream reader、writer 和 extension trait 请使用
`qubit-io-binary`。

详细用法请参见[中文用户指南](doc/user_guide.zh_CN.md)。API 参考文档可在
[docs.rs](https://docs.rs/qubit-io) 查看。

## 设计目标

- **只做通用 I/O**：保持本 crate 聚焦可复用的 `std::io` helper。
- **字节级缓冲**：提供高效 byte buffer，但不嵌入 binary codec、text codec
  或 record format 逻辑。
- **低层契约显式化**：`Buffer` 与 unchecked range helper 面向 hot path，调用方
  责任必须清楚。
- **组合 Trait 可对象化**：让常见 trait 组合易于命名和传递。
- **Extension Trait 行为可预测**：提供常见 read、write、seek 和 copy 模式，同时不隐藏分配和错误行为。
- **分层隔离**：binary 和 text codec 的 stream adapter 放在相邻 crate 中。
- **依赖图小**：不引入运行时依赖也能提供实用 I/O 工具。

## 特性

### Buffered Byte I/O

- **`Buffer<T>`**：低层 position/limit 存储，维护 readable window 与 spare
  tail capacity。
- **`BufferedByteInput`**：包装 `Read` 的缓冲字节输入，支持查看 unread window、
  按数量 refill、逻辑 seek，以及针对已验证输出 range 的 indexed unchecked read。
- **`BufferedByteOutput`**：包装 `Write` 的缓冲字节输出，支持 spare window 访问、
  checked / unchecked advance、flush、seek，以及大块写入绕过缓冲区。
- **`DEFAULT_BUFFER_CAPACITY`**：byte input 与 byte output 共用的默认缓冲容量。

### 组合 Trait

- **`ReadSeek`**：命名 `Read + Seek`。
- **`BufReadSeek`**：命名 `BufRead + Seek`。
- **`ReadWrite`**：命名 `Read + Write`。
- **`ReadWriteSeek`**：命名 `Read + Write + Seek`。
- **`WriteSeek`**：命名 `Write + Seek`。

### Extension Trait

- **`ReadExt`**：exact read、partial EOF read、limited read 和 copy helper。
- **`BufReadExt`**：带上限的按行和按分隔符读取。
- **`SeekExt`**：保持位置不变的 stream size helper。
- **`ReadSeekExt`**：恢复位置的 peek/read-at helper。
- **`WriteExt`**：针对已验证 range 的 unchecked write helper。
- **`WriteSeekExt`**：保持位置不变的 write-at helper。

### 工具函数与 Wrapper

- **`Streams`**：复制、有界复制、相等判断和字典序比较。
- **计数 wrapper**：`CountingReader` 与 `CountingWriter`。
- **限量 wrapper**：`LimitReader` 与 `LimitWriter`。
- **Tee wrapper**：`TeeReader` 与 `TeeWriter`。
- **Checksum wrapper**：`ChecksumReader` 与 `ChecksumWriter`。
- **位置保护**：`PositionGuard` 在 drop 时恢复 stream 位置，除非显式 dismiss。

## 文档

- [中文用户指南](doc/user_guide.zh_CN.md)
- [API 文档](https://docs.rs/qubit-io)
- [英文 README](README.md)

## 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
qubit-io = "0.6"
```

## 快速开始

```rust
use std::io::Cursor;

use qubit_io::{
    BufferedByteInput,
    BufferedByteOutput,
    ReadExt,
    Streams,
};

let mut input = Cursor::new(b"abcdef".to_vec());
let mut prefix = [0_u8; 3];

let read = input.read_exact_or_eof(&mut prefix)?;
assert_eq!(3, read);
assert_eq!(b"abc", &prefix);

let mut source = Cursor::new(b"payload".to_vec());
let mut output = Vec::new();
let copied = Streams::copy_at_most(&mut source, &mut output, 4)?;

assert_eq!(4, copied);
assert_eq!(b"payl", output.as_slice());

let mut buffered_input = BufferedByteInput::with_capacity(
    Cursor::new(b"abcdef".to_vec()),
    3,
);
buffered_input.ensure_available(3)?;
assert_eq!(b"abc", buffered_input.unread_slice());
unsafe {
    buffered_input.consume_unchecked(3);
}

let mut buffered_output =
    BufferedByteOutput::with_capacity(Cursor::new(Vec::<u8>::new()), 4);
buffered_output.ensure_spare_capacity(3)?;
buffered_output.spare_buffer_mut()[0..3].copy_from_slice(b"xyz");
unsafe {
    buffered_output.advance_unchecked(3);
}
let cursor = buffered_output.into_inner()?;
assert_eq!(b"xyz", cursor.into_inner().as_slice());
# Ok::<(), std::io::Error>(())
```

## API 参考

### Trait Alias

| Trait | 等价约束 |
|-------|----------|
| `ReadSeek` | `Read + Seek` |
| `BufReadSeek` | `BufRead + Seek` |
| `ReadWrite` | `Read + Write` |
| `ReadWriteSeek` | `Read + Write + Seek` |
| `WriteSeek` | `Write + Seek` |

### 工具类型

| 类型 | 用途 |
|------|------|
| `Buffer` | 面向 hot path buffering 的低层 position/limit 存储 |
| `BufferedByteInput` | 包装 `Read` 的缓冲字节输入 |
| `BufferedByteOutput` | 包装 `Write` 的缓冲字节输出 |
| `Streams` | 用于复制和比较 stream 的静态 helper |
| `CountingReader` / `CountingWriter` | 统计成功读取或写入的字节数 |
| `LimitReader` / `LimitWriter` | 限制 wrapper 可读取或写入的字节数 |
| `TeeReader` / `TeeWriter` | 把字节镜像到第二个 sink |
| `ChecksumReader` / `ChecksumWriter` | 把成功通过的字节送入调用方提供的 hasher |
| `PositionGuard` | 除非显式 dismiss，否则恢复 seek 位置 |

### 常量

| 常量 | 用途 |
|------|------|
| `DEFAULT_BUFFER_CAPACITY` | byte input 与 byte output 共用的默认缓冲容量 |

## Crate 拆分

当前 codec 与 stream 分层如下：

- `qubit-codec`：核心 byte order、codec、transcoder、encoder 和 decoder trait；
- `qubit-codec-binary`：缓冲区级 binary、LEB128 和 ZigZag codec；
- `qubit-io`：通用 `std::io` helper；
- `qubit-io-binary`：二进制 stream reader、writer 和 extension trait；
- `qubit-codec-text` 和 `qubit-io-text`：文本 codec 与文本 stream adapter。

## 性能考虑

大多数 helper 直接操作调用方提供的缓冲区，并委托到底层 `Read`、`Write`
或 `Seek` 实现。Wrapper 类型不做隐藏分配；是否缓冲以及如何缓冲由调用点显式决定。

`Buffer<T>` 与 indexed unchecked read/write helper 是低层 API，面向已经完成
range 校验的调用方。它们主要用于 binary/text stream adapter 这类 hot path，在这些
场景中避免重复 slicing 和 bounds check 有明确价值。通用调用仍应优先使用安全方法。

## 测试与代码覆盖率

本项目通过 `tests/` 下的集成测试覆盖通用 I/O 行为。

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行覆盖率报告
./coverage.sh

# 生成文本格式报告
./coverage.sh text

# 对齐 CI 要求
./align-ci.sh

# 运行 CI 检查（格式化、clippy、测试、覆盖率、安全审计）
RS_CI_SKIP_TOOLCHAIN_UPDATE=1 ./ci-check.sh
```

## 依赖项

`qubit-io` 没有运行时依赖。

## 许可证

Copyright (c) 2026. Haixing Hu.

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

- 保持本 crate 通用，不依赖具体 codec 格式。
- 为 I/O 边界场景保持稳定且确定的测试。
- 为公开 API 和错误行为编写文档。
- 提交 PR 前确保所有检查通过。

## 作者

**胡海星**

## 相关项目

Qubit 旗下的更多 Rust 库发布在 GitHub 组织 [qubit-ltd](https://github.com/qubit-ltd)。

---

仓库地址：[https://github.com/qubit-ltd/rs-io](https://github.com/qubit-ltd/rs-io)
