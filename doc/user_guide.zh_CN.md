# Qubit IO 用户指南

## 1. 这个 crate 解决什么问题

`qubit-io` 是运行时中立、以 item 为单位的传输层。它让 codec、buffer 和
wrapper 可以处理 stream，而无需在公开抽象中选定 `std::io`、Tokio 或
`futures-io`。

本 crate 有意只建模传输，不表示文件路径、文件身份、metadata、publication、
commit、abort 或持久化。需要这类生命周期语义时使用 `qubit-fs`；需要 typed
binary value 时使用 `qubit-io-binary`；需要文本与字符编码时使用
`qubit-io-text`。

| 需求 | 同步 API | 异步 API |
| --- | --- | --- |
| 搬运 item | `Input`、`Output` | `AsyncInput`、`AsyncOutput` |
| flush 或 close | `Output::flush` | `AsyncOutput::flush_async`、`AsyncClose::close_async` |
| 添加缓冲 | `BufferedInput`、`BufferedOutput` | `AsyncBufferedInput`、`AsyncBufferedOutput` |
| 限制或观测传输 | limit、counting、checksum、tee wrapper | 异步 limit、counting、checksum wrapper |
| 与其他生态互操作 | `std::io` blanket implementation | 可选 Tokio 与 `futures-io` newtype |

所有公共类型都会从 crate 根导出；内部模块不属于兼容性边界。

## 2. 添加依赖并选择 feature

默认 feature 只包含运行时中立的核心层：

```toml
[dependencies]
qubit-io = "0.14"
```

只有应用实际使用某个生态时才启用对应 adapter：

```toml
[dependencies]
qubit-io = { version = "0.14", features = ["tokio"] }
```

`tokio` 会启用 `TokioInput`、`TokioOutput`、`TokioAsyncRead` 和
`TokioAsyncWrite`；`futures-io` 会启用对应的 `Futures*` 类型。核心 trait 不选定
executor，也不要求启用这两个 feature。

## 3. 同步 item 传输

`Input` 产生 `Item`，`Output` 接受 `Item`。安全方法会校验实现返回的数量；只有
在能满足文档所述 range 契约时，才需要由实现者提供 unchecked indexed 操作。多数
应用只调用安全方法，无需自行实现 trait。

所有 `std::io::Read` 都实现 `Input<Item = u8>`，所有 `std::io::Write` 都实现
`Output<Item = u8>`。这只是字节流 adapter，不表示任意 input 都是文件。

```rust
use std::io::{Cursor, Result};
use qubit_io::{Input, Output};

fn main() -> Result<()> {
    let mut input = Cursor::new(b"qubit".to_vec());
    let mut bytes = [0_u8; 5];
    input.read_exactly(&mut bytes)?;

    let mut output = Vec::new();
    output.write_fully(&bytes)?;
    assert_eq!(b"qubit", output.as_slice());
    Ok(())
}
```

`read` 与 `write` 只执行一次操作，可能只完成部分传输。`read_fully` 在 EOF 时
停止并返回已传输数量；`read_exactly` 未填满目标时返回 `UnexpectedEof`。
`write_fully` 会重试 interrupted write；在所有 item 被接受前若输出报告零进度，
则返回 `WriteZero`。`flush` 要求 `Output` 送达内部缓冲的 item，不等同于 close。

item 类型是泛型。limit、counting 与 tee 等 wrapper 因而能处理字节以外的廉价
标量 item。checksum wrapper 有意仅支持字节，因为 `std::hash::Hasher` 消费字节。

### seek 与组合 trait

`Seekable` 是面向 item 的 `std::io::Seek` 对应物，位置以内部 stream 的单位
衡量；标准 `Seek` 类型的单位为 `u8`。`SeekableInput`、`SeekableOutput`、
`ReadSeek`、`WriteSeek`、`ReadWrite` 与 `ReadWriteSeek` 只表达有用的 trait
组合，不引入新行为。这些标准库组合 trait 从 `qubit_io::std_io` 导出。`PositionGuard` 记录 `Seekable` 的位置，除非调用
`dismiss`，否则会在 drop 时恢复；需要观察恢复错误时调用 `restore`。

