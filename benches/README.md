# stream 基准说明（生产场景）

本文档用于约束 `benches/stream.rs` 的基准口径，避免不同版本间口径漂移导致误判。

- 基准只覆盖二进制整数路径：
  - `prod_binary_pipeline`：固定字段的二进制读写。
  - `prod_varints`：随机类型字段流的无符号 LEB128 编解码。
  - `prod_signed_varints`：随机类型字段流的 ZigZag 编解码。
- 已移除 UTF-8 文本读写基准。
- 输入规模采用大批量重复：
  - 单批记录数：`BINARY_BATCH = 1_048_576`
  - 单批 varint 字段数：`VARINT_COUNT = 262_144`
  - 每次 benchmark iteration 内重复次数：`BINARY_REPEAT = 32`、`VARINT_REPEAT = 64`
  - `BINARY_REPEAT` 与 `VARINT_REPEAT` 低于纯内存场景，因为所有 stream 基准都走真实文件 IO；二进制固定字段单次样本仍会处理约 1.3 GiB 数据。
- 数据分布采用近似正态分布采样（基于固定 seed 的 Box-Muller），以贴近真实业务里“高峰聚集、少量极端值”的场景。
- `prod_varints` 与 `prod_signed_varints` 的字段类型也是随机分布：
  - 无符号 LEB128 在 `u8/u16/u32/u64/usize/u128` 中随机选择字段类型，并按类型生成随机值。
  - ZigZag 在 `i8/i16/i32/i64/isize/i128` 中随机选择字段类型，并按类型生成随机值。
  - 读 benchmark 使用同一字段 schema 流 dispatch 对应读取方法，避免连续读取单一类型造成过度理想化。
- `prod_binary_pipeline` 使用真实文件系统文件作为输入输出：
  - 写入基准使用 `File::create` + `BufWriter<File>`，每轮覆盖写同一个临时文件。
  - 读取基准先由 writer 生成临时源文件，再使用 `File::open` + `BufReader<File>` 读取。
  - `std_native_*` 使用标准库原生 fixed-width 路径：`write_all()` + `to_le_bytes()`，以及 `read_exact()` + `from_le_bytes()`。该组只适用于固定宽度二进制字段。
  - 基准不调用 `sync_all()`，因此结果仍可能受 OS page cache 影响；目标是排除 `Cursor<&[u8]>` / `Cursor<Vec<u8>>` 的内存特化，而不是测物理磁盘落盘延迟。
  - 临时文件目录创建在系统临时目录下，benchmark 结束时尽力清理。
- `prod_varints` 与 `prod_signed_varints` 同样使用真实文件系统文件：
  - `ext_*` 使用 `File::create/open` + `BufWriter<File>` / `BufReader<File>`，再调用对应 extension trait。
  - `std_manual_*` 使用 `BufWriter<File>` / `BufReader<File>`，并在 benchmark 内部手写安全的 LEB128 / ZigZag 编解码，不调用本 crate 的 codec，也不使用 unchecked slice 优化。
  - `wrapper_*` 使用 `BinaryReader/Writer`、`Leb128Reader/Writer`、`ZigZagReader/Writer` 包装 `BufReader<File>` / `BufWriter<File>`。
  - `buffered_*` 使用 `BufferedBinaryReader/Writer`、`BufferedLeb128Reader/Writer`、`BufferedZigZagReader/Writer` 直接包装 `File`，由 wrapper 内部维护 8 KiB 缓冲区。
- 每组基准设置 `warm_up_time = 2s`、`measurement_time = 8s`、`sample_size = 12`。
- 每个大组必须在独立的 `cargo bench` 进程中执行，避免前一个大组的长时间
  文件 IO、分支预测和 CPU 调度状态污染后续大组结果。`benches/stream.rs`
  会读取 `QUBIT_IO_STREAM_BENCH_GROUP`，每次只注册并执行一个大组。
  直接运行 `cargo bench --bench stream` 不再作为完整套件入口。

## 基线约定

当前基线口径是同一次 benchmark run 内的 `Read` / `Write` extension trait 实现：

- `ext_*`：使用 `BinaryReadExt` / `BinaryWriteExt`、`Leb128ReadExt` / `Leb128WriteExt`、`ZigZagReadExt` / `ZigZagWriteExt`。
- `std_native_*`：仅在 `prod_binary_pipeline` 中出现，使用标准库 `BufReader<File>` / `BufWriter<File>` 加 `read_exact()` / `write_all()` 和基础类型字节序转换。
- `std_manual_*`：仅在 `prod_varints` 与 `prod_signed_varints` 中出现，使用标准库 `BufReader<File>` / `BufWriter<File>` 加手写安全 LEB128 / ZigZag 协议实现。
- `wrapper_*`：使用 `BinaryReader` / `BinaryWriter`、`Leb128Reader` / `Leb128Writer`、`ZigZagReader` / `ZigZagWriter`。
- `buffered_*`：使用自带大缓冲区并直接在缓冲区上调用 codec unsafe 方法的 buffered reader/writer。

结果解读时应比较同一 group 下相同方向的 `ext_*` 与 `wrapper_*`，例如：

- `prod_varints/ext_leb128_read_mixed_batch`
- `prod_varints/std_manual_leb128_read_mixed_batch`
- `prod_varints/buffered_leb128_read_mixed_batch`

不再把“上一次提交版本”作为主要性能基线。提交间 baseline 只适合判断同一实现随代码演进是否漂移，不适合评估 wrapper 相对 extension trait 的收益。

示例流程（在本仓库根目录执行）：

```bash
benches/run_stream_bench_groups.sh
```

如只运行单个大组：

```bash
QUBIT_IO_STREAM_BENCH_GROUP=prod_varints cargo bench --bench stream
```
