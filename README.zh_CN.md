# Qubit IO

[![Rust CI](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io/coverage-badge.json)](https://qubit-ltd.github.io/rs-io/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io.svg?color=blue)](https://crates.io/crates/qubit-io)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

面向 Rust 的小型 stream I/O trait 与扩展工具库。

## 概述

Qubit IO 在 `std::io` 之上提供一组紧凑的底层能力：

- 为常用 `std::io` 能力组合提供 object-safe 的组合 trait；
- 为标准库留给调用方反复手写的底层 I/O 模式提供 extension trait；
- 提供 `Streams` 命名空间，用于 stream 层复制和比较；
- 为 stream 统计、限制、tee、checksum 和位置恢复提供 wrapper 类型；
- 提供 binary、LEB128、ZigZag 和 length-prefixed UTF-8 编解码能力。

本地文件系统 helper，例如 `Files`、`Filenames`、`TempFile` 和 `TempDir`，已经移动到
`qubit-local-fs`。

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

# Ok::<(), std::io::Error>(())
```

## 主要 API

| API | 用途 |
| --- | --- |
| `ReadSeek`、`BufReadSeek`、`ReadWrite`、`WriteSeek`、`ReadWriteSeek` | 常用 `std::io` 能力集合的 object-safe 组合 trait |
| `ReadExt`、`BufReadExt`、`SeekExt`、`ReadSeekExt`、`WriteSeekExt` | 精确、有界、分隔符导向和位置保持 I/O 的 extension trait |
| `BinaryReadExt`、`BinaryWriteExt` | 基础数字标量编解码 |
| `Leb128ReadExt`、`Leb128WriteExt` | unsigned / signed LEB128 编解码 |
| `ZigZagReadExt`、`ZigZagWriteExt` | ZigZag 映射有符号整数编解码 |
| `StringReadExt`、`StringWriteExt` | length-prefixed UTF-8 字符串编解码 |
| `Streams` | stream 复制、有界复制、EOF 限制复制和内容比较 |
| `CountingReader`、`CountingWriter` | 字节计数 wrapper |
| `LimitReader`、`LimitWriter` | 字节限制 wrapper |
| `TeeReader`、`TeeWriter` | 把读取或写入复制到 branch writer |
| `ChecksumReader`、`ChecksumWriter` | 在读写时更新调用方提供的 checksum 状态 |
| `PositionGuard` | 除非主动 dismiss，否则恢复 seek 位置 |

## Crate 边界

`qubit-io` 只保留 stream 和字节 I/O 能力，不再暴露本地文件系统 helper。本地路径工具、
临时文件和目录、目录复制、目录清理、atomic file write 请使用 `qubit-local-fs`。

## 运行时依赖

本 crate 运行时只依赖 Rust 标准库。
