# Qubit IO

[![Rust CI](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io/coverage-badge.json)](https://qubit-ltd.github.io/rs-io/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io.svg?color=blue)](https://crates.io/crates/qubit-io)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

Qubit IO 让 codec、协议和存储库能够公开 I/O 能力而不替调用方选择异步运行时。调用方可在 codec 外部组合缓冲、限量、观测和校验策略，而不是把这些策略嵌入每一种传输实现。

它是传输层，不是文件系统抽象：它不表达路径、文件身份、commit、abort 或持久化。当契约包含这些生命周期语义时，应使用 `qubit-fs` 等更高层 crate。

## 为什么需要这层抽象

单一应用只使用一个运行时时，原生 I/O trait 往往最简单。库的公开边界不同：若公开 API 选择 Tokio，就排除了 `futures-io` 用户；若只接受字节流，文本和数据处理管道就必须先把逻辑 item 编码成字节，算法才能处理它们。

Qubit IO 将这个边界保持得很小：

- `Input` 与 `Output` 传输同步 item。
- `AsyncInput` 与 `AsyncOutput` 传输异步 item，但不选择 executor。
- buffer 和 wrapper 每次只增加一项传输策略。
- 显式 adapter 将边界连接到 `std::io`、Tokio 和 `futures-io`。

它不是要替代原生 I/O，而是在 transport、运行时或 item 类型属于调用方时，为库提供稳定的公开边界。

## 一个异步 API，运行时由调用方选择

设想一个协议库读取带四字节大端长度头的 frame。协议最多接受 64 KiB 的 frame，且必须在分配 payload 前拒绝过大的长度头。

```rust
use std::io::{self, Error, ErrorKind};
use qubit_io::AsyncInput;

const MAX_FRAME_LEN: usize = 64 * 1024;

async fn read_frame_async<I>(input: &mut I) -> io::Result<Vec<u8>>
where
    I: AsyncInput<Item = u8> + Unpin,
{
    let mut header = [0_u8; 4];
    input.read_exactly_async(&mut header).await?;
    let length = u32::from_be_bytes(header) as usize;
    if length > MAX_FRAME_LEN {
        return Err(Error::new(ErrorKind::InvalidData, "frame is too large"));
    }

    let mut payload = vec![0_u8; length];
    input.read_exactly_async(&mut payload).await?;
    Ok(payload)
}
```

协议库只写一次这个异步函数。Tokio 应用传入 `TokioInput`，`futures-io` 应用传入 `FuturesInput`：

```rust,ignore
use qubit_io::{
    AsyncBufferedInput, AsyncCountingInput, AsyncLimitInput, FuturesInput,
    TokioInput,
};

const CONNECTION_BUDGET: u64 = 1_048_576;

// Tokio 调用方：`stream` 实现 `tokio::io::AsyncRead`。
let mut tokio_input = AsyncCountingInput::new(AsyncBufferedInput::with_capacity(
    AsyncLimitInput::new(TokioInput::new(stream), CONNECTION_BUDGET),
    8 * 1024,
));
let frame = read_frame_async(&mut tokio_input).await?;
assert_eq!(frame.len() as u64 + 4, tokio_input.bytes_read());

// futures-io 调用方：`reader` 实现 `futures_io::AsyncRead`。
let mut futures_input = FuturesInput::new(reader);
let frame = read_frame_async(&mut futures_input).await?;
# Ok::<(), std::io::Error>(())
```

同步版本使用对应的 `Input<Item = u8>` 边界。它是独立的阻塞驱动，因为 Rust 的同步与异步控制流不同；协议常量和校验规则保持一致。

## 策略留在 codec 外部

上面的 frame decoder 只知道 wire format，调用方自行选择传输策略：

```text
transport adapter -> limit -> buffer -> counting -> frame decoder
```

`AsyncLimitInput` 限制连接总共可暴露的字节数，`AsyncBufferedInput` 批量读取 transport，`AsyncCountingInput` 报告 decoder 实际消费的字节数。协议自己的 64 KiB 检查仍不可省略：transport budget 不是格式校验规则。

同步侧使用相同的策略词汇：limit、counting、checksum、tee 和 buffer。wrapper 顺序具有语义。例如，counter 位于 buffer 外侧时统计 decoder 的消费量；位于 buffer 内侧时统计 transport 的供给量，后者包括预取字节。[用户指南](doc/user_guide.zh_CN.md)会展开说明这些选择及其恢复语义。

## 不只传输字节，也传输类型化 item

`Item` 是泛型。数据处理 operator 可以消费业务记录并输出映射记录，无需在每一个 operator 边界重复序列化：

```rust,ignore
use std::io;
use qubit_io::{Input, Output};

fn map_partition<I, O>(input: &mut I, output: &mut O) -> io::Result<()>
where
    I: Input<Item = Sale>,
    O: Output<Item = CategoryRevenue>,
{
    // 读取 `Sale` 记录，转换后写出 `CategoryRevenue` 记录。
    # Ok(())
}
```

在该管道中，limit 与 counting 的单位是“记录”而不是字节。`TeeOutput` 可以把同一批类型化记录同时写入 shuffle sink 和 audit sink。用户指南提供完整可运行的 Map/Reduce 案例，并解释通用 buffer 额外要求 item 满足 `Copy + Default`。

## 何时使用 Qubit IO

| 场景 | 建议 |
| --- | --- |
| 库必须同时支持 Tokio 和 `futures-io` 调用方 | 公开 Qubit async trait，让调用方使用 adapter。 |
| 传输策略必须独立于 codec 组合 | 使用 Qubit buffer 与 wrapper。 |
| stream 承载 `char`、固定布局记录或其他逻辑 item | 使用泛型 `Input` 与 `Output`。 |
| 单一应用、单一运行时、普通字节流 | 优先使用原生 I/O trait。 |
| 契约包含路径、文件身份、commit 或持久化 | 使用更高层的文件系统抽象。 |

## API 地图

| 关注点 | 同步 | 异步 |
| --- | --- | --- |
| 传输 | `Input<Item = T>`、`Output<Item = T>` | `AsyncInput<Item = T>`、`AsyncOutput<Item = T>` |
| 缓冲 | `BufferedInput`、`BufferedOutput` | `AsyncBufferedInput`、`AsyncBufferedOutput` |
| 观测与限量 | limit、counting、checksum、tee | 异步 limit、counting、checksum |
| 生态桥接 | `qubit_io::std_io` | 可选 Tokio 与 `futures-io` adapter |

`AsyncClose` 表示真实的 transport shutdown，并且有意与刷新缓冲输出的 flush 分离。

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

本项目基于 Apache License 2.0 授权。完整许可证文本请参阅 [LICENSE](LICENSE)。

## 贡献

欢迎贡献。请遵循 Rust API 指南，及时更新公共 API 文档与测试，并在提交 Pull Request 前运行 `./align-ci.sh`格式化代码，运行`./ci-check.sh`对齐CI要求。

## 作者

**Haixing Hu** - *Qubit Co. Ltd.*

仓库地址：[https://github.com/qubit-ltd/rs-io](https://github.com/qubit-ltd/rs-io)
