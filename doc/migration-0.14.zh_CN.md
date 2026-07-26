# 从 Qubit IO 0.13 迁移到 0.14

0.14 将面向标准库同步流的 wrapper 替换为面向 Qubit IO 泛型 item-stream trait
的 wrapper。这是一次破坏性变更。

## 类型映射

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

异步 wrapper 的名称不变。

## Trait 变化

改名后的同步 wrapper 实现 `Input` 或 `Output`，不再实现
`std::io::Read` 或 `std::io::Write`。标准字节流仍可直接作为内部流使用：
所有 `Read` 都自动实现 `Input<Item = u8>`，所有 `Write` 都自动实现
`Output<Item = u8>`。

请通过 `Input::read`、`Input::read_fully`、`Output::write` 和
`Output::write_fully` 调用 Qubit 操作。如果标准库 trait 带来了同名方法，
请使用完全限定语法：

```rust
use std::io::Cursor;

use qubit_io::{
    CountingInput,
    Input,
};

let mut input = CountingInput::new(Cursor::new(b"abc".to_vec()));
let mut bytes = [0_u8; 3];
Input::read_exactly(&mut input, &mut bytes)?;
assert_eq!(3, input.bytes_read());
# Ok::<(), std::io::Error>(())
```

原有的 `Seek` 转发改为 `Seekable`。位置单位由内部流的 `Seekable::Unit`
决定；标准 `Seek` 类型自动使用 `u8`。原有的 `BufRead` 转发以及通过
`consume` 计数或限量的行为已移除。

## Item 与 accessor 行为

`CountingInput`、`CountingOutput`、`LimitInput`、`LimitOutput`、
`TeeInput` 和 `TeeOutput` 现在可以处理任意匹配的 item 类型。计数值在
`u64::MAX` 饱和。`ChecksumInput` 和 `ChecksumOutput` 仍只处理 `u8`。

可变 accessor 会绕过 wrapper 的状态维护。通过 `inner_mut()` 直接读取时，
wrapper 不会计数、限量、hash 或镜像；直接操作 branch 也可能让 tee 两侧
产生差异。checksum 和 tee wrapper 通过 `into_parts()` 返回两个所有权组件；
counting 和 limit wrapper 继续使用 `into_inner()`。

## Tee 失败语义

Tee 操作按顺序执行，并且不具备事务性：

- `TeeInput` 和 `SyncSeekTeeInput` 先读取 source，再写 branch。branch
  失败时，source 已前进，调用方的 destination 也已被修改。
- `TeeOutput` 先写 primary，再写 branch。branch 失败时，primary 已前进。
- Tee 的 flush 和同步 seek 先操作 primary/source；后续 branch 失败不会回滚
  第一个操作。

需要原子复制的调用方应在更高层提供事务或恢复机制。
