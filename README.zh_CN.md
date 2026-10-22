# Qubit IO

[![Rust CI](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml/badge.svg)](https://github.com/qubit-ltd/rs-io/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/endpoint?url=https://qubit-ltd.github.io/rs-io/coverage-badge.json)](https://qubit-ltd.github.io/rs-io/coverage/)
[![Crates.io](https://img.shields.io/crates/v/qubit-io.svg?color=blue)](https://crates.io/crates/qubit-io)
[![Rust](https://img.shields.io/badge/rust-1.94+-blue.svg?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![English Document](https://img.shields.io/badge/Document-English-blue.svg)](README.md)

Qubit IO 为 Rust 提供了一套以 `Input` 和 `Output` 为核心的泛型输入输出流系统。
流通过关联类型 `Item` 指定传输的元素，既可以传输字节，也可以传输字符、业务记录或其他
Rust 类型。库和应用开发者可以直接围绕这些元素编写处理算法，无需在每个处理环节之间
先将数据序列化为字节。

标准库的 `Read`、`Write` 可以直接接入这套系统。围绕统一的流接口，Qubit IO 提供
缓冲、限量、计数、校验和、内容摘要和分流等可组合包装器，并通过 `AsyncInput`、
`AsyncOutput` 将这一模型扩展到异步操作。异步核心不绑定运行时，可按需适配 Tokio
和 `futures-io`。

## 核心模型：以元素为单位的输入输出流

| 接口 | 职责 | 元素类型 |
| --- | --- | --- |
| `Input<Item = T>` | 同步输入 | 实现所支持的任意 `T` |
| `Output<Item = T>` | 同步输出 | 实现所支持的任意 `T` |
| `AsyncInput<Item = T>` | 异步输入 | 实现所支持的任意 `T` |
| `AsyncOutput<Item = T>` | 异步输出 | 实现所支持的任意 `T` |

输入流将元素读入切片，输出流从切片写出元素。当 `Item = Sale` 时，读取数量 10
表示十条销售记录；当 `Item = u8` 时，才表示十个字节。限量和计数也使用相同的元素单位。

核心接口不要求 `Item: Clone + Default`，具体的适配器和包装器按自身需要施加约束。
例如，通用缓冲要求元素满足 `Clone + Default`，校验和及内容摘要包装器则只处理字节。

## 安装

要求 Rust 1.94 或更高版本。默认不启用异步生态适配器及内容摘要依赖。

```toml
[dependencies]
qubit-io = "0.19"
```

## 快速开始：处理业务记录流

假设一条销售数据流水线需要输出各分类的收入记录。如果每经过一个处理环节都将记录
编码为字节再解码，会给本来只需要处理业务对象的算法增加序列化环节。使用泛型流后，
映射函数直接接收销售记录流，输出收入记录流；调用方负责选择数据来源、去向和传输策略：

```text
销售记录源 → 限量（2 条）→ 缓冲 → 计数 → 映射函数 → 计数 → 收入记录接收端
```

下面是完整的可运行程序。内存中的记录源和接收端由示例自行实现；流接口、缓冲、限量、
计数以及检查边界的读写方法由 Qubit IO 提供：

```rust
use std::io;
use qubit_io::{
    BufferedInput, CountingInput, CountingOutput, Input, LimitInput, Output,
};

#[derive(Clone, Debug, Default, PartialEq)]
struct Sale {
    category: u16,
    cents: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Revenue {
    category: u16,
    cents: u64,
}

// 应用自行实现的适配器，直接传输完整记录。
struct RecordInput<T> {
    records: Vec<T>,
    position: usize,
}

impl<T: Clone> Input for RecordInput<T> {
    type Item = T;

    // 安全约束：调用方必须保证目标切片范围有效。
    unsafe fn read_unchecked(
        &mut self, output: &mut [T], index: usize, count: usize,
    ) -> io::Result<usize> {
        let n = count.min(self.records.len() - self.position);
        output[index..index + n]
            .clone_from_slice(&self.records[self.position..self.position + n]);
        self.position += n;
        Ok(n)
    }
}

struct RecordOutput<T>(Vec<T>);

impl<T: Clone> Output for RecordOutput<T> {
    type Item = T;

    // 安全约束：调用方必须保证源切片范围有效。
    unsafe fn write_unchecked(
        &mut self, input: &[T], index: usize, count: usize,
    ) -> io::Result<usize> {
        self.0.extend_from_slice(&input[index..index + count]);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn map_sales<I, O>(input: &mut I, output: &mut O) -> io::Result<()>
where
    I: Input<Item = Sale>,
    O: Output<Item = Revenue>,
{
    let mut batch = [Sale::default(); 1];
    while input.read(&mut batch)? != 0 {
        let sale = &batch[0];
        output.write_fully(&[Revenue {
            category: sale.category,
            cents: sale.cents,
        }])?;
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let source = RecordInput {
        records: vec![
            Sale { category: 7, cents: 300 },
            Sale { category: 7, cents: 500 },
            Sale { category: 9, cents: 900 },
        ],
        position: 0,
    };
    let limited = LimitInput::new(source, 2);
    let buffered = BufferedInput::with_capacity(limited, 2);
    let mut input = CountingInput::new(buffered);
    let mut output = CountingOutput::new(RecordOutput(Vec::new()));

    map_sales(&mut input, &mut output)?;
    output.flush()?;
    assert_eq!(input.items_read(), 2);
    assert_eq!(output.items_written(), 2);
    assert_eq!(output.into_inner().0, vec![
        Revenue { category: 7, cents: 300 },
        Revenue { category: 7, cents: 500 },
    ]);
    Ok(())
}
```

程序检查了输出记录和传输数量：限量包装器只允许读取前两条销售记录，第三条没有进入
处理流程。整个流程没有将记录序列化为字节。映射函数只调用安全方法；适配器负责实现
底层的索引读写契约，其中 `unsafe` 方法要求调用方保证切片范围有效。

真实应用需要为自己的记录来源和接收端提供适配器，Qubit IO 本身不提供记录存储引擎。
现有的[业务记录示例](examples/typed_records.rs)还展示了如何通过 `TeeOutput`
将同一批输出记录复制到两个接收端。

## 用更简洁的约束处理字节流

API 只处理字节时，可以用 `ByteInput`、`ByteOutput` 分别简写
`Input<Item = u8>`、`Output<Item = u8>` 约束，读写方法仍由原来的接口提供：

```rust
use std::io::{self, Cursor};
use qubit_io::{ByteInput, ByteOutput};

fn input() -> impl ByteInput {
    Cursor::new(b"QBIT")
}

fn copy_magic<I: ByteInput + ?Sized, O: ByteOutput + ?Sized>(
    input: &mut I, output: &mut O,
) -> io::Result<()> {
    let mut magic = [0; 4];
    input.read_exactly(&mut magic)?;
    output.write_fully(&magic)
}

fn main() -> io::Result<()> {
    let mut bytes = Vec::new();
    copy_magic(&mut input(), &mut bytes)?;
    assert_eq!(bytes, b"QBIT");
    Ok(())
}
```

已有的字节流会自动满足这两个 trait，包括标准库 `Read`、`Write` 的实现。
实现自定义流时仍只需实现 `Input` 或 `Output`，无需再实现字节 trait。
返回值可写成 `impl ByteInput`／`impl ByteOutput` 来隐藏具体类型；需要动态分发时，
可通过引用或 box 使用 `dyn ByteInput`／`dyn ByteOutput`。
借用输出和标准库转换的完整用法见[用户手册](doc/user_guide.zh_CN.md#4-连接标准库字节流)。

## 兼容标准库 Read 和 Write

所有实现 `std::io::Read` 的类型都已自动实现 `Input<Item = u8>`；所有实现
`std::io::Write` 的类型都已自动实现 `Output<Item = u8>`。因此，文件、套接字、
`Cursor`、`Vec<u8>` 等类型可以直接接入，无需再套一层输入输出适配器。
反过来，`StdRead` 和 `StdWrite` 可以将 Qubit 字节流交给使用标准库接口的代码：

```rust
use std::io::{self, Cursor, Read};
use qubit_io::{CountingInput, Input, Output, StdRead};

fn read_magic<I: Input<Item = u8>>(input: &mut I) -> io::Result<[u8; 4]> {
    let mut magic = [0; 4];
    input.read_exactly(&mut magic)?;
    Ok(magic)
}

fn main() -> io::Result<()> {
    let mut bytes = Vec::new();
    bytes.write_fully(b"QBITpayload")?;
    let mut input = CountingInput::new(Cursor::new(bytes));
    assert_eq!(read_magic(&mut input)?, *b"QBIT");
    assert_eq!(input.bytes_read(), 4);

    // 将剩余的 Qubit 输入交给使用 std::io::Read 的代码。
    let mut reader = StdRead::new(input);
    let mut rest = String::new();
    reader.read_to_string(&mut rest)?;
    assert_eq!(rest, "payload");
    assert_eq!(reader.get_ref().bytes_read(), 11);
    Ok(())
}
```

需要借用已有流（包括 trait 对象）时，可以先使用 `InputRef` 或 `OutputRef` 再转换。
标准库兼容针对字节流；业务记录如何编码为字节，仍由相应的编码层决定。

## 组合流包装器

包装后的流仍实现相应的流接口，因此业务算法不需要知道外部组合了哪些策略。
快速开始中的 `CountingInput<BufferedInput<LimitInput<_>>>` 仍然是
`Input<Item = Sale>`。

| 能力 | 同步类型 | 异步类型 | 元素约束 |
| --- | --- | --- | --- |
| 缓冲 | `BufferedInput`、`BufferedOutput` | `AsyncBufferedInput`、`AsyncBufferedOutput` | `Clone + Default` |
| 限量 | `LimitInput`、`LimitOutput` | `AsyncLimitInput`、`AsyncLimitOutput` | 泛型元素 |
| 计数 | `CountingInput`、`CountingOutput` | `AsyncCountingInput`、`AsyncCountingOutput` | 泛型元素 |
| 校验和 | `ChecksumInput`、`ChecksumOutput` | `AsyncChecksumInput`、`AsyncChecksumOutput` | `u8`；调用方提供 `std::hash::Hasher` |
| 内容摘要 | `DigestInput`、`DigestOutput` | `AsyncDigestInput`、`AsyncDigestOutput` | `u8`；启用 `digest`，调用方提供算法 |
| 分流 | `TeeInput`、`TeeOutput`、`SyncSeekTeeInput` | — | 泛型元素；同步定位要求源流和分支流均可定位 |

包装顺序决定观察到的数据范围。计数器放在输入缓冲外侧时，统计算法实际消费的元素；
放在缓冲内侧时，还会计入预取的元素。限量包装器约束自身暴露或接收的元素数量，
应用仍须执行自己的校验，例如分配内存前检查消息声明的长度，参见
[长度前缀帧示例](examples/bounded_frame.rs)。

使用输出缓冲时，应显式刷新并检查结果。同步缓冲在析构时只会尽力写出待发送数据；
异步缓冲在析构时无法执行异步 I/O。[用户指南](doc/user_guide.zh_CN.md)
进一步说明包装顺序、部分读写以及未读数据和待发送数据的恢复方式。

### 内容摘要

需要计算 SHA-256 等内容指纹时，启用 `digest`，并添加兼容的具体算法：

```toml
[dependencies]
qubit-io = { version = "0.19", features = ["digest"] }
sha2 = "0.11"
```

```rust
use std::io::{self, Cursor};
use qubit_io::DigestInput;
use sha2::{Digest, Sha256};

fn main() -> io::Result<()> {
    let mut input = Cursor::new(b"hello world");
    let fingerprint = DigestInput::<_, Sha256>::digest_all(&mut input)?;
    assert_eq!(fingerprint, Sha256::digest(b"hello world"));
    Ok(())
}
```

`digest_all` 使用固定大小的缓冲，从当前位置读到 EOF。只有从输入起点开始读取，
得到的才是完整原始输入的指纹。需要边传输边计算时，可使用 `DigestInput::new`
或 `DigestOutput::new` 包装字节流。算法需要兼容 `digest` 0.11 接口，具体选择由应用决定。

摘要包装器按成功传输的顺序更新摘要，定位后重复读取或覆盖写入的字节也会计入，
因此结果不自动等于最终文件内容的指纹。`finalize()` 不会继续读取、刷新或关闭流。
详细说明见[内容摘要指南](doc/user_guide.zh_CN.md#内容摘要)。

## 异步输入输出流

`AsyncInput` 和 `AsyncOutput` 沿用相同的泛型元素模型，通过 `Pin` 和 `Poll`
表达就绪状态。通常可以直接使用 `read_exactly_async`、`write_fully_async` 等便捷方法；
固定位置的 `!Unpin` 值及 trait 对象则使用 `PinnedAsyncInputExt`、`PinnedAsyncOutputExt`。

异步算法只依赖泛型流接口，调用方通过适配器接入 Tokio 或 `futures-io`。
下面是使用 Tokio 运行通用异步读取函数的完整程序：

```toml
[dependencies]
qubit-io = { version = "0.19", features = ["tokio"] }
tokio = { version = "1.54", features = ["macros", "rt"] }
```

```rust
use std::io;
use qubit_io::{AsyncCountingInput, AsyncInput, TokioInput};

async fn read_magic<I>(input: &mut I) -> io::Result<[u8; 4]>
where
    I: AsyncInput<Item = u8> + Unpin,
{
    let mut magic = [0; 4];
    input.read_exactly_async(&mut magic).await?;
    Ok(magic)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let source = TokioInput::new(&b"QBIT"[..]);
    let mut input = AsyncCountingInput::new(source);
    assert_eq!(read_magic(&mut input).await?, *b"QBIT");
    assert_eq!(input.bytes_read(), 4);
    Ok(())
}
```

读取函数依赖 `AsyncInput`，Tokio 由应用选择。使用 `futures-io` 的调用方可以传入
`FuturesInput`，复用同一个异步函数。同步算法与异步算法仍有各自的入口。

| 转换方向 | Tokio 适配器 | futures-io 适配器 |
| --- | --- | --- |
| 生态 `AsyncRead` → Qubit `AsyncInput<Item = u8>` | `TokioInput` | `FuturesInput` |
| 生态 `AsyncWrite` → Qubit `AsyncOutput<Item = u8>` | `TokioOutput` | `FuturesOutput` |
| Qubit `AsyncInput<Item = u8>` → 生态 `AsyncRead` | `TokioAsyncRead` | `FuturesAsyncRead` |
| Qubit `AsyncOutput<Item = u8>` → 生态 `AsyncWrite` | `TokioAsyncWrite` | `FuturesAsyncWrite` |

`AsyncClose` 单独表达正常关闭输出的能力，包括必要的缓冲写出和底层关闭操作。
刷新本身不代表关闭传输。[用户指南](doc/user_guide.zh_CN.md)说明了轮询、固定位置
以及取消操作时的行为。

## 其他流工具

| 工具 | 用途 |
| --- | --- |
| `Seekable`、`SeekableInput`、`SeekableOutput` | 同步定位，位置以流元素为单位 |
| `PositionGuard` | 恢复保存的位置；需要检查恢复错误时显式调用 `restore()` |
| `Streams` | 复制泛型元素流；复制、比较标准字节流及判断其内容是否相等 |
| `std_io::ext` | 为标准库 I/O 增加有上限的读取、行读取、复制及保留当前位置的操作 |
| `std_io` 组合接口 | 表达标准库能力组合，如 `ReadSeek`、`ReadWriteSeek` |
| `Buffer`、`EnsuredBufferedInput`、`EnsuredBufferedOutput` | 底层缓冲存储及按需添加的同步缓冲 |
| `BoxInput`、`BoxOutput`、`BoxAsyncInput`、`BoxAsyncOutput` | 装箱流适配器 |

## 适用场景与职责边界

当算法需要直接处理业务对象流、传输策略需要独立组合，或者异步库需要让调用方选择
运行时时，可以使用 Qubit IO。如果应用只处理普通字节流且只使用一个运行时，原生
I/O 接口可能已经足够。

本库负责流传输及相关工具。结构化二进制值和文本编码分别由
[qubit-io-binary](https://github.com/qubit-ltd/rs-io-binary/blob/main/README.zh_CN.md) 与
[qubit-io-text](https://github.com/qubit-ltd/rs-io-text/blob/main/README.zh_CN.md) 处理。路径、文件身份、元数据、
提交、放弃及持久化等语义由 [qubit-fs](https://github.com/qubit-ltd/rs-fs/blob/main/README.zh_CN.md) 等更高层的文件系统抽象负责。

## 可选功能与文档

| Feature | 启用内容 |
| --- | --- |
| 默认：无 | 同步及异步核心接口、缓冲、标准库集成和内容摘要以外的包装器 |
| `digest` | 同步及异步内容摘要包装器，以及读到 EOF 的摘要计算方法 |
| `tokio` | 与 Tokio 字节 I/O 的双向适配 |
| `futures-io` | 与 futures-io 字节 I/O 的双向适配 |

- [API 文档](https://docs.rs/qubit-io)：启用项目声明的全部 feature 构建。
- [用户指南](doc/user_guide.zh_CN.md) | [English user guide](doc/user_guide.md)
- [业务记录示例](examples/typed_records.rs) | [长度前缀帧示例](examples/bounded_frame.rs)
- [English README](README.md)

## 测试

```bash
# 使用默认 feature 集运行测试
cargo test

# 使用项目声明的全部 feature 运行测试
cargo test --all-features

# 运行项目 CI 检查
./.infra/bin/ci-check.sh

# 检查代码覆盖率
./.infra/bin/coverage.sh
```

## 许可证

Copyright (c) 2025 - 2026. Haixing Hu. All rights reserved.

本项目基于 Apache License 2.0 授权。完整许可证文本请参阅
[LICENSE](LICENSE)。

## 贡献

欢迎贡献。请遵循 Rust API 指南，及时更新公共 API 文档与测试，并在提交
Pull Request 前运行 `./.infra/bin/align-ci.sh` 格式化代码，运行 `./.infra/bin/ci-check.sh` 满足 CI 要求。

## 作者

**Haixing Hu** - *Qubit Co. Ltd.*

仓库地址：[https://github.com/qubit-ltd/rs-io](https://github.com/qubit-ltd/rs-io)
