# Qubit IO 用户指南

## 目录

1. [先理解抽象边界](#1-先理解抽象边界)
2. [添加依赖并选择 feature](#2-添加依赖并选择-feature)
3. [案例一：有上限的长度前缀 frame](#3-案例一有上限的长度前缀-frame)
4. [保持异步驱动运行时中立](#4-保持异步驱动运行时中立)
5. [在 decoder 周围组合传输策略](#5-在-decoder-周围组合传输策略)
6. [在部分进度后恢复所有权](#6-在部分进度后恢复所有权)
7. [案例二：不需要字节管线的 Map/Reduce 记录](#7-案例二不需要字节管线的-mapreduce-记录)
8. [标准 I/O、seek 与高级工具](#8-标准-ioseek-与高级工具)
9. [选择最窄的边界](#9-选择最窄的边界)

## 1. 先理解抽象边界

当库拥有传输算法、却不应拥有调用方的运行时、transport 或 item 类型时，使用 Qubit IO。codec 可以依赖 `Input<Item = u8>` 或 `AsyncInput<Item = u8>`；应用决定字节来自标准 reader、Tokio 还是 `futures-io`。

本 crate 有意止步于传输，不表达路径、文件身份、metadata、publication、commit、abort 或持久化。需要这些生命周期语义时使用 [qubit-fs](https://docs.rs/qubit-fs)；需要 typed byte value 时使用 [qubit-io-binary](https://docs.rs/qubit-io-binary)；需要文本编码时使用 [qubit-io-text](https://docs.rs/qubit-io-text)。

本指南用两个案例回答两个问题：

- 长度前缀 frame 说明异步库边界为何不应选择 Tokio 或 `futures-io`。
- Map/Reduce mapper 说明 item stream 不只是换了名字的字节流。

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

`tokio` 启用 `TokioInput`、`TokioOutput`、`TokioAsyncRead` 和 `TokioAsyncWrite`；`futures-io` 启用对应的 `Futures*` 类型。核心 trait 不选择 executor，也不要求这两个 feature。

adapter 的名称表示目标接口：

| 已有值 | 暴露为 | Tokio adapter | futures-io adapter |
| --- | --- | --- | --- |
| 生态 `AsyncRead` | Qubit `AsyncInput<Item = u8>` | `TokioInput` | `FuturesInput` |
| 生态 `AsyncWrite` | Qubit `AsyncOutput<Item = u8>` | `TokioOutput` | `FuturesOutput` |
| Qubit `AsyncInput<Item = u8>` | 生态 `AsyncRead` | `TokioAsyncRead` | `FuturesAsyncRead` |
| Qubit `AsyncOutput<Item = u8>` | 生态 `AsyncWrite` | `TokioAsyncWrite` | `FuturesAsyncWrite` |

## 3. 案例一：有上限的长度前缀 frame

该协议有四字节大端长度头和指定长度的 payload。空 payload 合法；声明长度超过 64 KiB 时必须在分配前拒绝；截断输入是错误。包含断言且经过 Cargo 检查的完整程序见 [examples/bounded_frame.rs](../examples/bounded_frame.rs)。

```rust
use std::io::{self, Error, ErrorKind};
use qubit_io::Input;

const MAX_FRAME_LEN: usize = 64 * 1024;

fn read_frame<I>(input: &mut I) -> io::Result<Vec<u8>>
where
    I: Input<Item = u8>,
{
    let mut header = [0_u8; 4];
    input.read_exactly(&mut header)?;
    let length = u32::from_be_bytes(header) as usize;
    if length > MAX_FRAME_LEN {
        return Err(Error::new(ErrorKind::InvalidData, "frame is too large"));
    }

    let mut payload = vec![0_u8; length];
    input.read_exactly(&mut payload)?;
    Ok(payload)
}
```

`Input::read_exactly` 会把不完整的 header 或 payload 转换为 `UnexpectedEof`。即使外层 wrapper 已限制连接，仍必须显式校验长度：transport quota 与协议规则保护不同边界。

所有 `std::io::Read` 都实现 `Input<Item = u8>`，因此命令行程序可在测试中向 `read_frame` 传入 `Cursor`，在生产环境传入 `File`。decoder 不含文件专属行为。

## 4. 保持异步驱动运行时中立

异步入口使用同一套协议规则，但依赖 `AsyncInput`：

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

Tokio 调用方将 reader 包装成 `TokioInput`，`futures-io` 调用方包装成 `FuturesInput`。二者调用同一个 `read_frame_async`。当 Qubit byte stream 需要暴露给对应生态时，使用反向 adapter `TokioAsyncRead` 和 `FuturesAsyncRead`。

同步与异步驱动仍是两个函数。这里的价值是异步 public API 不必分别为 Tokio 与 `futures-io` 维护实现。

## 5. 在 decoder 周围组合传输策略

调用方可把 transport policy 放在协议外部：

```text
transport adapter -> limit -> buffer -> counting -> frame decoder
```

例如 Tokio 服务可构造：

```rust,ignore
use qubit_io::{
    AsyncBufferedInput, AsyncCountingInput, AsyncLimitInput, TokioInput,
};

let mut input = AsyncCountingInput::new(AsyncBufferedInput::with_capacity(
    AsyncLimitInput::new(TokioInput::new(stream), 1_048_576),
    8 * 1024,
));
let frame = read_frame_async(&mut input).await?;
let consumed = input.bytes_read();
```

1 MiB limit 约束连接，decoder 中的 64 KiB 校验约束单个 frame。两项策略都无需知道对方的实现。

wrapper 顺序会改变语义。比较以下两条同步数据流：

```text
decoder -> CountingInput -> BufferedInput -> LimitInput -> source
decoder -> BufferedInput -> CountingInput -> LimitInput -> source
```

第一条中 counter 位于 buffer 外侧，统计交付给 decoder 的 item；第二条中 counter 位于 buffer 内侧，统计从 source 拉取的 item，包含尚未消费的预取 item。决定 checksum 表示 codec 消费的字节还是 transport 拉取的字节时，也遵循同一规则。

`LimitInput` 与 `LimitOutput` 最多暴露剩余指定数量的 item。`CountingInput` 与 `CountingOutput` 对成功 item 做饱和计数。`ChecksumInput` 与 `ChecksumOutput` 仅 hash 成功传输的 `u8` 前缀。`TeeInput` 与 `TeeOutput` 镜像 source 或 primary，但它们有序且非事务性：branch 失败不会回滚 source 或 primary 已完成的工作。

`inner_mut()` 与 `branch_mut()` 有意绕过 wrapper 记账；经由它们的读写不会被限量、计数、hash 或镜像。

## 6. 在部分进度后恢复所有权

`read` 与 `write` 只执行一次操作，可能部分完成。`read_fully` 在 EOF 时停止并返回已传输数量；`read_exactly` 未填满目标时返回 `UnexpectedEof`；`write_fully` 重试 interrupted write，并在输出停止进度时返回 `WriteZero`。

buffer 使所有权保持显式：

- `BufferedInput::into_parts` 返回 inner input 与 unread buffer，不会丢弃预取 item。继续读取 inner input 前先处理 unread window。
- `BufferedOutput::into_parts` 不执行 I/O，返回 inner output 与 pending item。正常完成时先 flush；flush 失败后保留 wrapper 以便重试或检查。
- 丢弃同步 `BufferedOutput` 只会 best-effort flush。异步析构函数无法 I/O，必须显式调用 `flush_async`。
- `AsyncClose::close_async` 表示真实 transport shutdown；flush 不是 close。

具名 async future 保留跨多次 poll 的状态。丢弃 pending `ReadFullyFuture`、`ReadExactFuture` 或 `WriteFullyFuture` 前，若恢复需要精确进度，读取 `items_read()` 或 `items_written()`。一次底层 poll 返回 `Pending` 或错误时不会报告新的成功 item，但聚合 future 可能已经保存前序 poll 的进度；实现不得让 `WouldBlock` 或 `Interrupted` 穿过异步 trait 边界。

## 7. 案例二：不需要字节管线的 Map/Reduce 记录

item stream 可以传输类型化业务记录。下面的 mapper 不知道记录来自内存分区、文件引擎还是网络反序列化器。包含内存 adapter 与断言、且经过 Cargo 检查的完整程序见 [examples/typed_records.rs](../examples/typed_records.rs)。

```rust
use std::io;
use qubit_io::{Input, Output};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Sale {
    store_id: u32,
    category_id: u16,
    amount_cents: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct CategoryRevenue {
    category_id: u16,
    amount_cents: u64,
}

fn map_partition<I, O>(input: &mut I, output: &mut O) -> io::Result<()>
where
    I: Input<Item = Sale>,
    O: Output<Item = CategoryRevenue>,
{
    let mut sales = [Sale::default(); 2];
    loop {
        let count = input.read(&mut sales)?;
        if count == 0 {
            return Ok(());
        }

        let mut revenues = [CategoryRevenue::default(); 2];
        for (sale, revenue) in sales[..count].iter().zip(&mut revenues) {
            *revenue = CategoryRevenue {
                category_id: sale.category_id,
                amount_cents: sale.amount_cents,
            };
        }
        output.write_fully(&revenues[..count])?;
    }
}
```

执行引擎只需实现一次 I/O 边界。下列最小内存 adapter 使案例可运行；应用代码通常从引擎获得 adapter。

```rust
use std::io;
use qubit_io::{Input, Output};

struct SliceRecordInput<'a, T> {
    items: &'a [T],
    position: usize,
}

impl<T: Copy> Input for SliceRecordInput<'_, T> {
    type Item = T;

    unsafe fn read_unchecked(
        &mut self,
        output: &mut [T],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        let read = count.min(self.items.len() - self.position);
        output[index..index + read]
            .copy_from_slice(&self.items[self.position..self.position + read]);
        self.position += read;
        Ok(read)
    }
}

#[derive(Default)]
struct VecRecordOutput<T> {
    items: Vec<T>,
}

impl<T: Copy> Output for VecRecordOutput<T> {
    type Item = T;

    unsafe fn write_unchecked(
        &mut self,
        input: &[T],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        self.items.extend_from_slice(&input[index..index + count]);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
```

`unsafe` 方法被限制在 adapter 中。调用方已证明 indexed range 合法；实现恰好复制 `count` 个 item，并以同样数量推进 source。mapper 本身只调用安全操作。

现在组合 record pipeline：

```rust
use std::io;
use qubit_io::{
    BufferedInput, CountingInput, CountingOutput, LimitInput, TeeOutput,
};

fn run_mapper() -> io::Result<()> {
    let source = SliceRecordInput {
        items: &[
            Sale { store_id: 1, category_id: 7, amount_cents: 300 },
            Sale { store_id: 2, category_id: 7, amount_cents: 500 },
            Sale { store_id: 3, category_id: 9, amount_cents: 900 },
        ],
        position: 0,
    };
    let limited = LimitInput::new(source, 2);
    let buffered = BufferedInput::with_capacity(limited, 2);
    let mut input = CountingInput::new(buffered);

    let output = TeeOutput::new(
        VecRecordOutput::default(),
        VecRecordOutput::default(),
    );
    let mut output = CountingOutput::new(output);
    map_partition(&mut input, &mut output)?;

    assert_eq!(2, input.items_read());
    assert_eq!(2, output.items_written());
    let (shuffle, audit) = output.into_inner().into_parts();
    assert_eq!(shuffle.items, audit.items);
    assert_eq!(2, shuffle.items.len());
    Ok(())
}
```

这里的 limit、buffer 与 counter 都以 record 而不是字节为单位。`TeeOutput` 把类型化 mapping 结果同时写入 shuffle sink 与 audit sink，而无需修改 mapper。

边界也必须明确：

- `Input`/`Output`、limit、counting 与 tee 可以传输非 `u8` item。
- checksum wrapper 仅支持字节，因为 `std::hash::Hasher` 消费字节。
- Qubit `Buffer<T>` 与 buffered wrapper 要求 `Copy + Default`；本例记录满足该条件。
- 含 `String` 等非 `Copy` 字段的记录仍可使用核心 stream 与不要求复制的 wrapper，但不能直接使用当前泛型 buffer。
- 网络和磁盘边界仍需要编码。本例的价值是避免每个业务 operator 重复编解码。

## 8. 标准 I/O、seek 与高级工具

### 标准 I/O 扩展

标准库集成位于 `qubit_io::std_io`，扩展 trait 位于 `qubit_io::std_io::ext`。它们为字节流提供 bounded read、bounded string 与 delimiter read、copy、discard 和保持位置的操作。处理由其他方控制大小的数据时，优先使用 `*_limited`，不要使用无上限的 `read_to_end`、`read_to_string` 或 delimiter read；上限是资源策略。

### seek 与传输工具

`Seekable` 的位置以 wrapped stream 的 item 单位衡量。`SeekableInput` 与 `SeekableOutput` 只表达有用组合，不增加新行为。`PositionGuard` 除非被 dismiss，否则在 drop 时恢复记录的位置；需要观察恢复错误时调用 `restore`。

`Streams` 是不可构造命名空间。`copy_input_to_output*` 处理泛型 Qubit item；`copy*` 与比较方法处理 `std::io` 字节流。

### buffer 与 pinned 异步值

`Buffer<T>` 持有已初始化的 `Copy + Default` 存储，公开 readable window 与 spare slot。它的低层状态修改方法是 `unsafe`，因为调用方必须证明 range 合法。普通缓冲使用 `BufferedInput` 或 `BufferedOutput`；只有实现专用 driver 或 encoder 时才直接使用 `Buffer<T>`。

`BufferedInput::ensure` 与 `BufferedOutput::ensure` 仅在 `is_buffered()` 报告已缓冲时避免再包 Qubit buffer。它们无法识别经 blanket `Read`/`Write` implementation 接入的 `std::io::BufReader` 或 `BufWriter`。

已 pinned 的 `!Unpin` async 值与 trait object 应使用 `PinnedAsyncInputExt`、`PinnedAsyncOutputExt`，而不是 `Unpin` 便利方法。

## 9. 选择最窄的边界

1. 阻塞传输使用 `Input`/`Output`；只有调用方已有异步执行路径时才使用 async trait。
2. 将 limit 放在最接近不可信 input 或 output quota 的位置。
3. 在最能从批处理获益的层添加 buffer。不要通过 `ensure` 再包标准 `BufReader` 或 `BufWriter`。
4. 按必须观测的字节或 item 放置 counter 与 checksum。
5. 显式 flush 或 close。恢复需要 unread 或 pending data 时，失败后保留 wrapper。
6. 不需要运行时中立边界、item 泛型传输或可组合策略时，直接使用原生 I/O。

[API 文档](https://docs.rs/qubit-io)说明每个 API 精确的错误、panic、所有权与 pinning 约束；本指南说明这些 API 如何组成库设计。
