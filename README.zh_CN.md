# Qubit IO

[![Rust CI](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io/coverage-badge.json)](https://qubit-ltd.github.io/rs-io/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io.svg?color=blue)](https://crates.io/crates/qubit-io)
[![License](https://img.shields.io/crates/l/qubit-io.svg)](LICENSE)

面向 Rust `std::io` 的轻量 trait 工具库。

`qubit-io` 提供：

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

## 安装

在 `Cargo.toml` 中添加：

```toml
[dependencies]
qubit-io = "0.6"
```

## 快速示例

```rust
use std::io::Cursor;

use qubit_io::{
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
# Ok::<(), std::io::Error>(())
```

## Crate 拆分

当前 codec 与 stream 分层如下：

- `qubit-codec`：核心 byte order、codec、coder、encoder 和 decoder trait；
- `qubit-codec-binary`：缓冲区级 binary、LEB128 和 ZigZag codec；
- `qubit-io`：通用 `std::io` helper；
- `qubit-io-binary`：二进制 stream reader、writer 和 extension trait；
- `qubit-codec-text` 和 `qubit-io-text`：文本 codec 与文本 stream adapter。

仓库地址：[https://github.com/qubit-ltd/rs-io](https://github.com/qubit-ltd/rs-io)
