# Qubit IO 用户手册

Qubit IO 是 Qubit Rust crate 家族中的 stream 和字节 I/O crate。它专注于
`std::io` trait、extension method、stream wrapper 和 codec helper。它有意不做本地
文件系统工具库。

如果需要本地文件系统相关能力，请参考
[qubit-local-fs](https://github.com/qubit-ltd/rs-local-fs)。

## 何时使用本 crate

当代码处理的是字节流，而不是文件系统路径时，适合使用 `qubit-io`。典型场景包括 parser、
binary codec、协议适配器、archive reader、内存 buffer、网络 stream，以及需要接受灵活
`Read` / `Write` / `Seek` 实现的 API。

适合的场景：

- 读取固定长度，但遇到 EOF 时需要知道实际读了多少字节。
- 从不可信输入中读取有界字节或有界文本。
- 只有在大小限制内到达 EOF 时才复制整个 stream。
- 不把全部内容加载进内存，也能比较 stream 内容。
- 使用显式字节序读写 binary scalar。
- 使用 LEB128 或 ZigZag 编码紧凑整数。
- 暴露类似 `dyn ReadSeek` 的 trait-object-friendly I/O 能力。
- 给 stream 加上计数、限制、tee、checksum 或位置恢复能力。

不适合的场景：

- 创建临时文件或临时目录。
- 递归复制或清理本地目录。
- 校验本地文件名或生成本地临时名。
- 对本地文件路径做 atomic replacement write。
- 抽象本地、FTP、对象存储或远程文件系统。

这些本地文件系统能力请参考
[qubit-local-fs](https://github.com/qubit-ltd/rs-local-fs)。

## 安装

```toml
[dependencies]
qubit-io = "0.2"
```

## 导入方式

具体 wrapper 和命名空间从 crate root 导入：

```rust
use qubit_io::{
    CountingReader,
    LimitWriter,
    Streams,
};
```

需要让标准库 I/O 值获得扩展方法时，显式导入 extension trait：

```rust
use qubit_io::{
    ReadExt,
    SeekExt,
    WriteSeekExt,
};
```

如果一个模块主要使用 extension trait 和组合 trait，可以导入 prelude：

```rust
use qubit_io::prelude::*;
```

prelude 有意不导入 wrapper 类型。这样可以让具体运行时行为在调用点保持明确。

## Object-Safe 组合 Trait

Rust 中直接把多个 trait 组合写成 trait object 并不总是足够顺手。`qubit-io` 为常用
`std::io` 能力集合提供具名 trait：

| Trait | Supertrait | 使用场景 |
| --- | --- | --- |
| `ReadSeek` | `Read + Seek` | 消费方需要可读取的随机访问输入 |
| `BufReadSeek` | `BufRead + Seek` | 消费方需要带缓冲的随机访问输入 |
| `ReadWrite` | `Read + Write` | stream 或 buffer 同时可读写 |
| `WriteSeek` | `Write + Seek` | 输出需要按绝对位置 patch |
| `ReadWriteSeek` | `Read + Write + Seek` | 输入输出共享一个随机访问对象 |

示例：

```rust
use std::io::{Read, SeekFrom};

use qubit_io::{ReadSeek, SeekExt};

fn read_header(input: &mut dyn ReadSeek) -> std::io::Result<Vec<u8>> {
    let size = input.stream_size()?;
    let mut header = vec![0; size.min(8) as usize];
    input.seek(SeekFrom::Start(0))?;
    input.read_exact(&mut header)?;
    Ok(header)
}
```

## Streams 命名空间

`Streams` 提供 generic stream 操作的 associated function。

### 复制全部内容

当你希望通过 Qubit IO 命名空间使用标准 `std::io::copy` 行为时，使用 `Streams::copy`：

```rust
use std::io::Cursor;

use qubit_io::Streams;

let mut input = Cursor::new(b"payload".to_vec());
let mut output = Vec::new();

let copied = Streams::copy(&mut input, &mut output)?;

assert_eq!(7, copied);
assert_eq!(b"payload", output.as_slice());
# Ok::<(), std::io::Error>(())
```

### 最多复制 N 字节

当调用方要控制最多消费多少数据时，使用 `copy_at_most`：

```rust
use std::io::Cursor;

use qubit_io::Streams;

let mut input = Cursor::new(b"abcdef".to_vec());
let mut output = Vec::new();

let copied = Streams::copy_at_most(&mut input, &mut output, 3)?;

assert_eq!(3, copied);
assert_eq!(b"abc", output.as_slice());
# Ok::<(), std::io::Error>(())
```

### 只有在限制内到达 EOF 才复制

处理不可信 stream 时，使用 `copy_to_end_limited`。如果超过允许大小后仍有数据，它会失败：

```rust
use std::io::Cursor;

use qubit_io::Streams;

let mut input = Cursor::new(b"small".to_vec());
let mut output = Vec::new();

let copied = Streams::copy_to_end_limited(&mut input, &mut output, 16)?;

assert_eq!(5, copied);
assert_eq!(b"small", output.as_slice());
# Ok::<(), std::io::Error>(())
```

### 比较 Stream

`content_eq` 用于相等性判断，`compare_content` 用于字典序比较。两者都是增量读取，不需要把
完整 stream 先加载进内存。

```rust
use std::io::Cursor;

use qubit_io::Streams;

let mut left = Cursor::new(b"abc".to_vec());
let mut right = Cursor::new(b"abc".to_vec());

assert!(Streams::content_eq(&mut left, &mut right)?);
# Ok::<(), std::io::Error>(())
```

## Read Extension Method

`ReadExt` 包含精确读取、有界读取和复制相关 helper。

### 精确读取或 EOF

`read_exact_or_eof` 会尽量填满目标 buffer；如果先遇到 EOF，则返回实际读取的字节数，而不是把
提前 EOF 直接变成错误。

```rust
use std::io::Cursor;

use qubit_io::ReadExt;

let mut input = Cursor::new(b"abc".to_vec());
let mut buffer = [0_u8; 8];

let count = input.read_exact_or_eof(&mut buffer)?;

assert_eq!(3, count);
assert_eq!(b"abc", &buffer[..count]);
# Ok::<(), std::io::Error>(())
```

### 有界读取到内存

当输入大小不完全可信时，使用 `read_to_end_limited` 和 `read_to_string_limited`：

```rust
use std::io::Cursor;

use qubit_io::ReadExt;

let mut input = Cursor::new(b"hello".to_vec());
let bytes = input.read_to_end_limited(16)?;

assert_eq!(b"hello", bytes.as_slice());
# Ok::<(), std::io::Error>(())
```

`_into` 变体会追加到调用方提供的 buffer，并在方法承诺 rollback 的错误路径上回滚追加内容。

### 方法式复制

`copy_to`、`copy_to_at_most` 和 `copy_to_end_limited` 把 `Streams` 命名空间中的复制能力
作为 reader 方法提供。

```rust
use std::io::Cursor;

use qubit_io::ReadExt;

let mut input = Cursor::new(b"abcdef".to_vec());
let mut output = Vec::new();

let copied = input.copy_to_at_most(&mut output, 4)?;

assert_eq!(4, copied);
assert_eq!(b"abcd", output.as_slice());
# Ok::<(), std::io::Error>(())
```

## BufRead Extension Method

`BufReadExt` 增加有界分隔符操作。适合处理 line-based 或 delimiter-based 协议，避免单行或
单字段无界增长。

```rust
use std::io::Cursor;

use qubit_io::BufReadExt;

let mut input = Cursor::new(b"name=value\nrest".to_vec());
let line = input.read_line_limited(32)?;

assert_eq!("name=value\n", line);
# Ok::<(), std::io::Error>(())
```

如果只想跳过一个有界字段，并且不想为这个字段分配 buffer，可以使用
`discard_until_limited`。

## Seek Extension Method

`SeekExt::stream_size` 获取 stream 长度，并恢复原始位置。

```rust
use std::io::{Cursor, Seek, SeekFrom};

use qubit_io::SeekExt;

let mut cursor = Cursor::new(b"abcdef".to_vec());
cursor.seek(SeekFrom::Start(2))?;

let size = cursor.stream_size()?;

assert_eq!(6, size);
assert_eq!(2, cursor.stream_position()?);
# Ok::<(), std::io::Error>(())
```

## Read + Seek Extension Method

`ReadSeekExt` 提供不消费当前位置的读取，以及读取绝对 offset 后恢复原位置的能力。

```rust
use std::io::{Cursor, Seek};

use qubit_io::ReadSeekExt;

let mut cursor = Cursor::new(b"abcdef".to_vec());
let mut buffer = [0_u8; 3];

let count = cursor.peek_exact_or_eof(&mut buffer)?;

assert_eq!(3, count);
assert_eq!(b"abc", &buffer);
assert_eq!(0, cursor.stream_position()?);
# Ok::<(), std::io::Error>(())
```

当需要检查固定 offset，但不能改变调用方可见的 cursor 位置时，使用
`read_exact_or_eof_at`。

## Write + Seek Extension Method

`WriteSeekExt::write_all_at_preserving_position` 在绝对 offset 写入，并恢复原位置。

```rust
use std::io::{Cursor, Seek, SeekFrom};

use qubit_io::WriteSeekExt;

let mut cursor = Cursor::new(vec![0; 8]);
cursor.seek(SeekFrom::Start(7))?;

cursor.write_all_at_preserving_position(2, b"rs")?;

assert_eq!(7, cursor.stream_position()?);
assert_eq!(&[0, 0, b'r', b's', 0, 0, 0, 0], cursor.get_ref().as_slice());
# Ok::<(), std::io::Error>(())
```

## Binary Scalar 编解码

`BinaryReadExt` 和 `BinaryWriteExt` 使用显式字节序读写 fixed-width 数字标量。

```rust
use std::io::Cursor;

use qubit_io::{BinaryReadExt, BinaryWriteExt};

let mut buffer = Vec::new();
buffer.write_u32_be(0x0102_0304)?;
buffer.write_i16_le(-2)?;

let mut input = Cursor::new(buffer);
assert_eq!(0x0102_0304, input.read_u32_be()?);
assert_eq!(-2, input.read_i16_le()?);
# Ok::<(), std::io::Error>(())
```

当字节序由格式元数据决定，而不是由代码结构固定时，使用运行时 `ByteOrder` API。

## LEB128 与 ZigZag 编码

`Leb128ReadExt` 和 `Leb128WriteExt` 支持通过 128 位值读写 unsigned / signed LEB128
整数。严格读取变体会拒绝 non-canonical 编码。

`ZigZagReadExt` 和 `ZigZagWriteExt` 把有符号值编码成 unsigned LEB128 payload。
当小的负数也需要保持紧凑时，使用 ZigZag。

```rust
use std::io::Cursor;

use qubit_io::{Leb128ReadExt, Leb128WriteExt, ZigZagReadExt, ZigZagWriteExt};

let mut buffer = Vec::new();
buffer.write_uleb128_u64(300)?;
buffer.write_zig_zag_i64(-42)?;

let mut input = Cursor::new(buffer);
assert_eq!(300, input.read_uleb128_u64()?);
assert_eq!(-42, input.read_zig_zag_i64()?);
# Ok::<(), std::io::Error>(())
```

## Length-Prefixed UTF-8 字符串

`StringReadExt` 和 `StringWriteExt` 使用 ULEB128、`u16` 或 `u32` 字节长度前缀读写 UTF-8
字符串。读取时由调用方提供大小上限。

```rust
use std::io::Cursor;

use qubit_io::{StringReadExt, StringWriteExt};

let mut buffer = Vec::new();
buffer.write_utf8_string_uleb128("hello")?;

let mut input = Cursor::new(buffer);
let value = input.read_utf8_string_uleb128(32)?;

assert_eq!("hello", value);
# Ok::<(), std::io::Error>(())
```

## Wrapper 类型

### CountingReader 和 CountingWriter

当 metrics 或校验逻辑需要知道成功读写了多少字节时，使用 counting wrapper。

```rust
use std::io::{Cursor, Read};

use qubit_io::CountingReader;

let inner = Cursor::new(b"abc".to_vec());
let mut reader = CountingReader::new(inner);
let mut output = Vec::new();

reader.read_to_end(&mut output)?;

assert_eq!(3, reader.count());
# Ok::<(), std::io::Error>(())
```

### LimitReader 和 LimitWriter

当下游 API 在固定字节预算之后应该看到 EOF 或写入失败时，使用 limit wrapper。

```rust
use std::io::Read;

use qubit_io::LimitReader;

let inner = std::io::Cursor::new(b"abcdef".to_vec());
let mut reader = LimitReader::new(inner, 3);
let mut output = Vec::new();

reader.read_to_end(&mut output)?;

assert_eq!(b"abc", output.as_slice());
# Ok::<(), std::io::Error>(())
```

### TeeReader 和 TeeWriter

当你希望在保持正常读写流程的同时，把流量复制到 branch writer 时，使用 tee wrapper。

### ChecksumReader 和 ChecksumWriter

checksum wrapper 会对成功读取或写入的字节更新调用方持有的 checksum 状态。它不绑定具体
checksum 算法。

### PositionGuard

`PositionGuard` 记录当前 stream 位置，并在 drop 时恢复，除非主动 dismiss。它适合函数需要
检查 header、探测格式元数据、读取 magic number，或者运行一个 speculative parser，但不能
改变调用方可见 cursor 位置的场景。

当探测操作应该成为新的可见位置时，调用 `dismiss`。否则让 guard drop 即可恢复原位置。

```rust
use std::io::{Cursor, Read};

use qubit_io::PositionGuard;

fn looks_like_qubit(input: &mut Cursor<Vec<u8>>) -> std::io::Result<bool> {
    let _guard = PositionGuard::new(input)?;
    let mut magic = [0_u8; 4];
    input.read_exact(&mut magic)?;
    Ok(&magic == b"QBIT")
}

let mut input = Cursor::new(b"QBITpayload".to_vec());

assert!(looks_like_qubit(&mut input)?);
assert_eq!(0, input.position());

# Ok::<(), std::io::Error>(())
```

## Codec Reader 和 Writer Object

如果偏好 object-style codec API，而不是 extension trait，可以使用 reader 和 writer
wrapper。这些 wrapper 持有底层 stream，并复用与 extension trait 相同的编码逻辑。当 codec
配置应该跟随 reader 或 writer 对象时，这种风格会更合适。

| Wrapper | 用途 |
| --- | --- |
| `BinaryReader`、`BinaryWriter` | fixed-width 标量 |
| `Leb128Reader`、`Leb128Writer` | LEB128 整数 |
| `ZigZagReader`、`ZigZagWriter` | ZigZag 有符号整数 |

### BinaryReader 和 BinaryWriter

当解析或写入 fixed-width scalar 格式，并希望把字节序操作集中在一个专门对象上时，使用
binary wrapper。

```rust
use std::io::Cursor;

use qubit_io::{BinaryReader, BinaryWriter};

let mut writer = BinaryWriter::new(Vec::new());
writer.write_u16_be(0x0102)?;
writer.write_i32_le(-7)?;
let bytes = writer.into_inner();

let mut reader = BinaryReader::new(Cursor::new(bytes));
assert_eq!(0x0102, reader.read_u16_be()?);
assert_eq!(-7, reader.read_i32_le()?);

# Ok::<(), std::io::Error>(())
```

### Leb128Reader 和 Leb128Writer

当整数希望以紧凑形式存储时，使用 LEB128 wrapper。reader 可以配置为 strict canonical
decoding；当格式边界需要拒绝 non-canonical 编码时，这很有用。

```rust
use std::io::Cursor;

use qubit_io::{Leb128Reader, Leb128Writer};

let mut writer = Leb128Writer::new(Vec::new());
writer.write_u64(300)?;
writer.write_i64(-42)?;
let bytes = writer.into_inner();

let mut reader = Leb128Reader::new(Cursor::new(bytes)).with_strict(true);
assert_eq!(300, reader.read_u64()?);
assert_eq!(-42, reader.read_i64()?);

# Ok::<(), std::io::Error>(())
```

### ZigZagReader 和 ZigZagWriter

当有符号整数通常在零附近，包括负数也应该保持紧凑时，使用 ZigZag wrapper。ZigZag 会把
有符号值映射成 unsigned LEB128 payload，使 `-1`、`0`、`1` 这类值保持短编码。

```rust
use std::io::Cursor;

use qubit_io::{ZigZagReader, ZigZagWriter};

let mut writer = ZigZagWriter::new(Vec::new());
writer.write_i64(-1)?;
writer.write_i64(42)?;
let bytes = writer.into_inner();

let mut reader = ZigZagReader::new(Cursor::new(bytes)).with_strict(true);
assert_eq!(-1, reader.read_i64()?);
assert_eq!(42, reader.read_i64()?);

# Ok::<(), std::io::Error>(())
```

## 错误模型

大多数 API 返回 `std::io::Result`。本 crate 尽量保持标准 I/O 行为，并使用
`std::io::ErrorKind` 表达大小限制、non-canonical 编码等校验失败。涉及位置恢复的方法会在
文档中说明：当原操作和恢复操作都可能失败时，最终返回哪个错误。

## Crate 边界

`qubit-io` 不包含本地文件系统工具。如果需要本地路径工具、临时文件和目录、递归目录操作、
目录清理或 atomic file write，请使用
[qubit-local-fs](https://github.com/qubit-ltd/rs-local-fs)。
