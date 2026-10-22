# Qubit IO 用户手册

[English](user_guide.md) | [项目介绍](../README.zh_CN.md) | [API 文档](https://docs.rs/qubit-io/0.19.0/qubit_io/)

本手册面向需要处理字节、字符或业务对象流的 Rust 开发者，以及需要扩展流实现的库作者。内容以 `qubit-io 0.19.0` 当前代码为准，要求 Rust 1.94 或更新版本，使用 Rust 2024 edition。

## 目录

1. [认识泛型输入输出流](#1-认识泛型输入输出流)
2. [构建第一条对象处理管道](#2-构建第一条对象处理管道)
3. [掌握基本读写语义](#3-掌握基本读写语义)
4. [连接标准库字节流](#4-连接标准库字节流)
5. [为流添加缓冲](#5-为流添加缓冲)
6. [限制与统计传输量](#6-限制与统计传输量)
7. [校验和与内容摘要](#7-校验和与内容摘要)
8. [镜像输入与输出](#8-镜像输入与输出)
9. [定位、复制与比较](#9-定位复制与比较)
10. [使用异步泛型流](#10-使用异步泛型流)
11. [实现自己的输入输出流](#11-实现自己的输入输出流)
12. [排障与能力速查](#12-排障与能力速查)

初次接入请顺序阅读第 1—3 章，再按需要选择包装器；已有标准字节流可先读第 4 章。异步使用者继续阅读第 10 章，自定义实现者阅读第 11 章。API 文档提供精确签名，本手册解释如何组合、检查结果和处理失败。

## 1. 认识泛型输入输出流

一个批处理任务需要读取销售记录，生成分类收入记录，再交给下一个处理阶段。如果每个阶段的接口都只接收字节，业务处理就需要反复参与序列化。Qubit IO 为这些阶段提供统一的泛型流接口：

```text
Input<Item = Sale> → 业务处理 → Output<Item = CategoryRevenue>
```

`Input` 将元素读入调用方提供的切片，`Output` 从切片接收元素。关联类型 `Item` 决定元素类型；`u8`、`char`、`String` 和业务结构体都可以成为流的元素。一次读取或写入返回的数量以元素为单位，因此读取 10 条记录和读取 10 个字节是不同的操作。

核心 trait 本身没有统一的 `Copy`、`Clone` 或 `Default` 约束。具体实现和包装器可以增加要求：当前缓冲要求 `Clone + Default`，校验和与摘要只接受字节。泛型对象流不负责对象的磁盘或网络编码；跨这些边界时，仍需由编码层将对象与字节互相转换。

围绕这两个接口，可以逐层添加缓冲、限量、计数、镜像等行为。处理函数依赖输入输出接口，调用方决定数据来源、目标及包装顺序。标准库 `Read`、`Write` 自动接入字节流；异步接口提供同样的元素模型，并通过适配器接入不同生态。

本库的职责是传输元素。路径、文件身份、提交、撤销和持久化属于更高层；可参考 [qubit-fs](https://docs.rs/qubit-fs)。字节中的基本类型读写见 [qubit-io-binary](https://docs.rs/qubit-io-binary)，文本编码见 [qubit-io-text](https://docs.rs/qubit-io-text)。只需原生字节 I/O 的应用也可以直接使用标准库或运行时接口。

## 2. 构建第一条对象处理管道

### 准备项目

创建 Rust 2024 二进制项目，在 `Cargo.toml` 中添加：

```toml
[dependencies]
qubit-io = "0.19.0"
```

默认 feature 为空，不需要异步运行时。将下列完整程序保存为 `src/main.rs`，运行 `cargo run`。程序使用内存来源和接收端作为入门练习；断言全部通过且退出码为 0 即表示成功，不产生终端输出。

### 读取销售记录，生成分类收入记录

程序只允许前三条销售记录中的两条进入本次任务；记录包含 `String`，无需先编码成字节。`map_sales` 将每条销售记录映射成一条收入记录，不执行分类求和。后续聚合由接收端或下游阶段负责。

`Records` 和 `Collected` 是本例自行实现的内存适配器，不是库内置类型。阅读时可以先看 `map_sales` 与 `main`；第 11 章解释底层实现契约。

<!-- example: objects -->
```rust
use std::io;
use qubit_io::{BufferedInput, BufferedOutput, CountingInput, CountingOutput,
               Input, LimitInput, Output};

#[derive(Clone, Debug, Default, PartialEq)]
struct Sale {
    category: String,
    amount_cents: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct CategoryRevenue {
    category: String,
    amount_cents: u64,
}

// Application-owned adapters; not types provided by qubit-io.
struct Records<T> {
    items: Vec<T>,
    position: usize,
}

impl<T: Clone> Input for Records<T> {
    type Item = T;

    unsafe fn read_unchecked(
        &mut self, output: &mut [T], index: usize, count: usize,
    ) -> io::Result<usize> {
        let n = count.min(self.items.len() - self.position);
        output[index..index + n]
            .clone_from_slice(&self.items[self.position..self.position + n]);
        self.position += n;
        Ok(n)
    }
}

struct Collected<T>(Vec<T>);

impl<T: Clone> Output for Collected<T> {
    type Item = T;

    unsafe fn write_unchecked(
        &mut self, input: &[T], index: usize, count: usize,
    ) -> io::Result<usize> {
        self.0.extend_from_slice(&input[index..index + count]);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

fn map_sales<I, O>(input: &mut I, output: &mut O) -> io::Result<()>
where
    I: Input<Item = Sale> + ?Sized,
    O: Output<Item = CategoryRevenue> + ?Sized,
{
    let mut batch = std::array::from_fn::<_, 2, _>(|_| Sale::default());
    loop {
        let n = input.read(&mut batch)?;
        if n == 0 { return Ok(()); }
        for sale in &batch[..n] {
            let revenue = CategoryRevenue {
                category: sale.category.clone(),
                amount_cents: sale.amount_cents,
            };
            output.write_fully(&[revenue])?;
        }
    }
}

fn main() -> io::Result<()> {
    let source = Records {
        items: vec![
            Sale { category: "books".into(), amount_cents: 300 },
            Sale { category: "books".into(), amount_cents: 500 },
            Sale { category: "games".into(), amount_cents: 900 },
        ],
        position: 0,
    };
    let mut input = CountingInput::new(BufferedInput::with_capacity(
        LimitInput::new(source, 2), 4,
    ));
    let mut output = CountingOutput::new(BufferedOutput::with_capacity(
        Collected(Vec::new()), 4,
    ));
    map_sales(&mut input, &mut output)?;
    output.flush()?;
    assert_eq!(input.items_read(), 2);
    assert_eq!(output.items_written(), 2);
    let (sink, pending) = output.into_inner().into_parts();
    assert!(pending.is_empty());
    assert_eq!(sink.0, vec![
        CategoryRevenue { category: "books".into(), amount_cents: 300 },
        CategoryRevenue { category: "books".into(), amount_cents: 500 },
    ]);
    Ok(())
}
```

这条管道的职责如下：

| 层 | 本例中的职责 |
| --- | --- |
| `Records<Sale>` | 提供已经构造好的销售记录 |
| `LimitInput` | 最多暴露两条记录，不消耗第三条 |
| `BufferedInput` | 以记录为单位缓冲，容量为四条 |
| `CountingInput` | 统计处理函数真正读到的记录 |
| `map_sales` | 将销售记录映射成收入记录 |
| `CountingOutput<BufferedOutput<_>>` | 统计已接纳的收入记录；显式刷新后再检查接收端 |
| `Collected<CategoryRevenue>` | 保存本练习的最终结果 |

现在你已经验证了对象流输入、不同类型的输出、包装器组合、刷新和结果检查。`output.flush()?` 失败时不会继续执行成功断言；实际应用应保留输出对象，按第 5 章检查待写数据。

在真实项目中，应用负责准备数据来源和接收端、创建管道、调用处理函数，并决定失败后的重试或补偿。`map_sales` 可以独立放进业务模块；这里的两个内存适配器可由执行引擎提供的对象流实现替代。它们不代表持久化存储，也不包含事务或审计交付保证。

仓库还提供较短的、使用 `Copy` 记录的[类型化记录示例](../examples/typed_records.rs)：在仓库根目录执行 `cargo run --example typed_records`。该示例验证映射、限量、缓冲、计数与双路内存输出，不覆盖本章含 `String` 的记录或真实存储集成。

## 3. 掌握基本读写语义

`Input`、`Output` 以切片为批次操作，并不意味着每次都会完成整批传输。

| 操作 | 成功结果与结束条件 |
| --- | --- |
| `read` | 单次读取，返回实际元素数；非空请求返回 0 表示流结束 |
| `read_fully` | 重复读取直到填满或 EOF，返回已读取数量，重试 `Interrupted` |
| `read_exactly` | 必须填满；提前 EOF 返回 `UnexpectedEof` |
| `write` | 单次写入，返回实际接纳数量，可能短写 |
| `write_fully` | 重复写入直到全部接纳，重试 `Interrupted`；无进展返回 `WriteZero` |
| `flush` | 请求输出实现完成其刷新职责，须检查结果 |

空切片的零数量结果不代表 EOF。读取后只处理 `buffer[..n]`，不要把未填充的尾部当作有效元素。聚合方法返回错误时，前面的读取或写入可能已发生；错误不会自动恢复流位置，也不会提供事务回滚。

下例是独立的 `src/main.rs`：先接受短输入，再演示精确读取失败后的输入位置。

<!-- example: basic -->
```rust
use std::io::{self, Cursor, ErrorKind};
use qubit_io::{Input, Output};

fn main() -> io::Result<()> {
    let mut source = Cursor::new(b"abc");
    let mut batch = [0; 5];
    let n = source.read_fully(&mut batch)?;
    assert_eq!(n, 3);
    let mut destination = Vec::new();
    destination.write_fully(&batch[..n])?;
    assert_eq!(destination, b"abc");

    let mut truncated = Cursor::new(b"abc");
    assert_eq!(truncated.read_exactly(&mut [0; 5]).unwrap_err().kind(),
               ErrorKind::UnexpectedEof);
    assert_eq!(truncated.position(), 3);
    Ok(())
}
```

对第 2 章的对象管道，这些数量都是记录数。如果业务要求必须读满一批，可以使用 `read_exactly`；若尾批不足仍应处理，使用 `read` 或 `read_fully` 并检查返回数量。

[有上限的 frame 示例](../examples/bounded_frame.rs) 展示字节协议的另一种用法。运行 `cargo run --example bounded_frame` 会检查正常、空、超大和截断的消息。它使用四字节大端长度头，在分配前拒绝超过 64 KiB 的负载；输入限额不能代替协议长度校验。

## 4. 连接标准库字节流

每个 `std::io::Read` 实现自动实现 `Input<Item = u8>`；每个 `Write` 实现自动实现 `Output<Item = u8>`。因此 `File`、`Cursor` 等可以直接传入字节型 Qubit 算法，无须写适配器。

### 简化字节流 API 的约束

只处理字节的 API 可以把 `I: Input<Item = u8>` 写成 `I: ByteInput`，
把 `O: Output<Item = u8>` 写成 `O: ByteOutput`。这两个子 trait 不增加方法，
通过 blanket impl 自动覆盖所有符合条件的流，包括 `?Sized` 类型。
因此，标准库读写类型和已有的 Qubit 字节包装器都可以直接使用；自定义流仍只需实现
`Input` 或 `Output`，并将 `Item` 定义为 `u8`。

下面的独立程序同时展示参数约束和返回值写法。输出借用目标向量，通过 trait 对象传入
泛型函数，最后借助 `OutputRef` 转成标准库写接口，追加一个字节：

<!-- example: byte-traits -->
```rust
use std::io::{self, Cursor, Write};
use qubit_io::{ByteInput, ByteOutput, CountingOutput, Output, OutputRef};

fn input() -> impl ByteInput {
    Cursor::new(b"QBIT")
}

fn output(bytes: &mut Vec<u8>) -> impl ByteOutput + '_ {
    CountingOutput::new(bytes)
}

fn copy_magic<I: ByteInput + ?Sized, O: ByteOutput + ?Sized>(
    input: &mut I, output: &mut O,
) -> io::Result<()> {
    let mut magic = [0; 4];
    input.read_exactly(&mut magic)?;
    output.write_fully(&magic)
}

fn main() -> io::Result<()> {
    let mut source = input();
    let mut bytes = Vec::new();
    {
        let mut sink = output(&mut bytes);
        let input: &mut dyn ByteInput = &mut source;
        let output: &mut dyn ByteOutput = &mut sink;
        copy_magic(input, output)?;

        let mut writer = OutputRef::new(output).into_std_write();
        Write::write_all(&mut writer, b"!")?;
        Write::flush(&mut writer)?;
    }
    assert_eq!(bytes, b"QBIT!");
    Ok(())
}
```

最终断言检查了两次写入的结果：前四个字节由 Qubit 接口复制，末尾的 `!` 由标准库接口追加。
换用短约束不会改变错误语义：头部不完整时，`read_exactly` 仍返回 `UnexpectedEof`；
接收端无法继续写入时，`write_fully` 仍返回 `WriteZero`。这两个 trait 本身不增加
缓冲、分配或错误处理。

`ByteInput`、`ByteOutput` 是 trait，不能直接当作具体返回类型。
应使用 `impl ByteInput`／`impl ByteOutput`、具体流类型，或 `Box<dyn ByteInput>`
这样的指针类型。将具体返回类型改成 `impl ByteInput` 后，调用方就不能直接调用原类型的
专有方法。参数上的 `?Sized` 允许传入借用的 trait 对象；若 API 需要拥有一个实现
`Input`／`Output` 的值，可用 `BoxInput`／`BoxOutput` 包装拥有的 trait 对象。
异步字节流则可以使用 `AsyncByteInput`、`AsyncByteOutput`。

### 为字符流和异步流使用短约束

四种基础接口都有固定元素类型的子 trait：

| 基础接口 | `Item = u8` | `Item = char` |
| --- | --- | --- |
| `Input` | `ByteInput` | `CharInput` |
| `Output` | `ByteOutput` | `CharOutput` |
| `AsyncInput` | `AsyncByteInput` | `AsyncCharInput` |
| `AsyncOutput` | `AsyncByteOutput` | `AsyncCharOutput` |

这些 trait 继承基础接口的操作，并自动覆盖元素类型匹配的流，包括 unsized trait 对象。
自定义实现仍放在基础 trait 上，不要再显式实现对应的短约束 trait。
参数和返回值的用法与前面的字节示例相同，例如 `I: CharInput`、
`-> impl AsyncCharOutput`。它们不会额外要求 `Send`、`Sync` 或 `Unpin`。

下面是供调用方接入的 API 定义片段，分别展示同步字符操作、异步字符读取和异步字节写入。
具体流由调用方提供；异步调用还需要已固定的借用和任务上下文：

```rust
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use qubit_io::{AsyncByteOutput, AsyncCharInput, CharInput, CharOutput};

fn read_char<I: CharInput + ?Sized>(input: &mut I) -> io::Result<char> {
    let mut chars = ['\0'];
    input.read_exactly(&mut chars)?;
    Ok(chars[0])
}

fn write_chars<O: CharOutput + ?Sized>(output: &mut O, chars: &[char]) -> io::Result<()> {
    output.write_fully(chars)
}

fn poll_chars<I: AsyncCharInput + ?Sized>(
    input: Pin<&mut I>, cx: &mut Context<'_>, chars: &mut [char],
) -> Poll<io::Result<usize>> {
    input.poll_read(cx, chars)
}

fn poll_bytes<O: AsyncByteOutput + ?Sized>(
    output: Pin<&mut O>, cx: &mut Context<'_>, bytes: &[u8],
) -> Poll<io::Result<usize>> {
    output.poll_write(cx, bytes)
}
```

如果流包含 `['你', '🦀']`，一次完整读取返回两个元素；同样的文本编码为 UTF-8 后需要
七个字节。一个 `char` 表示一个 Unicode 标量值，不一定是用户看到的一个完整字符；
组合序列可能包含多个元素。上述 trait 不执行文本编解码、规范化或字素簇分割，
字符流也不能直接调用 `into_std_read()`、`into_std_write()`。跨越字节边界时需要编码层。

异步短约束保留基础接口的就绪规则：返回 `Pending` 时不能传输元素，实现还必须安排唤醒。
`!Unpin` 流须先固定再轮询；需要返回 Future 的辅助方法时，可使用
`PinnedAsyncInputExt`、`PinnedAsyncOutputExt`。Tokio、`futures-io` 类型仍需通过
现有适配器接入；仅实现运行时自己的异步接口，不会自动实现 `AsyncByteInput`、
`AsyncByteOutput`。

### 转换为标准库接口

反方向使用 `input.into_std_read()`、`output.into_std_write()`，得到 `StdRead`、`StdWrite`。转换消费原值；通过 `InputRef`、`OutputRef` 包装借用后再转换，可以保留原对象的所有权。下面的独立程序演示双向接入，并使用限定调用解决方法同名问题：

<!-- example: std -->
```rust
use std::io::{self, Cursor, Read, Write};
use qubit_io::{Input, InputRef, Output};

fn main() -> io::Result<()> {
    let mut source = Cursor::new(b"QBIT");
    let mut magic = [0; 4];
    Input::read_exactly(&mut source, &mut magic)?;
    assert_eq!(&magic, b"QBIT");

    let mut reader = InputRef::new(&mut source).into_std_read();
    let mut rest = Vec::new();
    Read::read_to_end(&mut reader, &mut rest)?;
    assert!(rest.is_empty());
    let _borrow = reader.into_inner();

    let mut writer = Vec::<u8>::new().into_std_write();
    Write::write_all(&mut writer, b"QBIT")?;
    Write::flush(&mut writer)?;
    assert_eq!(writer.into_inner(), b"QBIT");
    Ok(())
}
```

`StdRead`、`StdWrite` 提供 `get_ref`、`get_mut`、`into_inner`，在内部流实现字节单位的 `Seekable` 时也转发标准 `Seek`。这些转换只构造包装器，不分配、不读写、不刷新或关闭；标准向量读写采用默认回退，不承诺保留内部流的 scatter/gather 优化。

动态分发时，借用的 `dyn Input<Item = u8>` 可先放进 `InputRef`，拥有的 boxed trait object 使用 `BoxInput`；输出分别使用 `OutputRef`、`BoxOutput`。这些适配器也适用于对象流，但转换成标准 `Read`、`Write` 仍只允许 `u8`。

已经实现标准 `Read` 的具体类型，不能再直接实现另一套 `Input<Item = Sale>`，因为已有 blanket impl。为对象语义新建包装类型；`Write`/`Output` 和 `Seek`/`Seekable` 有对应的兼容性边界。

## 5. 为流添加缓冲

### 以元素为单位配置容量

`BufferedInput`、`BufferedOutput` 要求 `Item: Clone + Default`，所以 `String` 和满足条件的业务对象都可以缓冲。`new` 使用库的默认容量，`with_capacity(inner, n)` 指定元素槽位数；零容量会调整为一个槽位。需要处理分配失败时，选择 `try_with_capacity`、`try_reserve_capacity`。

对象缓冲不是零成本移动：初始化和传输可能调用 `Default`、`Clone`。四个 `String` 槽位不意味着最多占用四个字节；元素拥有的堆内存和深拷贝成本也要计入。对象数量上限不能替代对象大小的资源约束。

`BufferedInput::ensure`、`BufferedOutput::ensure` 根据 `is_buffered()` 决定是否增加缓冲，返回 `EnsuredBufferedInput`、`EnsuredBufferedOutput`；`ensure_boxed` 用于需要 boxed 边界的代码。标准 `BufReader`、`BufWriter` 经 blanket impl 接入时不能被这种检查识别，已有标准缓冲时不要据此再次自动包装。

### 预读、待写数据与所有权恢复

输入可能预读。`unread()`、`unread_len()` 查看尚未交给调用方的元素；`fill_more`、`fill_until` 可以补充缓冲，`ensure_available` 在无法取得所需数量时返回错误。`into_parts()` 不执行 I/O，返回底层输入和 `Buffer`，后者的 `readable()` 是必须先处理的未读窗口。

输出的 `write_fully` 成功可能仅表示进入缓冲。正常结束时先 `flush` 并检查结果；`into_parts()` 不刷新，返回底层输出与仍待写的窗口。以下独立程序故意在刷新前拆开输出，展示由调用方接管数据的过程：

<!-- example: buffer -->
```rust
use std::io::{self, Cursor};
use qubit_io::{BufferedInput, BufferedOutput, Input, Output};

fn main() -> io::Result<()> {
    let mut input = BufferedInput::with_capacity(Cursor::new(b"abcdef"), 4);
    assert!(input.fill_until(4)?);
    let mut first = [0; 1];
    input.read_exactly(&mut first)?;
    assert_eq!(&first, b"a");
    let (mut source, unread) = input.into_parts();
    let mut rest = unread.readable().to_vec();
    let mut tail = [0; 8];
    let n = source.read_fully(&mut tail)?;
    rest.extend_from_slice(&tail[..n]);
    assert_eq!(rest, b"bcdef");

    let mut output = BufferedOutput::with_capacity(Vec::<u8>::new(), 8);
    output.write_fully(b"abc")?;
    let (mut sink, pending) = output.into_parts();
    assert!(sink.is_empty());
    sink.write_fully(pending.readable())?;
    Output::flush(&mut sink)?;
    assert_eq!(sink, b"abc");
    Ok(())
}
```

flush 失败后应保留包装器。若在排空过程中失败，已成功写出的前缀会从待写窗口移除；不要重发整批数据。也可能待写数据已排空，而底层 `flush` 失败，此时仍需处理刷新失败。只有底层错误允许重试、且已知其副作用时，才重试刷新；也可用 `into_parts()` 接管剩余元素。

同步 `BufferedOutput` 析构只尽力排空自身缓冲，忽略错误，不能代替显式 `flush`。异步缓冲析构不执行 I/O。通过 `inner_mut()` 直接读取可能越过预读数据，直接写入可能排在待写数据之前；正常传输应经过包装器。

`Buffer<T>` 是这些包装器使用的底层存储，提供已消费、可读、空闲窗口。普通调用者优先使用缓冲流；只有专用驱动或编码器需要直接管理窗口时，才使用其 `unsafe` 状态修改方法，并遵守范围契约。

## 6. 限制与统计传输量

`LimitInput` 和 `LimitOutput` 的限额是 `u64` 元素数。`remaining()` 查看剩余额度，成功传输后扣减。输入额度耗尽返回 0，即使底层还有元素；输出额度耗尽也返回 0，因此超额的 `write_fully` 最终返回 `WriteZero`，已写前缀保留。

`CountingInput`、`CountingOutput` 对成功返回的传输量进行饱和计数，不会因 `u64` 溢出回绕。对象流使用 `items_read()`、`items_written()`；字节流另有 `bytes_read()`、`bytes_written()`。计数器不能恢复内部操作在返回错误前已经产生、却未报告为成功的副作用。

下面的独立程序比较两个观测位置，并检查额度耗尽后的结果：

<!-- example: order -->
```rust
use std::io::{self, Cursor, ErrorKind};
use qubit_io::{BufferedInput, CountingInput, Input, LimitInput,
               LimitOutput, Output};

fn main() -> io::Result<()> {
    let mut consumed = CountingInput::new(
        BufferedInput::with_capacity(Cursor::new(b"abcdef"), 4));
    consumed.read_exactly(&mut [0; 1])?;
    assert_eq!(consumed.items_read(), 1);

    let mut fetched = BufferedInput::with_capacity(
        CountingInput::new(Cursor::new(b"abcdef")), 4);
    fetched.read_exactly(&mut [0; 1])?;
    assert_eq!(fetched.inner().items_read(), 4);

    let mut limited = LimitInput::new(Cursor::new(b"abc"), 2);
    assert_eq!(limited.read_fully(&mut [0; 4])?, 2);
    assert_eq!(limited.remaining(), 0);
    assert_eq!(limited.read(&mut [0; 1])?, 0);
    assert_eq!(limited.into_inner().position(), 2);

    let mut output = LimitOutput::new(Vec::<u8>::new(), 2);
    assert_eq!(output.write_fully(b"abc").unwrap_err().kind(),
               ErrorKind::WriteZero);
    assert_eq!(output.into_inner(), b"ab");
    Ok(())
}
```

```text
处理函数 → CountingInput → BufferedInput → 来源
           统计消费量
处理函数 → BufferedInput → CountingInput → 来源
                           统计拉取量（含预读）
```

第 2 章将输入限量放在缓冲内侧，因此缓冲不能预读超过两条记录。若限量在缓冲外侧，虽然处理函数最多读两条，缓冲仍可能从来源多取记录。输出同理：限量位于缓冲外侧限制接纳量；位于内侧则可能在缓冲排空时才暴露额度不足。

`remaining() == 0` 不能证明底层恰好结束。业务若要求整个输入必须不超过上限，应使用第 9 章的完整输入验证方法。经 `inner_mut()` 绕过限量或计数的传输不会记账。

## 7. 校验和与内容摘要

### 基于 Hasher 的校验和

`ChecksumInput`、`ChecksumOutput` 接受调用方提供的 `std::hash::Hasher`，只处理 `u8`。它们将成功传输的字节前缀传给 hasher，`checksum()` 返回当前 `u64` 值。下面用 `DefaultHasher` 演示接口，不将其数值作为稳定的跨版本文件格式或密码学摘要：

<!-- example: checksum -->
```rust
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::io::{self, Cursor};
use qubit_io::{ChecksumInput, Input};

fn main() -> io::Result<()> {
    let mut input = ChecksumInput::new(Cursor::new(b"abc"), DefaultHasher::new());
    let mut bytes = [0; 3];
    input.read_exactly(&mut bytes)?;
    let mut expected = DefaultHasher::new();
    expected.write(&bytes);
    assert_eq!(input.checksum(), expected.finish());
    Ok(())
}
```

### 内容摘要

需要 SHA-256 等固定长度摘要时，在项目依赖中启用 `digest` 并添加兼容算法：

```toml
[dependencies]
qubit-io = { version = "0.19.0", features = ["digest"] }
sha2 = "0.11"
```

`DigestInput`、`DigestOutput` 接受 `digest::Digest` 实现；当前接口版本为 `digest 0.11`，基于 0.10 的算法不能直接混用。以下独立程序计算整个剩余输入的指纹，再验证输出摘要：

<!-- example: digest -->
```rust
use std::io::{self, Cursor};
use qubit_io::{DigestInput, DigestOutput, Output};
use sha2::{Digest, Sha256};

fn main() -> io::Result<()> {
    let mut input = Cursor::new(b"hello world");
    let fingerprint = DigestInput::<_, Sha256>::digest_all(&mut input)?;
    assert_eq!(fingerprint, Sha256::digest(b"hello world"));

    let mut output = DigestOutput::new(Vec::<u8>::new(), Sha256::new());
    output.write_fully(b"hello ")?;
    assert_eq!(output.digest(), Sha256::digest(b"hello "));
    output.write_fully(b"world")?;
    output.flush()?;
    let (bytes, written_digest) = output.finalize();
    assert_eq!(bytes, b"hello world");
    assert_eq!(written_digest, fingerprint);
    Ok(())
}
```

选择方法时先确定需要覆盖哪些数据：

| 方法 | 行为 |
| --- | --- |
| `DigestInput::<_, D>::digest_all(&mut input)` | 创建新算法状态，从当前位置读取到 EOF；输入所有权保留在调用方 |
| `new(inner, hasher)` | 保留传入算法的初始状态，随正常读写累计 |
| `digest()` | 克隆并终结状态副本，得到快照后仍可继续传输；要求 `D: Clone` |
| `finalize()` | 消费包装器，返回 `(inner, digest)` |
| `into_parts()` | 返回 `(inner, hasher)`，供调用方继续计算 |

`digest_all` 使用固定 16 KiB 读缓冲，不将整个输入载入内存，也无须先根据 `is_buffered()` 再包一层。要计算完整原始输入的指纹，应从输入起点调用；读取失败不返回部分摘要，也不回退已消费的位置。

校验和与摘要包装器只累计成功返回的字节前缀；错误、零传输和异步 `Pending` 不增加当前操作的统计。`inner_mut()` 绕过跟踪，`hasher_mut()` 直接改变算法状态。

`finalize()`、`into_parts()` 不读到 EOF、不刷新也不关闭。输出摘要可能已经覆盖内部缓冲接纳、但尚未交付的字节，应先检查显式刷新或关闭结果。刷新失败不会撤回摘要。

包装顺序沿用第 6 章的规则：`DigestInput<BufferedInput<_>, _>` 记录消费量；缓冲包在摘要外侧可能提前计入预读字节。同步 seek 会转发到底层，但不重置摘要或校验和；重复读取、覆盖写入都按传输顺序累计，结果不自动等于最终文件内容的摘要。

## 8. 镜像输入与输出

第 2 章若需要两路收入输出，可将原来的 `Collected(Vec::new())` 替换为 `TeeOutput::new(Collected(Vec::new()), Collected(Vec::new()))`，并导入 `TeeOutput`；其余缓冲与计数层保持原位置。结果检查时将接收端用 `into_parts()` 拆成两个 `Collected`，分别检查记录。两路实际的业务接收端由应用提供。

`TeeInput` 先读取来源，再把成功读取的前缀写入分支；`TeeOutput` 先写主路，再将主路接纳的前缀完整写入分支。它们支持非字节元素，但有序执行，不提供原子性。

下面的独立字节示例同时展示正常镜像与分支额度不足：

<!-- example: tee -->
```rust
use std::io::{self, ErrorKind};
use qubit_io::{LimitOutput, Output, TeeOutput};

fn main() -> io::Result<()> {
    let mut success = TeeOutput::new(Vec::<u8>::new(), Vec::<u8>::new());
    success.write_fully(b"abc")?;
    success.flush()?;
    let (primary, branch) = success.into_parts();
    assert_eq!(primary, branch);

    let mut failed = TeeOutput::new(Vec::<u8>::new(),
                                   LimitOutput::new(Vec::<u8>::new(), 2));
    let error = failed.write_fully(b"abc").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::WriteZero);
    let (primary, branch) = failed.into_parts();
    assert_eq!(primary, b"abc");
    assert_eq!(branch.into_inner(), b"ab");
    // The primary has already accepted abc. Do not resend the entire batch.
    Ok(())
}
```

分支失败时，`TeeInput` 的来源和调用方目标切片可能已经推进或改变；`TeeOutput` 的主路可能已经接收整批，而分支只有一部分。应用应停止常规处理，检查两路状态，依据自身幂等或补偿机制处理，不能把错误当作“完全没有传输”而盲目重试。外侧计数器也未必反映这些副作用。

`TeeOutput::flush` 先刷新主路，主路成功后才刷新分支。`into_parts()` 不刷新；`TeeInput` 的分支是输出端，需要调用方自行刷新，例如经 `branch_mut()`。经 `inner_mut()` 或 `branch_mut()` 直接传输会使两路分离。

seek 也要选对语义：普通 `TeeInput` 仅定位来源；`TeeInput::with_sync_branch_seek` 返回 `SyncSeekTeeInput`，定位来源后再对齐分支。`TeeOutput` 同样先定位主路，再对齐分支。第二路定位失败不回滚第一路。这些类型目前没有对应的异步 tee 包装器。

## 9. 定位、复制与比较

### 定位和恢复位置

`Seekable` 以关联类型 `Unit` 为位置单位。`SeekableInput`、`SeekableOutput` 组合流与匹配单位的定位能力，不额外实现存储。标准 `Seek` 接入时单位为 `u8`；对象流的单位可以是记录，不能把传入的 `SeekFrom` 一概理解成字节偏移。

`PositionGuard` 记录创建时的位置；显式 `restore()` 可以检查恢复错误，`dismiss()` 放弃自动恢复。析构恢复失败不会报错或记录日志。缓冲输入定位会按逻辑位置处理未读窗口；缓冲输出的一般定位会先排空待写数据，`Current(0)` 查询则不刷新。

下面的独立程序先临时读取开头并恢复位置，再验证泛型有界复制的超限行为：

<!-- example: seek -->
```rust
use std::io::{self, Cursor, ErrorKind, SeekFrom};
use qubit_io::{Input, PositionGuard, Seekable, Streams};

fn main() -> io::Result<()> {
    let mut source = Cursor::new(b"abcdef");
    source.seek_to(SeekFrom::Start(2))?;
    {
        let mut guard = PositionGuard::new(&mut source)?;
        guard.inner_mut().seek_to(SeekFrom::Start(0))?;
        let mut magic = [0; 2];
        guard.inner_mut().read_exactly(&mut magic)?;
        assert_eq!(&magic, b"ab");
        guard.restore()?;
    }
    assert_eq!(source.position(), 2);

    let mut input = Cursor::new(b"abc");
    let mut output = Vec::new();
    let error = Streams::copy_input_to_output_end_limited(
        &mut input, &mut output, 2).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidData);
    assert!(output.is_empty());
    assert_eq!(input.position(), 3);
    Ok(())
}
```

### 选择复制策略

`Streams` 是工具命名空间，通过关联函数调用。下面的方法不自动关闭或刷新输出，成功后仍由应用检查 `flush`。

| 任务 | 泛型元素流 | 标准字节流 |
| --- | --- | --- |
| 复制到 EOF | `copy_input_to_output` | `copy` |
| 最多复制 N 个元素或字节 | `copy_input_to_output_at_most` | `copy_at_most` |
| 要求整个剩余输入不超过 N | `copy_input_to_output_end_limited` | `copy_to_end_limited` |

泛型复制要求 `Clone + Default`。两种完整输入限长方法都可能消费一个额外元素来检测超限，但输出行为不同：泛型版本先收集，超限或读入阶段失败时不写输出，临时内存可增长到上限对应的元素规模；确认 EOF 后写出仍可能部分失败。标准字节版本边读边写，超限时输出可能已有 N 字节。第 6 章的 `LimitInput` 只截断可见范围，不提供这种完整输入验证。

`Streams::content_eq`、`compare_content` 比较标准字节流从当前位置开始的剩余内容，后者返回字典序。比较会推进位置，可能读取到首个差异之后的块边界；需要保留位置时使用位置守卫并检查恢复结果。

### 标准 I/O 扩展

直接处理标准字节流时，从 `qubit_io::std_io::ext` 导入所需扩展 trait：

| Trait | 常用操作 |
| --- | --- |
| `ReadExt` | `read_exact_array`、`read_exact_vec_limited`、`read_to_end_limited`、`read_to_string_limited`、复制和丢弃 |
| `BufReadExt` | `read_until_limited`、`read_line_limited`、`discard_until_limited` |
| `ReadSeekExt` | `peek_exact_or_eof`、`read_exact_or_eof_at` |
| `WriteSeekExt` | `write_all_at_preserving_position` |
| `SeekExt` | `stream_size` |
| `WriteExt` | 面向已证明合法范围的低层写入方法 |

下面的独立程序观察超限后的位置和输出，不假设错误意味着回滚：

<!-- example: limited -->
```rust
use std::io::{self, Cursor, ErrorKind};
use qubit_io::std_io::ext::ReadExt;

fn main() -> io::Result<()> {
    let mut source = Cursor::new(b"abc");
    assert_eq!(source.read_to_end_limited(2).unwrap_err().kind(),
               ErrorKind::InvalidData);
    assert_eq!(source.position(), 3);

    let mut source = Cursor::new(b"abc");
    let mut destination = Vec::new();
    assert_eq!(source.copy_to_end_limited(&mut destination, 2)
                     .unwrap_err().kind(), ErrorKind::InvalidData);
    assert_eq!(destination, b"ab");
    assert_eq!(source.position(), 3);
    Ok(())
}
```

有界字符串方法要求有效 UTF-8；行和分隔符读取的上限按字节计，分隔符读入结果时也计入上限。`*_into` 是否恢复目标集合、输入多消费多少，以及定位恢复失败时返回什么，都按具体方法的 API 契约处理；例如 `read_to_end_limited_into` 失败会恢复目标向量原长度，却不恢复来源位置。处理不可信数据前，应先确定需要“截断成功”还是“超限拒绝”。

## 10. 使用异步泛型流

### 同一元素模型，独立异步驱动

`AsyncInput<Item = T>`、`AsyncOutput<Item = T>` 保留泛型元素模型，通过 `Pin`、`Poll` 表达就绪状态，不选择执行器。它们不是把同步方法自动放进后台；同步驱动与异步驱动需要分别实现。

当前提供 `AsyncBufferedInput`、`AsyncBufferedOutput`，以及异步 limit、counting、checksum、digest 包装器。缓冲仍要求 `Clone + Default`，校验和与摘要仍只用于字节。同步 seek 和 tee 不能假定存在异步对应物。

### 接入 Tokio

在独立项目的 `Cargo.toml` 中使用以下依赖。Tokio 的运行时和宏由应用显式启用，Qubit IO 的 `tokio` feature 只负责 I/O 适配。

```toml
[dependencies]
qubit-io = { version = "0.19.0", features = ["tokio"] }
tokio = { version = "1", features = ["rt", "macros", "io-util"] }
```

将下列完整程序保存为 `src/main.rs`，执行 `cargo run`。它把三个字节传入缓冲输出，显式刷新、关闭并检查结果；最后演示反向适配：

<!-- example: async -->
```rust
use std::io;
use qubit_io::{AsyncBufferedOutput, AsyncClose, AsyncCountingInput,
               AsyncInput, AsyncLimitInput, AsyncOutput,
               TokioAsyncRead, TokioInput, TokioOutput};

async fn transfer<I, O>(input: &mut I, output: &mut O) -> io::Result<()>
where
    I: AsyncInput<Item = u8> + Unpin,
    O: AsyncOutput<Item = u8> + Unpin,
{
    let mut batch = [0; 16];
    loop {
        let n = input.read_async(&mut batch).await?;
        if n == 0 { break; }
        output.write_fully_async(&batch[..n]).await?;
    }
    output.flush_async().await
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let source = TokioInput::new(&b"abc"[..]);
    let mut input = AsyncCountingInput::new(AsyncLimitInput::new(source, 3));
    let mut output = AsyncBufferedOutput::with_capacity(
        TokioOutput::new(Vec::<u8>::new()), 8);
    transfer(&mut input, &mut output).await?;
    assert_eq!(input.items_read(), 3);
    output.close_async().await?;
    let (sink, pending) = output.into_parts();
    assert!(pending.is_empty());
    assert_eq!(sink.into_inner(), b"abc");

    let mut reader = TokioAsyncRead::new(TokioInput::new(&b"xyz"[..]));
    let mut bytes = [0; 3];
    tokio::io::AsyncReadExt::read_exact(&mut reader, &mut bytes).await?;
    assert_eq!(&bytes, b"xyz");
    Ok(())
}
```

`transfer` 的算法仅依赖 Qubit trait，使用其他生态时可原样保留。此处关闭内存输出只是演示能力调用；实际 socket 的关闭行为由底层适配器决定。`AsyncClose` 是独立的可选能力，不能只凭 `AsyncOutput` 就调用关闭。flush 成功不等于 transport shutdown，更不证明远端业务处理完成或数据已持久化。

### 接入 futures-io

在另一个独立项目中使用：

```toml
[dependencies]
qubit-io = { version = "0.19.0", features = ["futures-io"] }
futures = "0.3"
```

以下完整程序用 `futures` 的内存流和执行器演示双向接入；执行 `cargo run` 后断言通过，无终端输出。执行器由应用选择，库本身不要求 `futures::executor`。

<!-- example: futures -->
```rust
use std::io;
use qubit_io::{AsyncInput, AsyncOutput, FuturesAsyncRead,
               FuturesInput, FuturesOutput};

fn main() -> io::Result<()> {
    futures::executor::block_on(async {
        let mut input = FuturesInput::new(futures::io::Cursor::new(b"abc"));
        let mut output = FuturesOutput::new(futures::io::Cursor::new(Vec::new()));
        let mut bytes = [0; 3];
        input.read_exactly_async(&mut bytes).await?;
        output.write_fully_async(&bytes).await?;
        output.flush_async().await?;
        assert_eq!(output.into_inner().into_inner(), b"abc");

        let mut reader = FuturesAsyncRead::new(
            FuturesInput::new(futures::io::Cursor::new(b"xyz")));
        futures::io::AsyncReadExt::read_exact(&mut reader, &mut bytes).await?;
        assert_eq!(&bytes, b"xyz");
        Ok(())
    })
}
```

适配器方向如下；所有生态桥接都只用于字节：

| 已有接口 | 目标接口 | Tokio | futures-io |
| --- | --- | --- | --- |
| 生态异步 reader | Qubit `AsyncInput` | `TokioInput` | `FuturesInput` |
| 生态异步 writer | Qubit `AsyncOutput` | `TokioOutput` | `FuturesOutput` |
| Qubit `AsyncInput` | 生态 `AsyncRead` | `TokioAsyncRead` | `FuturesAsyncRead` |
| Qubit `AsyncOutput` | 生态 `AsyncWrite` | `TokioAsyncWrite` | `FuturesAsyncWrite` |

反向 writer 还需要内部实现 `AsyncClose`，才能满足生态 writer 的 shutdown/close 接口；仅有 `AsyncOutput` 不够。

### 固定值、取消与进度

普通 `read_async`、`write_fully_async` 等便利方法要求 `Unpin`。已固定的 `!Unpin` 流或 trait object 使用 `PinnedAsyncInputExt`、`PinnedAsyncOutputExt`。下面的独立程序沿用 Tokio 依赖，通过 pinned trait object 完成读取；不要求 trait object 为 `Unpin`：

<!-- example: pinned -->
```rust
use std::io;
use std::pin::Pin;
use qubit_io::{AsyncInput, PinnedAsyncInputExt, TokioInput};

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let mut input: Pin<Box<dyn AsyncInput<Item = u8>>> =
        Box::pin(TokioInput::new(&b"abc"[..]));
    let mut bytes = [0; 3];
    input.as_mut().read_exactly_async(&mut bytes).await?;
    assert_eq!(&bytes, b"abc");
    Ok(())
}
```

`InputRef`/`OutputRef` 用于同步借用；拥有的异步 boxed 边界可使用 `BoxAsyncInput`、`BoxAsyncOutput`。已经固定的值应继续保持其固定状态，不应为取得所有权而移动底层 `!Unpin` 值。

一次底层 poll 返回 `Pending` 或错误时不能传输新元素；但 `ReadFullyFuture`、`ReadExactFuture`、`WriteFullyFuture` 等聚合 future 可能已在此前的底层调用中取得进度。需要取消并恢复时，先保留具名 future，在丢弃前读取 `items_read()` 或 `items_written()`；读取进度不是协议解析状态，应用仍要维护消息或批次状态，不能从头重试并假设没有副作用。

异步摘要还提供 `AsyncDigestInput::<_, D>::digest_all(input.as_mut())`，接收 `Pin<&mut I>`，返回 `DigestAllFuture`。它在多次 poll 间保留状态，每次 poll 最多进行 64 次成功读取后让出执行机会；取消会丢弃部分摘要，输入位置保留。完成后再次 poll 会 panic。异步摘要包装器支持底层 `!Unpin`；获取快照可通过共享引用，消费型 `finalize()` 则需要包装器所有权。

异步输出必须显式 `flush_async`，若具备关闭能力则按应用生命周期调用 `close_async`；`AsyncBufferedOutput` 在关闭时先排空缓冲，再关闭底层。错误后保留对象，检查剩余数据；析构不能代替异步 I/O。

## 11. 实现自己的输入输出流

### 从同步适配器开始

第 2 章给出了完整的 `Records<T>`、`Collected<T>` 实现。它们使用 `T: Clone` 是自身的复制策略，不是核心 trait 的统一要求。实现输入需要 `type Item` 与 `read_unchecked`；实现输出需要 `type Item`、`write_unchecked` 和 `flush`。

底层方法声明为 `unsafe`，因为调用者负责证明 `index..index + count` 在切片内且加法不溢出。实现者仍须遵守该范围、返回不超过 `count` 的数量，并正确推进自身状态。示例采用安全切片操作，也必须保留 trait 要求的 `unsafe fn` 签名。上层算法应优先调用安全的 `read`、`write` 等方法。

针对自己的流检查这些行为：空请求、尾批不足、EOF、短写、零进展、`Interrupted`、错误后的可观察状态，以及自定义对象的克隆语义。安全入口会将超出请求数量的报告转为 `InvalidData`，但这不能补救实现中已经发生的越界内存访问。

### 接入业务模块

将业务处理函数保留为 `I: Input<Item = Sale>`、`O: Output<Item = CategoryRevenue>` 边界，由应用装配。第 2 章的 `?Sized` 允许借用 trait object；需要把所有权交给另一层时选择 `BoxInput`、`BoxOutput`。不要为了兼容字节 API 而给对象流直接实现标准 `Read`；应使用独立编码层和新类型表达不同元素语义。

### 实现异步流前确认就绪机制

异步输入实现 `poll_read_unchecked`，输出实现 `poll_write_unchecked` 和 `poll_flush`；真实关闭能力另外实现 `AsyncClose::poll_close`。除了同步的范围与数量要求，还必须遵守：

- 空读写请求立即返回 `Ready(Ok(0))`。
- 尚不能推进时，先注册当前 waker 的通知机制，再返回 `Pending`；不能只返回 `Pending` 而无人唤醒。
- 单次 poll 的 `Pending` 和错误不传输元素，部分成功应返回 `Ready(Ok(n))`。
- `WouldBlock` 应转成就绪等待，`Interrupted` 在实现内部处理，不能穿过异步边界；安全入口和相关包装器会把这两类契约违规错误转为 `InvalidData`，保留原错误作为原因。
- 尊重 pinning，不移动已固定的内部值。支持 `!Unpin` 时必须正确处理字段投影。

内存就绪流不需要制造 `Pending`；网络流应使用底层生态的就绪机制，优先采用已有适配器。同步阻塞 I/O 不能仅因为包了一层 `AsyncInput` 就变成非阻塞操作。

## 12. 排障与能力速查

| 症状 | 检查与处理 |
| --- | --- |
| 对象流不能使用缓冲 | 检查 `Clone + Default`；`String` 本身满足条件，核心流无需这些统一约束 |
| `read`、`write`、`flush` 调用歧义 | 同时导入了标准和 Qubit trait；使用 `Input::read`、`Output::write`、`std::io::Write::flush` 等限定调用 |
| 自定义实现报冲突 | 具体类型可能已有标准 I/O blanket impl；使用新类型区分字节流和对象流 |
| 额度耗尽后误以为原始输入结束 | 检查 `remaining()`，区分限量产生的 EOF 与底层 EOF；完整验证使用 end-limited 方法 |
| 输出出现 `WriteZero` | 检查限额和底层零进展；已写前缀不回滚，勿直接重发整批 |
| 精确读取报 `UnexpectedEof` | 检查协议截断、错误的预期长度或外层额度；已读取部分不会恢复 |
| 计数或摘要比业务消费量大 | 检查是否位于缓冲内侧，或发生重复读取；根据观测需求调整顺序 |
| 返回错误但两路输出不一致 | tee 可能已推进主路；拆开检查并按应用幂等或补偿策略处理 |
| 写入成功却看不到最终输出 | 显式检查 flush/close；缓冲接纳、传输完成与持久化是不同阶段 |
| 异步 `InvalidData` 带有 WouldBlock/Interrupted 原因 | 底层实现违反异步契约；修正就绪或重试处理，不把它当普通 EOF |
| 异步操作一直不再执行 | 检查返回 `Pending` 前是否注册了当前 waker，以及应用是否持续驱动执行器 |
| SHA 算法类型不兼容 | 检查是否使用兼容 `digest 0.11` 的算法版本 |

| 能力 | 元素与约束 | Feature / 范围 |
| --- | --- | --- |
| `Input`、`Output` | 泛型元素，无统一克隆约束 | 默认 |
| `AsyncInput`、`AsyncOutput` | 泛型元素，poll 接口 | 默认；应用选择执行器 |
| 同步与异步缓冲 | `Clone + Default` | 默认 |
| 同步与异步限量、计数 | 泛型元素 | 默认 |
| `TeeInput`、`TeeOutput`、`SyncSeekTeeInput` | 泛型元素；定位按底层能力 | 当前为同步 |
| 同步与异步 checksum | `u8` + `Hasher` | 默认 |
| 同步与异步 digest | `u8` + `digest::Digest` | `digest` |
| 标准桥接与扩展 | 字节流 | 默认 |
| Tokio 双向桥接 | 异步字节流 | `tokio` |
| futures-io 双向桥接 | 异步字节流 | `futures-io` |
| 泛型复制工具 | `Clone + Default` | 同步 `Streams` |

接下来可查阅[当前版本 API 文档](https://docs.rs/qubit-io/0.19.0/qubit_io/)，或回到[项目介绍](../README.zh_CN.md)。本手册的英文版本见 [English user guide](user_guide.md)。