## 4. `Buffer<T>` 与同步缓冲

`Buffer<T>` 持有已初始化的 `Copy + Default` 存储，并维护可读窗口
`position..limit`。`readable()` 返回待处理 item，`spare_mut()` 返回空闲且已初始
化的存储，`available()` 与 `spare_capacity()` 分别报告其长度。修改状态的底层方法
是 `unsafe`，调用方必须证明请求的 range 合法。它主要用于构建 buffered driver 与
专用 encoder。

```rust
use qubit_io::Buffer;

fn main() {
    let source = [10_u8, 20, 30];
    let mut buffer = Buffer::with_capacity(4);

    // SAFETY: `source[0..3]` 合法，新建 buffer 有四个 spare slot。
    unsafe { buffer.copy_from(&source, 0, source.len()) };
    assert_eq!(&[10, 20, 30], buffer.readable());

    // SAFETY: 当前有三个可读 item，消费两个仍在合法范围内。
    unsafe { buffer.consume(2) };
    assert_eq!(&[30], buffer.readable());
}
```

`BufferedInput<I>` 会预读 input item。手动处理其 unread window 前，使用
`fill_more`、`fill_until` 或 `ensure_available`；随后对已处理 item 恰好调用一次
`consume`。`BufferedOutput<O>` 聚合小写入，并在需要时 flush。二者默认容量均为
`DEFAULT_BUFFER_CAPACITY`；`with_capacity` 会把零容量钳制为一，
`try_with_capacity` 与 `try_reserve_capacity` 则报告分配失败。

```rust
use std::io::{Cursor, Result};
use qubit_io::BufferedInput;

fn main() -> Result<()> {
    let mut input = BufferedInput::with_capacity(
        Cursor::new(b"abcdef".to_vec()),
        4,
    );
    assert!(input.fill_until(3)?);
    assert_eq!(b"abcd", input.unread());

    // SAFETY: `fill_until(3)` 已成功，并且已有四个 item 被缓冲。
    unsafe { input.consume(2) };
    let (_inner, unread) = input.into_parts();
    assert_eq!(b"cd", unread.readable());
    Ok(())
}
```

`into_parts()` 使所有权决策保持显式。对于 input，它返回内部 input 与未读
`Buffer`；继续读取同一逻辑流前，必须先消费该 readable window。对于 output，它不
执行 I/O，直接返回内部 output 与 pending `Buffer`。正常完成时先调用 `flush`；若
刷新失败，wrapper 仍由调用方持有，可检查或重试。

丢弃同步 `BufferedOutput` 会 best-effort flush。不要把 drop 当作送达保证；需要取得
output 所有权时，应显式 `flush` 后再使用 `into_parts`。

```rust
use std::io::Result;
use qubit_io::BufferedOutput;

fn main() -> Result<()> {
    let mut output = BufferedOutput::with_capacity(Vec::<u8>::new(), 4);
    output.write_fully(b"abc")?;
    output.flush()?;

    let (inner, pending) = output.into_parts();
    assert_eq!(b"abc", inner.as_slice());
    assert!(pending.is_empty());
    Ok(())
}
```

`BufferedInput::ensure` 与 `BufferedOutput::ensure` 仅在 `is_buffered()` 报告
已缓冲时避免再次套 Qubit buffer。它们无法识别 `std::io::BufReader` 或
`BufWriter`，因为后者经由 `Read`/`Write` blanket implementation 接入。不要把
已由标准库缓冲的 stream 交给 `ensure`。

## 5. Wrapper 与组合

每个 wrapper 都在转发底层契约的同时只增加一项策略：

