# Buffered Varint Fast Path and Std Native Benchmark - 2026-05-24

This file records the `benches/stream.rs` result after adding the buffered
varint read fast path refinements and the fixed-width `std_native_*` benchmark
cases.

## Environment

- Command: `benches/run_stream_bench_groups.sh`
- Date: 2026-05-24
- OS: Darwin arm64
- Cargo: `cargo 1.94.1 (29ea6fb6a 2026-03-24)`
- Rustc: `rustc 1.94.1 (e408947bf 2026-03-25)`
- Source: `target/criterion/*/new/estimates.json`

## Benchmark Shape

- `prod_binary_pipeline`: fixed record with mixed fixed-width fields.
- `prod_varints`: random unsigned LEB128 field stream. Field type is randomly
  selected from `u8/u16/u32/u64/usize/u128`; field value is randomly generated
  for the selected type.
- `prod_signed_varints`: random ZigZag field stream. Field type is randomly
  selected from `i8/i16/i32/i64/isize/i128`; field value is randomly generated
  for the selected type.
- All stream benchmarks use real filesystem files.
- `ext_*`: `BufReader<File>` / `BufWriter<File>` plus extension traits.
- `std_native_*`: standard `BufReader<File>` / `BufWriter<File>` plus
  `read_exact()` / `write_all()` and primitive byte-order conversions. This
  group exists only for fixed-width binary fields because the standard library
  has no native LEB128 or ZigZag codec.
- `wrapper_*`: existing wrapper reader/writer plus `BufReader<File>` /
  `BufWriter<File>`.
- `buffered_*`: buffered reader/writer directly wrapping `File`.

## Mean Time

Times are Criterion mean estimates in milliseconds. Lower is faster.

| Group | Case | Mean ms | 95% CI ms |
|---|---|---:|---:|
| `prod_binary_pipeline` | `ext_write_record_batch` | 485.05 | 481.62 - 489.21 |
| `prod_binary_pipeline` | `std_native_write_record_batch` | 484.12 | 481.50 - 487.82 |
| `prod_binary_pipeline` | `wrapper_write_record_batch` | 493.50 | 481.84 - 506.75 |
| `prod_binary_pipeline` | `buffered_write_record_batch` | 452.22 | 449.28 - 456.26 |
| `prod_binary_pipeline` | `ext_read_record_batch` | 250.09 | 244.77 - 255.27 |
| `prod_binary_pipeline` | `std_native_read_record_batch` | 275.62 | 274.89 - 276.36 |
| `prod_binary_pipeline` | `wrapper_read_record_batch` | 281.61 | 277.92 - 285.96 |
| `prod_binary_pipeline` | `buffered_read_record_batch` | 204.38 | 203.26 - 205.69 |
| `prod_varints` | `ext_leb128_write_mixed_batch` | 197.31 | 196.02 - 198.80 |
| `prod_varints` | `wrapper_leb128_write_mixed_batch` | 197.35 | 196.78 - 197.86 |
| `prod_varints` | `buffered_leb128_write_mixed_batch` | 149.52 | 147.91 - 151.34 |
| `prod_varints` | `ext_leb128_read_mixed_batch` | 158.01 | 156.98 - 159.28 |
| `prod_varints` | `wrapper_leb128_read_mixed_batch` | 156.51 | 155.86 - 157.21 |
| `prod_varints` | `buffered_leb128_read_mixed_batch` | 153.50 | 152.37 - 154.93 |
| `prod_signed_varints` | `ext_zigzag_write_mixed_batch` | 205.26 | 203.36 - 207.63 |
| `prod_signed_varints` | `wrapper_zigzag_write_mixed_batch` | 192.94 | 192.35 - 193.57 |
| `prod_signed_varints` | `buffered_zigzag_write_mixed_batch` | 156.59 | 155.22 - 158.59 |
| `prod_signed_varints` | `ext_zigzag_read_mixed_batch` | 158.26 | 157.85 - 158.83 |
| `prod_signed_varints` | `wrapper_zigzag_read_mixed_batch` | 161.77 | 160.22 - 163.56 |
| `prod_signed_varints` | `buffered_zigzag_read_mixed_batch` | 158.03 | 157.40 - 158.70 |

## Speed Compared With Current Baselines

Speed change is computed from mean time as `baseline_mean / candidate_mean - 1`.
Positive values mean the candidate is faster than the baseline.

| Scenario | `std_native vs ext` | `wrapper vs ext` | `buffered vs ext` | `buffered vs std_native` | `buffered vs wrapper` |
|---|---:|---:|---:|---:|---:|
| Binary write | +0.19% | -1.71% | +7.26% | +7.06% | +9.13% |
| Binary read | -9.26% | -11.19% | +22.36% | +34.86% | +37.79% |
| LEB128 mixed write | N/A | -0.02% | +31.96% | N/A | +31.99% |
| LEB128 mixed read | N/A | +0.96% | +2.93% | N/A | +1.96% |
| ZigZag mixed write | N/A | +6.39% | +31.08% | N/A | +23.21% |
| ZigZag mixed read | N/A | -2.17% | +0.14% | N/A | +2.37% |

## Current Interpretation

- Buffered writes are the clearest win in this run. Fixed-width binary,
  LEB128, and ZigZag writes are all faster than the extension-trait and wrapper
  baselines.
- Buffered fixed-width binary read is also clearly faster than the three
  `BufReader<File>` baselines.
- Buffered LEB128 read is modestly faster than both extension-trait and wrapper
  reads.
- Buffered ZigZag read is effectively tied with extension-trait read, but still
  faster than wrapper read in this run.
- The fixed-width `std_native_*` baseline is not a performance ceiling here.
  It is close to extension-trait write, but slower than extension-trait read.
