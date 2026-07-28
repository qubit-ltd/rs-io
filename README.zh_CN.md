# Qubit IO

[![Rust CI](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io/coverage-badge.json)](https://qubit-ltd.github.io/rs-io/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io.svg?color=blue)](https://crates.io/crates/qubit-io)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

Qubit IO 提供运行时中立的同步与异步 item stream，是 Qubit 文件系统、二进制
和文本 crate 共用的传输层。

这些 trait 有意止步于传输：只负责搬运 item 并报告 `std::io::Error`，不表达
文件身份、路径、commit、abort 或持久化语义。完整契约、缓冲区所有权、wrapper
组合方式与适配器说明请参阅 [用户指南](doc/user_guide.zh_CN.md)。

## 核心 API

| 关注点 | 同步 | 异步 |
| --- | --- | --- |
| 传输 | `Input<Item = T>`、`Output<Item = T>` | `AsyncInput<Item = T>`、`AsyncOutput<Item = T>` |
| 缓冲 | `BufferedInput`、`BufferedOutput` | `AsyncBufferedInput`、`AsyncBufferedOutput` |
| wrapper | 限量、计数、checksum、tee | 限量、计数、checksum |
| 生态桥接 | `qubit_io::std_io` 标准库集成 | 可选 Tokio 与 `futures-io` adapter |

## 同步示例

所有 `std::io::Read` 字节流都会实现 `Input<Item = u8>`，所有
`std::io::Write` 字节流都会实现 `Output<Item = u8>`。

```rust
use std::io::Cursor;
use qubit_io::{Input, Output};

let mut input = Cursor::new(b"qubit".to_vec());
let mut bytes = [0_u8; 5];
input.read_exactly(&mut bytes)?;

let mut output = Vec::new();
output.write_fully(&bytes)?;
assert_eq!(b"qubit", output.as_slice());
# Ok::<(), std::io::Error>(())
```

`Input` 与 `Output` 的 item 类型是泛型，因此 codec 也可以直接使用 `u16`、
`char` 或其他廉价标量，而不必先转换成字节。
标准库专属的组合 trait 从 `qubit_io::std_io` 导出，扩展 trait 位于
`qubit_io::std_io::ext`。

## Feature

```toml
[dependencies]
qubit-io = "0.14"
```

- 默认 feature：仅运行时中立的核心层。
- `tokio`：Qubit 与 Tokio I/O trait 的双向适配。
- `futures-io`：Qubit 与 `futures-io` trait 的双向适配。

## 文档

- [English user guide](doc/user_guide.md)
- [用户指南](doc/user_guide.zh_CN.md)
- [0.13 to 0.14 migration guide](doc/migration-0.14.md)
- [0.13 到 0.14 迁移指南](doc/migration-0.14.zh_CN.md)

docs.rs 上的 API 文档会启用项目声明的全部 feature 构建。

## 测试

```bash
# 使用默认 feature 集运行测试
cargo test

# 使用项目声明的全部 feature 运行测试
cargo test --all-features

# 运行项目 CI 检查
./ci-check.sh

# 检查代码覆盖率
./coverage.sh
```

## 许可证

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

本项目基于 Apache License 2.0 授权。完整许可证文本请参阅
[LICENSE](LICENSE)。

## 贡献

欢迎贡献。请遵循 Rust API 指南，及时更新公共 API 文档与测试，并在提交
Pull Request 前运行 `./align-ci.sh`格式化代码，运行`./ci-check.sh`对齐CI要求。

## 作者

**Haixing Hu** - *Qubit Co. Ltd.*

仓库地址：[https://github.com/qubit-ltd/rs-io](https://github.com/qubit-ltd/rs-io)