| Wrapper 系列 | 含义 | 关键边界 |
| --- | --- | --- |
| `LimitInput` / `LimitOutput` | 最多暴露剩余指定数量的 item | input 到零时表现为 EOF；output 不再接受 item |
| `CountingInput` / `CountingOutput` | 对成功 item 做饱和计数 | `u8` 可用 `bytes_*`；任意 item 可用 `items_*` |
| `ChecksumInput` / `ChecksumOutput` | 对成功传输的字节前缀计算 hash | 仅 `u8`；`Pending` 与错误不更新 hash |
| `TeeInput` / `TeeOutput` | 将 source 或 primary 传输镜像到 branch | 有序且非事务性 |
| `SyncSeekTeeInput` | 镜像读取并同步 seek | branch 失败前 source 已可能改变 |

`inner_mut()` 与 `branch_mut()` 有意绕过 wrapper 的记账。经由它们的读取和写入
不会被计数、限量、hash 或镜像；只应在这正是所需 escape hatch 时使用。

```rust
use std::io::Result;
use qubit_io::{CountingOutput, Output, TeeOutput};

fn main() -> Result<()> {
    let tee = TeeOutput::new(Vec::<u8>::new(), Vec::<u8>::new());
    let mut output = CountingOutput::new(tee);
    output.write_fully(b"copy")?;
    assert_eq!(4, output.bytes_written());

    let (primary, branch) = output.into_inner().into_parts();
    assert_eq!(b"copy", primary.as_slice());
    assert_eq!(b"copy", branch.as_slice());
    Ok(())
}
```

tee write 先更新 primary output，再更新 branch；tee read 先推进 source，再写入
branch。flush 与同步 seek 也会先操作 primary 或 source。后续错误不会回滚前面的
工作；需要原子复制时，应在 Qubit IO 之上增加事务或恢复层。

## 6. 标准 I/O 扩展与 `Streams`

扩展 trait 面向标准字节流，并提供显式资源上限。`ReadExt` 提供 exact-or-EOF
读取、受限 vector 与 string、受限 copy 和 discard；`BufReadExt` 提供受限的行与
分隔符读取；`ReadSeekExt`、`SeekExt` 与 `WriteSeekExt` 提供保持位置的操作。
unchecked 扩展方法具有其名称所表达的同类 range 约束。

`Streams` 是不可构造的命名空间。`copy_input_to_output*` 方法处理泛型 Qubit
item；`copy*` 与比较方法处理 `std::io` 字节流。

```rust
use std::io::{Cursor, Result};
use qubit_io::Streams;
use qubit_io::std_io::ext::BufReadExt;

fn main() -> Result<()> {
    let mut input = Cursor::new(b"abcdef".to_vec());
    let mut output = Vec::new();
    let copied = Streams::copy_input_to_output_at_most(
        &mut input,
        &mut output,
        3,
    )?;
    assert_eq!(3, copied);
    assert_eq!(b"abc", output.as_slice());

    let mut line_input = Cursor::new(b"hello\nrest".to_vec());
    assert_eq!("hello\n", line_input.read_line_limited(6)?);
    Ok(())
}
```

处理由其他方控制大小的数据时，应使用 `*_limited`，不要使用无上限的
`read_to_end`、`read_to_string` 或 delimiter read。这个上限是调用方资源策略的
一部分，而不仅是便利参数。

## 7. 异步契约

`AsyncInput` 与 `AsyncOutput` 使用 `Pin`、`Context` 和 `Poll`，但不依赖特定
runtime。实现者提供 `poll_read_unchecked` 或 `poll_write_unchecked`；使用者通常
调用 `read_async`、`read_fully_async`、`read_exactly_async`、`write_async`、
`write_fully_async` 与 `flush_async`。

契约严格如下：

- 零长度传输立即完成，不轮询内部 stream。
- `Poll::Pending` 与错误不传输 item；`Pending` 必须注册当前 waker。
- `WouldBlock` 与 `Interrupted` 不得越过这个边界。
- 非空 read 返回零表示 EOF；full write 返回零会变为 `WriteZero`。
- 具名操作 future 完成后再次 poll 是调用方错误，会 panic。

