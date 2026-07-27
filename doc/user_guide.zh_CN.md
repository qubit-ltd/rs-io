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
`Input::read_fully` 返回 EOF 前实际可读的 item 数；`Input::read_exactly`
则要求填满整个目标，否则报告 `UnexpectedEof`。

## 3. 异步 trait

异步核心不依赖 executor：

```text
AsyncInput::poll_read_unchecked
AsyncOutput::poll_write_unchecked
AsyncOutput::poll_flush
AsyncClose::poll_close
```

底层 poll trait 支持 `!Unpin` 实现。便利扩展方法要求 `Unpin` 并返回具名
Future：

- `read_async`：执行一次读取；
- `read_fully_async`：填满目标，或在 EOF 停止；
- `read_exactly_async`：填满目标，否则报告 `UnexpectedEof`；
- `write_async`：执行一次写入；
- `write_fully_async`：接受全部来源，否则报告 `WriteZero`；
- `flush_async`：flush 输出。
- `close_async`：关闭实现了 `AsyncClose` 的输出。

已经 pinned 的 `!Unpin` 实现和 trait object 通过 `PinnedAsyncInputExt`、
`PinnedAsyncOutputExt` 使用同名操作。`ReadFullyFuture`、`ReadExactFuture` 和
`WriteFullyFuture` 会暴露已完成 item 数，便于取消后核算进度。

poll 契约是严格的：零长度传输立即完成且不轮询内部 stream；`Pending` 和错误
不得传输 item；`Pending` 必须注册当前 waker；`WouldBlock`、`Interrupted`
不得跨越异步 trait 边界。非空读取返回零表示 EOF；完整写入中返回零会转换为
`WriteZero`。具名 Future 完成后再次 poll 属于调用错误并会 panic。

## 4. 缓冲

`Buffer<T>` 持有已初始化的标量存储，并跟踪 `position..limit` readable window。

同步缓冲：

- `BufferedInput<I>` 保留预读 item，并通过 `unread()` 暴露它们；
- `BufferedOutput<O>` 聚合小写入并向 `O` flush；
- `EnsuredBufferedInput` 与 `EnsuredBufferedOutput` 避免重复套缓冲。

`ensure` 只能识别由 `Input::is_buffered` 或 `Output::is_buffered`
报告的缓冲状态。标准库的 `BufReader` 和 `BufWriter` 通过 `Read`/`Write`
blanket 实现接入，因而不会被识别；不要将已经由标准库缓冲的 stream 传给
`ensure`。

异步缓冲：

- `AsyncBufferedInput<I>` 跨 `Pending` 保留已预读 item；
- `AsyncBufferedOutput<O>` 保留已接受 item 和部分 flush 进度。

消费同步缓冲器时需要显式选择：

- `BufferedInput::into_inner()` 会丢弃未消费的预读 item；
  `try_into_inner()` 会报告这些 item，`into_parts()` 则可将其取回。
- `BufferedOutput::into_inner()` 会先 flush；失败时返回
  `IntoInnerError<Self>` 并保留 pending item。`try_into_inner()` 是兼容别名，
  `into_parts()` 不执行 I/O。

丢弃 `BufferedOutput` 会尝试 best-effort flush。调用
`IntoInnerError::into_error()` 会丢弃其中保留的 output，因此也可能触发该尝试；
若 pending 数据必须由调用方控制，应通过 `into_inner()`、`into_writer()` 或
`into_parts()` 取回 output。

`AsyncBufferedInput` 仅提供 `into_parts()`，避免 `into_inner()` 静默丢弃预读但
未消费的 item。内部 output 支持 `AsyncClose` 时，`AsyncBufferedOutput` 会先
排空自身缓冲区，再关闭内部 output。

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

同步 wrapper 以 item 为单位：`LimitInput` / `LimitOutput`、
`CountingInput` / `CountingOutput` 以及 tee wrapper 支持任意 item。
`ChecksumInput` 与 `ChecksumOutput` 仍仅支持字节，因为 `Hasher` 消费字节。
标准 `Read` 与 `Write` 可通过 blanket implementation 直接作为字节 input/output。

## 6. Tokio 与 futures-io 桥接

两个方向都使用显式 newtype：

| 外部生态到 Qubit | Qubit 到外部生态 |
| --- | --- |
| `TokioInput`、`TokioOutput` | `TokioAsyncRead`、`TokioAsyncWrite` |
| `FuturesInput`、`FuturesOutput` | `FuturesAsyncRead`、`FuturesAsyncWrite` |

显式 wrapper 避免重叠 blanket impl。默认 feature 不启用任何异步生态依赖。

close 不会用 flush 冒充：`TokioOutput` 委托给 `poll_shutdown`，
`FuturesOutput` 委托给 `poll_close`，反向 write adapter 则要求
`AsyncClose<Item = u8>`。

## 7. 分层建议

- 通用传输与缓冲使用 `qubit-io`；
- typed binary value 使用 `qubit-io-binary`；
- Unicode 文本和 charset 转换使用 `qubit-io-text`；
- 字节流还需要文件身份与 commit/abort 生命周期时使用 `qubit-fs`。

codec 状态应独立于同步或异步驱动。同一套 codec 应分别由 `Input`/`Output`
或 `AsyncInput`/`AsyncOutput` 驱动，而不是复制两套算法。

## 8. 从 0.13 迁移

0.14 将同步 `Reader`/`Writer` wrapper 改名为 `Input`/`Output` wrapper，
并相应修改了它们实现的 trait。完整的类型映射、方法变化和 tee 失败语义请参阅
[0.14 迁移指南](migration-0.14.zh_CN.md)。
