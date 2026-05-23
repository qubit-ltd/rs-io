# stream 基准说明（生产场景）

本文档用于约束 `benches/stream.rs` 的基准口径，避免不同版本间口径漂移导致误判。

- 基准只覆盖二进制整数路径：
  - `prod_binary_pipeline`：固定字段的二进制读写。
  - `prod_varints`：`u64` LEB128 编解码。
  - `prod_signed_varints`：`i64` ZigZag 编解码。
- 已移除 UTF-8 文本读写基准。
- 输入规模采用大批量重复：
  - 单批记录数：`BINARY_BATCH = 1_048_576`
  - 单批 int 数量：`VARINT_COUNT = 1_048_576`
  - 每次 benchmark iteration 内重复次数：`BINARY_REPEAT = 32`、`VARINT_REPEAT = 512`
  - `BINARY_REPEAT` 低于 varint 路径，因为 `prod_binary_pipeline` 走真实文件 IO；单次样本仍会处理约 1.3 GiB 二进制数据。
- 数据分布采用近似正态分布采样（基于固定 seed 的 Box-Muller），以贴近真实业务里“高峰聚集、少量极端值”的场景。
- `prod_binary_pipeline` 使用真实文件系统文件作为输入输出：
  - 写入基准使用 `File::create` + `BufWriter<File>`，每轮覆盖写同一个临时文件。
  - 读取基准先由 writer 生成临时源文件，再使用 `File::open` + `BufReader<File>` 读取。
  - 基准不调用 `sync_all()`，因此结果仍可能受 OS page cache 影响；目标是排除 `Cursor<&[u8]>` / `Cursor<Vec<u8>>` 的内存特化，而不是测物理磁盘落盘延迟。
  - 临时文件目录创建在系统临时目录下，benchmark 结束时尽力清理。
- `prod_varints` 与 `prod_signed_varints` 仍使用内存缓冲，用于聚焦 varint 编解码热路径。
- 每组基准设置 `warm_up_time = 2s`、`measurement_time = 8s`、`sample_size = 12`。

## 基线约定

当前基线口径是同一次 benchmark run 内的 `Read` / `Write` extension trait 实现：

- `ext_*`：使用 `BinaryReadExt` / `BinaryWriteExt`、`Leb128ReadExt` / `Leb128WriteExt`、`ZigZagReadExt` / `ZigZagWriteExt`。
- `wrapper_*`：使用 `BinaryReader` / `BinaryWriter`、`Leb128Reader` / `Leb128Writer`、`ZigZagReader` / `ZigZagWriter`。

结果解读时应比较同一 group 下相同方向的 `ext_*` 与 `wrapper_*`，例如：

- `prod_varints/ext_leb128_read_u64_batch`
- `prod_varints/wrapper_leb128_read_u64_batch`

不再把“上一次提交版本”作为主要性能基线。提交间 baseline 只适合判断同一实现随代码演进是否漂移，不适合评估 wrapper 相对 extension trait 的收益。

示例流程（在本仓库根目录执行）：

```bash
cargo bench --bench stream
```
