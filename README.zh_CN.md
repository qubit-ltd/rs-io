# Qubit IO

[![Rust CI](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io/coverage-badge.json)](https://qubit-ltd.github.io/rs-io/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io.svg?color=blue)](https://crates.io/crates/qubit-io)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

面向 Rust 的小型 stream I/O trait 与扩展工具库。

## 概述

Qubit IO 专注于基于 `std::io` 的 stream 和字节 I/O。它不尝试成为文件系统抽象层，而是为读、写、seek、buffer、编码、解码、限制、tee、计数和比较字节流的代码提供可复用构件。

适合使用本 crate 的场景包括：

- 需要常用 `std::io` 能力组合的 object-safe trait；
- 需要精确读取、有界读取、有界分隔符读取和保持位置不变的 seek 操作；
- 需要有界复制、EOF 限制复制、内容比较等 stream helper；
- 需要 binary、LEB128、ZigZag 和 length-prefixed UTF-8 编解码 helper；
- 需要计数、限制、tee、checksum、seek 位置恢复等 stream wrapper。

详细用法、示例和 API 选择建议请参见[中文用户手册](doc/user_guide.zh_CN.md)。API 参考文档可在 [docs.rs](https://docs.rs/qubit-io) 查看。

如果需要本地文件系统相关能力，请参考[qubit-local-files](https://github.com/qubit-ltd/rs-local-files)。

## 安装

```toml
[dependencies]
qubit-io = "0.2"
```

## 快速示例

```rust
use std::io::Cursor;

use qubit_io::{
    ReadExt,
    Streams,
    WriteSeekExt,
};

let mut input = Cursor::new(b"hello".to_vec());
let mut output = Vec::new();

Streams::copy(&mut input, &mut output)?;
assert_eq!(b"hello", output.as_slice());

let mut cursor = Cursor::new(vec![0; 8]);
cursor.write_all_at_preserving_position(2, b"rs")?;

let data = Cursor::new(b"bounded".to_vec()).read_to_end_limited(16)?;
assert_eq!(b"bounded", data.as_slice());

# Ok::<(), std::io::Error>(())
```

## 主要能力

### Object-Safe I/O 组合 Trait

`qubit-io` 提供可作为 trait object 使用的具名组合 trait：

| Trait | 组合能力 | 典型用途 |
| --- | --- | --- |
| `ReadSeek` | `Read + Seek` | 可读取的随机访问输入 |
| `BufReadSeek` | `BufRead + Seek` | 带缓冲的随机访问输入 |
| `ReadWrite` | `Read + Write` | 双向 stream 或可变 buffer |
| `WriteSeek` | `Write + Seek` | 可写入的随机访问输出 |
| `ReadWriteSeek` | `Read + Write + Seek` | 完整可变的随机访问 I/O 对象 |

当 API 希望接收 `&mut dyn ReadSeek`，而不是写成 `R: Read + Seek` 泛型约束时，这些 trait 会更方便。

### Extension Trait

extension trait 让常见底层 I/O 模式保持接近标准库，同时减少重复样板代码：

| Extension trait | 示例 |
| --- | --- |
| `ReadExt` | `read_exact_or_eof`、`discard_exact_or_eof`、`copy_to`、`read_to_end_limited`、`read_to_string_limited` |
| `BufReadExt` | `read_until_limited`、`read_line_limited`、`discard_until_limited` |
| `SeekExt` | `stream_size` |
| `ReadSeekExt` | `peek_exact_or_eof`、`read_exact_or_eof_at` |
| `WriteSeekExt` | `write_all_at_preserving_position` |
| `BinaryReadExt` / `BinaryWriteExt` | fixed-width 整数和浮点标量编解码 |
| `Leb128ReadExt` / `Leb128WriteExt` | unsigned / signed LEB128 整数编解码 |
| `ZigZagReadExt` / `ZigZagWriteExt` | ZigZag 映射有符号整数编解码 |
| `StringReadExt` / `StringWriteExt` | length-prefixed UTF-8 字符串 |

### Streams 命名空间

`Streams` 包含 stream 层 associated function：

| 方法 | 用途 |
| --- | --- |
| `copy` | 委托给 `std::io::copy` |
| `copy_at_most` | 最多复制指定字节数 |
| `copy_to_end_limited` | 只有在大小限制内到达 EOF 才完成复制 |
| `content_eq` | 比较两个 readable stream 的字节内容是否相同 |
| `compare_content` | 对两个 readable stream 做字典序比较 |

### Wrapper

wrapper 类型把 stream 行为放进类型语义，而不是一次性函数调用：

| Wrapper | 用途 |
| --- | --- |
| `CountingReader`、`CountingWriter` | 统计成功读写的字节数 |
| `LimitReader`、`LimitWriter` | 限制读取或写入的字节预算 |
| `TeeReader`、`TeeWriter` | 把数据复制到 branch writer |
| `ChecksumReader`、`ChecksumWriter` | 读写时更新调用方提供的 checksum 状态 |
| `PositionGuard` | drop 时恢复 seek 位置，除非主动 dismiss |

### Codec Wrapper

偏好 reader/writer object 调用风格的用户可以使用 root-level codec wrapper：

| Wrapper | 用途 |
| --- | --- |
| `BinaryReader`、`BinaryWriter` | fixed-width 标量编解码 |
| `Leb128Reader`、`Leb128Writer` | LEB128 编解码 |
| `ZigZagReader`、`ZigZagWriter` | 基于 unsigned LEB128 payload 的 ZigZag 编解码 |

## Prelude

`qubit_io::prelude` 重导出 method-providing extension trait 和 object-safe 组合 trait。它有意不重导出 wrapper 类型或具体命名空间。

```rust
use qubit_io::prelude::*;
```

## Crate 边界

`qubit-io` 只保留 stream 和字节 I/O 能力，不暴露本地路径 helper、临时文件、目录复制、目录清理或 atomic file write。如果需要本地文件系统相关能力，请参考[qubit-local-files](https://github.com/qubit-ltd/rs-local-files)。

## 运行时依赖

本 crate 运行时只依赖 Rust 标准库。

## 测试与代码覆盖率

本项目为 stream trait、extension method、codec helper 和 wrapper 类型保持测试覆盖。

### 运行测试

```bash
# 运行所有测试
cargo test

# 运行覆盖率报告
./coverage.sh

# 生成文本格式报告
./coverage.sh text

# 运行 CI 检查（格式化、clippy、测试、覆盖率、audit）
./ci-check.sh
```

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

欢迎贡献。请随时提交 Pull Request。

### 开发指南

- 遵循 Rust API 指南。
- 将 stream 和字节 I/O 相关能力保留在 `qubit-io` 中。
- 本地文件系统工具请使用 [qubit-local-files](https://github.com/qubit-ltd/rs-local-files)。
- 保持全面的测试覆盖。
- 公共 API 在有助于说明行为时应提供文档和示例。
- 提交 PR 前确保 `./ci-check.sh` 通过。

## 作者

**Haixing Hu**

## 相关项目

- [qubit-local-files](https://github.com/qubit-ltd/rs-local-files)：面向 Rust 的本地文件系统工具库。
- Qubit 旗下的更多 Rust 库发布在 GitHub 组织 [qubit-ltd](https://github.com/qubit-ltd)。

---

仓库地址：[https://github.com/qubit-ltd/rs-io](https://github.com/qubit-ltd/rs-io)
