# Qubit IO 用户指南

## 1. 边界与目标

`qubit-io` 是一个小型 item 传输抽象。它让上层可以组合缓冲与 codec，而不必
把公开 API 固定到 `std::io`、Tokio 或 `futures-io`。

该抽象有意停留在传输层：

- `Input` 与 `AsyncInput` 产生 item；
- `Output` 与 `AsyncOutput` 接受 item，并可 flush 传输缓冲；
- 传输错误统一使用 `std::io::Error`；
- 文件路径、metadata、publication、commit 和 abort 不属于这一层。

## 2. 同步 trait

`Input` 与 `Output` 使用关联类型 `Item`。unchecked indexed 方法是实现边界，
safe 方法负责检查范围和实现返回的数量。

```rust
use qubit_io::{Input, Output};

fn copy_once<I, O>(input: &mut I, output: &mut O) -> std::io::Result<usize>
where
    I: Input<Item = u8>,
    O: Output<Item = u8>,
{
    let mut buffer = [0_u8; 1024];
    let read = input.read(&mut buffer)?;
    output.write_fully(&buffer[..read])?;
    Ok(read)
}
```

标准库 `Read` 与 `Write` 字节流通过 blanket impl 自动适配。这并不表示任意
`Input<u8>` 都是文件，它只表示该对象是字节来源。

## 3. 异步 trait

异步核心不依赖 executor：

```text
AsyncInput::poll_read_unchecked
AsyncOutput::poll_write_unchecked
AsyncOutput::poll_flush
```

底层 poll trait 支持 `!Unpin` 实现。便利扩展方法要求 `Unpin` 并返回具名
Future：

- `read_async`：执行一次读取；
- `read_fully_async`：填满目标，或在 EOF 停止；
- `write_async`：执行一次写入；
- `write_fully_async`：接受全部来源，否则报告 `WriteZero`；
- `flush_async`：flush 输出。

多次操作的进度保存在 Future 对象中。poll 实现不得在返回 `Poll::Pending` 的
同时暗中报告调用方无法观察的传输进度。

## 4. 缓冲

`Buffer<T>` 持有已初始化的标量存储，并跟踪 `position..limit` readable window。

同步缓冲：

- `BufferedInput<I>` 保留预读 item，并通过 `unread()` 暴露它们；
- `BufferedOutput<O>` 聚合小写入并向 `O` flush；
- `EnsuredBufferedInput` 与 `EnsuredBufferedOutput` 避免重复套缓冲。

异步缓冲：

- `AsyncBufferedInput<I>` 跨 `Pending` 保留已预读 item；
- `AsyncBufferedOutput<O>` 保留已接受 item 和部分 flush 进度。

异步 `Drop` 不能 await，因此 `AsyncBufferedOutput` 不会伪装 drop-time 送达成功。
丢弃前应完成 `flush_async()`，或者用 `into_parts()` 取回内部 output 和 pending
`Buffer`。

## 5. 限量、计数与 checksum

异步 wrapper 直接实现 poll trait，并允许内部 stream 为 `!Unpin`：

- `AsyncLimitInput` / `AsyncLimitOutput` 最多放行指定数量的 item；
- `AsyncCountingInput` / `AsyncCountingOutput` 只统计成功的 ready 结果；
- `AsyncChecksumInput` / `AsyncChecksumOutput` 对成功传输的字节计算 hash。

`Pending` 和错误都不会改变计数或 hash。checksum wrapper 只处理底层实际报告
成功的前缀。

当 API 明确要求 `Read`、`Write`、`BufRead` 或 `Seek` 时，原有标准库 wrapper
仍然适用，包括 `LimitReader`、`CountingReader`、`ChecksumReader`、tee wrapper
及对应 writer。

## 6. Tokio 与 futures-io 桥接

两个方向都使用显式 newtype：

| 外部生态到 Qubit | Qubit 到外部生态 |
| --- | --- |
| `TokioInput`、`TokioOutput` | `TokioAsyncRead`、`TokioAsyncWrite` |
| `FuturesInput`、`FuturesOutput` | `FuturesAsyncRead`、`FuturesAsyncWrite` |

显式 wrapper 避免重叠 blanket impl。默认 feature 不启用任何异步生态依赖。

## 7. 分层建议

- 通用传输与缓冲使用 `qubit-io`；
- typed binary value 使用 `qubit-io-binary`；
- Unicode 文本和 charset 转换使用 `qubit-io-text`；
- 字节流还需要文件身份与 commit/abort 生命周期时使用 `qubit-fs`。

codec 状态应独立于同步或异步驱动。同一套 codec 应分别由 `Input`/`Output`
或 `AsyncInput`/`AsyncOutput` 驱动，而不是复制两套算法。
