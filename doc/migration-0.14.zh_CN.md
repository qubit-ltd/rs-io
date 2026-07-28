# 从 Qubit IO 0.13 迁移到 0.14

Qubit IO 0.14 将同步 wrapper 统一为围绕通用 item stream trait 的实现，并把标准库专属
API 移入明确的命名空间。使用旧 wrapper 名称或 crate 根标准 I/O trait 的代码需要进行
破坏性迁移。

## 发布说明

- 同步 reader/writer wrapper 更名为 input/output wrapper，并支持通用 Qubit item
  stream。
- 标准库组合 trait 与扩展 trait 现在分别从 `qubit_io::std_io` 和
  `qubit_io::std_io::ext` 导入。
- `Seek` 转发改由 `Seekable` 表达；旧的 `BufRead` 转发以及基于 `consume` 的计数和
  限制行为已移除。
- 异步 wrapper 名称以及可选的 Tokio、`futures-io` adapter 保持兼容。

## 重命名同步 wrapper

| 0.13 类型 | 0.14 类型 |
| --- | --- |
| `CountingReader` | `CountingInput` |
| `CountingWriter` | `CountingOutput` |
| `LimitReader` | `LimitInput` |
| `LimitWriter` | `LimitOutput` |
| `ChecksumReader` | `ChecksumInput` |
| `ChecksumWriter` | `ChecksumOutput` |
| `TeeReader` | `TeeInput` |
| `TeeWriter` | `TeeOutput` |
| `SyncSeekTeeReader` | `SyncSeekTeeInput` |

更名后的 wrapper 实现的是 `Input` 或 `Output`，不再实现 `std::io::Read` 或
`std::io::Write`。标准字节流仍可直接使用，因为每个 `Read` 都实现
`Input<Item = u8>`，每个 `Write` 都实现 `Output<Item = u8>`。

```rust
use std::io::Cursor;
use qubit_io::{CountingInput, Input};

let mut input = CountingInput::new(Cursor::new(b"abc".to_vec()));
let mut bytes = [0_u8; 3];
Input::read_exactly(&mut input, &mut bytes)?;
assert_eq!(3, input.bytes_read());
# Ok::<(), std::io::Error>(())
```

请使用 `Input` 与 `Output` 的操作（`read`、`read_fully`、`read_exactly`、`write`
和 `write_fully`），不要再依赖旧 wrapper 的标准库 trait 实现。完全限定语法可避免
同名方法的歧义。

## 更新标准 I/O 导入

crate 根保留 `Input`、`Output`、`Seekable` 等运行时中立 API。标准库专属 API 应从下列
命名空间导入：

| API 类型 | 0.14 导入路径 |
| --- | --- |
| 组合 trait | `qubit_io::std_io::{BufReadSeek, ReadSeek, ReadWrite, ReadWriteSeek, WriteSeek}` |
| 扩展 trait | `qubit_io::std_io::ext::{BufReadExt, ReadExt, ReadSeekExt, SeekExt, WriteExt, WriteSeekExt}` |

例如，把 `use qubit_io::{ReadSeek, ReadSeekExt};` 改为：

```rust
use qubit_io::std_io::ReadSeek;
use qubit_io::std_io::ext::ReadSeekExt;
```

## item、seek 与 tee 行为

`CountingInput`、`CountingOutput`、`LimitInput`、`LimitOutput`、`TeeInput` 与
`TeeOutput` 可包装字节以外的匹配 item 类型。计数值会饱和到 `u64::MAX`；checksum
wrapper 仍仅支持字节。

`Seek` 转发已由 `Seekable` 取代。位置使用被包装流的 `Seekable::Unit`；标准库的
`Seek` 值使用 `u8`。

tee 操作有顺序且不是事务性的：`TeeInput` 和 `SyncSeekTeeInput` 先推进 source，后写
branch；`TeeOutput` 先写 primary，后写 branch。后续操作失败不会回滚前一操作。需要
原子复制时，应在更高层补充事务或恢复机制。

wrapper 的可变 accessor 会绕过记账：通过 `inner_mut()` 的读操作不会被计数、限制、
hash 或镜像；直接操作 branch 可能使 tee 分叉。checksum 与 tee wrapper 通过
`into_parts()` 返回两个组件，counting 与 limit wrapper 保留 `into_inner()`。
