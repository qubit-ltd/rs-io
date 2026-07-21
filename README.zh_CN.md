# Qubit IO

[![Rust CI](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/qubit-io.svg?color=blue)](https://crates.io/crates/qubit-io)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

Qubit IO 提供运行时中立的同步与异步 item stream，是 Qubit 文件系统、二进制
和文本 crate 共用的传输层。

这些 trait 只负责搬运 item 并报告 `std::io::Error`；它们不暗示文件身份、
路径、commit、abort 或持久化语义。文件语义由 `qubit-fs` 在更高层表达。

## 核心 API

| 关注点 | 同步 | 异步 |
| --- | --- | --- |
| 输入 | `Input<Item = T>` | `AsyncInput<Item = T>` |
| 输出 | `Output<Item = T>` | `AsyncOutput<Item = T>` |
| 便利操作 | `read_fully`、`write_fully` | `AsyncInput`、`AsyncOutput` 的默认方法 |
| 缓冲 | `BufferedInput`、`BufferedOutput` | `AsyncBufferedInput`、`AsyncBufferedOutput` |
| 限量 | std stream 的 `LimitReader`、`LimitWriter` | `AsyncLimitInput`、`AsyncLimitOutput` |
| 计数 | `CountingReader`、`CountingWriter` | `AsyncCountingInput`、`AsyncCountingOutput` |
| 校验 | `ChecksumReader`、`ChecksumWriter` | `AsyncChecksumInput`、`AsyncChecksumOutput` |

`AsyncInput` 和 `AsyncOutput` 只使用 `Pin`、`Context` 与 `Poll`，不依赖
Tokio、`futures-io` 或任何 executor。跨多次 `Pending` 的操作由
`ReadFullyFuture`、`WriteFullyFuture` 等具名 Future 保存进度。

## 同步示例

所有 `std::io::Read` 字节流都会实现 `Input<Item = u8>`，所有
`std::io::Write` 字节流都会实现 `Output<Item = u8>`。

```rust
use std::io::Cursor;
use qubit_io::{Input, Output};

let mut input = Cursor::new(b"qubit".to_vec());
let mut bytes = [0_u8; 5];
assert_eq!(5, input.read_fully(&mut bytes)?);

let mut output = Vec::new();
output.write_fully(&bytes)?;
assert_eq!(b"qubit", output.as_slice());
# Ok::<(), std::io::Error>(())
```

`Input` 与 `Output` 的 item 类型是泛型，因此 codec 也可以直接使用 `u16`、
`char` 或其他廉价标量，而不必先转换成字节。

## 异步示例

Tokio 适配通过显式 newtype 完成，从而避免一个类型同时实现多套异步生态 trait
时产生 coherence 冲突。

```rust,ignore
use qubit_io::{AsyncInput, TokioInput};

let socket = /* 某个 tokio::io::AsyncRead */;
let mut input = TokioInput::new(socket);
let mut header = [0_u8; 16];
let read = input.read_fully_async(&mut header).await?;
```

反向适配器同样可用：`TokioAsyncRead`、`TokioAsyncWrite` 把 Qubit stream
暴露给 Tokio；`FuturesAsyncRead`、`FuturesAsyncWrite` 则暴露给
`futures-io`。

## 缓冲与组合

`Buffer<T>` 是底层 readable-window 容器，同步和异步缓冲器共用相同的
position/limit 模型。

`AsyncBufferedOutput` 会一直持有已接受但尚未被底层接受的 item。部分 flush
在返回 `Pending` 前会先记录已完成进度。异步缓冲器在 `Drop` 中无法执行 I/O；
需要保证送达时必须调用 `flush_async()`，或用 `into_parts()` 取回 pending 数据。

limit 和 counting wrapper 面向任意 item；checksum wrapper 只面向字节，因为
`std::hash::Hasher` 的输入是字节。

## Feature

```toml
[dependencies]
qubit-io = "0.14"
```

- 默认 feature：仅运行时中立的核心层。
- `tokio`：Qubit 与 Tokio I/O trait 的双向适配。
- `futures-io`：Qubit 与 `futures-io` trait 的双向适配。

## 文档与检查

- [User guide](doc/user_guide.md)
- [用户指南](doc/user_guide.zh_CN.md)

```bash
cargo test --no-default-features
cargo test --all-features
./align-ci.sh
./ci-check.sh
```

## 许可证

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

本项目基于 Apache License 2.0 授权，详见 [LICENSE](LICENSE)。