`ReadFuture`、`ReadFullyFuture`、`ReadExactFuture`、`WriteFuture`、
`WriteFullyFuture`、`FlushFuture` 与 `CloseFuture` 把跨多次 poll 的状态保存在
future 自身。在丢弃尚未完成的 `ReadFullyFuture`、`ReadExactFuture` 或
`WriteFullyFuture` 前，若恢复逻辑需要精确进度，可查询 `items_read()` 或
`items_written()`。已 pinned 的 `!Unpin` 值和 trait object 应通过
`PinnedAsyncInputExt`、`PinnedAsyncOutputExt` 使用这些操作，而不是 `Unpin`
便利方法。

`AsyncClose` 与 flush 不同；close 表示真实的 transport shutdown，并通过
`close_async` 暴露。

## 8. 异步缓冲与 adapter

`AsyncBufferedInput` 会跨 `Pending` 保留预读 item，并提供
`poll_fill_more`、`poll_fill_until`、`poll_ensure_available` 供手动窗口管理。
`AsyncBufferedOutput` 会保留每一个已接受但尚未被内部 output 接受的 item；部分
flush 在返回 `Pending` 前会更新已保留的进度。二者都提供 `try_with_capacity` 与
`try_reserve_capacity`，支持感知分配失败的构造方式。

异步析构函数无法执行 I/O。要保证 flush，请调用 `flush_async`；要恢复内部 stream
及 pending buffer，请调用 `into_parts`。当内部 output 支持 `AsyncClose` 时，
`AsyncBufferedOutput` 会先排空自身 pending item，再向内部 output 委托 close。

Tokio adapter 使用显式 newtype，以避免不同生态 trait 的重叠实现。以下完整程序
经由 `AsyncBufferedOutput` 写入并 close，再经由 `AsyncBufferedInput` 读取：

```toml
[dependencies]
qubit-io = { version = "0.14", features = ["tokio"] }
tokio = { version = "1", features = ["macros", "rt", "io-util"] }
```

```rust
use std::io;
use qubit_io::{
    AsyncBufferedInput, AsyncBufferedOutput, AsyncClose, AsyncInput,
    AsyncOutput, TokioInput, TokioOutput,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let (writer, reader) = tokio::io::duplex(64);
    let mut output = AsyncBufferedOutput::with_capacity(TokioOutput::new(writer), 4);
    output.write_fully_async(b"qubit").await?;
    output.close_async().await?;

    let mut input = AsyncBufferedInput::with_capacity(TokioInput::new(reader), 4);
    let mut received = [0_u8; 5];
    input.read_exactly_async(&mut received).await?;
    assert_eq!(b"qubit", &received);
    Ok(())
}
```

`TokioInput` 与 `TokioOutput` 把 Tokio 适配到 Qubit IO；`TokioAsyncRead` 与
`TokioAsyncWrite` 则把 Qubit byte stream 暴露给 Tokio。`FuturesInput`、
`FuturesOutput`、`FuturesAsyncRead`、`FuturesAsyncWrite` 为 `futures-io` 提供
相同的双向桥接。Tokio close 委托给 `poll_shutdown`，futures-io close 委托给
`poll_close`；二者都不会用 flush 冒充 close。

## 9. 选择并恢复正确的 owner

组合 stream 层时，请按以下清单检查：

1. 阻塞传输选择 `Input`/`Output`；只有调用方已经拥有异步执行路径时才选择异步
   trait。
2. 在最能从批处理获益的最外层添加 buffer。不要经由 `ensure` 再包一层标准
   `BufReader` 或 `BufWriter`。
3. 将 limit 放在最靠近不可信 input 或受保护 output quota 的位置。
4. 根据必须观测的字节或 item 放置 counter 与 checksum，不要只按构造方便程度。
5. 显式 flush 或 close。操作失败时，保留 wrapper 并检查或重试；只有明确要接管
   pending 数据时才使用 `into_parts`。

docs.rs 上的 API 文档说明每个方法精确的错误、panic、所有权与 pinning 约束；本
指南说明这些 API 在应用中应如何组合。
