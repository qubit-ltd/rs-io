# Qubit IO

面向 Rust 的小型 I/O trait 工具库。

本 crate 提供常用 `std::io` trait 组合的 object-safe trait：

- `ReadSeek`
- `ReadWrite`
- `WriteSeek`
- `ReadWriteSeek`

当 API 需要使用 `&mut dyn ReadSeek` 这类 trait object，而不是
`R: Read + Seek` 这类泛型约束时，可以使用这些组合 trait。
