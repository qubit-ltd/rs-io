# Buffered Input Refill Benchmark - 2026-05-24

This file records the `benches/stream.rs` result after changing
`BufferedInput` to follow the standard `BufReader` refill pattern more closely:
append into tail space first, backshift only when required, and bypass the
internal buffer for large raw reads.

## Environment

- Command: `cargo bench --bench stream`
- Date: 2026-05-24
- OS: Darwin arm64
- Cargo: `cargo 1.94.1 (29ea6fb6a 2026-03-24)`
- Rustc: `rustc 1.94.1 (e408947bf 2026-03-25)`
- Source: `target/criterion/*/new/estimates.json`

## Benchmark Shape

- `prod_binary_pipeline`: fixed record with mixed fixed-width fields.
- `prod_varints`: random unsigned LEB128 field stream.
  Field type is randomly selected from `u8/u16/u32/u64/usize/u128`; field value
  is randomly generated for the selected type.
- `prod_signed_varints`: random ZigZag field stream.
  Field type is randomly selected from `i8/i16/i32/i64/isize/i128`; field value
  is randomly generated for the selected type.
- All stream benchmarks use real filesystem files.
- `ext_*`: `BufReader<File>` / `BufWriter<File>` plus extension traits.
- `wrapper_*`: existing wrapper reader/writer plus `BufReader<File>` /
  `BufWriter<File>`.
- `buffered_*`: buffered reader/writer directly wrapping `File`.

## Mean Time

Times are Criterion mean estimates in milliseconds. Lower is faster.

| Group | Case | Mean ms | 95% CI ms |
|---|---|---:|---:|
| `prod_binary_pipeline` | `ext_write_record_batch` | 479.77 | 475.27 - 485.51 |
| `prod_binary_pipeline` | `wrapper_write_record_batch` | 481.37 | 479.18 - 483.94 |
| `prod_binary_pipeline` | `buffered_write_record_batch` | 739.40 | 734.51 - 745.04 |
| `prod_binary_pipeline` | `ext_read_record_batch` | 251.50 | 248.15 - 256.03 |
| `prod_binary_pipeline` | `wrapper_read_record_batch` | 282.67 | 278.38 - 287.43 |
| `prod_binary_pipeline` | `buffered_read_record_batch` | 182.56 | 182.03 - 183.16 |
| `prod_varints` | `ext_leb128_write_mixed_batch` | 199.24 | 197.53 - 201.60 |
| `prod_varints` | `wrapper_leb128_write_mixed_batch` | 197.62 | 196.93 - 198.37 |
| `prod_varints` | `buffered_leb128_write_mixed_batch` | 151.38 | 149.98 - 153.13 |
| `prod_varints` | `ext_leb128_read_mixed_batch` | 150.91 | 150.33 - 151.50 |
| `prod_varints` | `wrapper_leb128_read_mixed_batch` | 149.95 | 149.50 - 150.37 |
| `prod_varints` | `buffered_leb128_read_mixed_batch` | 157.16 | 156.83 - 157.50 |
| `prod_signed_varints` | `ext_zigzag_write_mixed_batch` | 203.18 | 201.69 - 204.91 |
| `prod_signed_varints` | `wrapper_zigzag_write_mixed_batch` | 194.75 | 193.57 - 196.43 |
| `prod_signed_varints` | `buffered_zigzag_write_mixed_batch` | 153.08 | 152.39 - 153.77 |
| `prod_signed_varints` | `ext_zigzag_read_mixed_batch` | 151.55 | 151.07 - 152.06 |
| `prod_signed_varints` | `wrapper_zigzag_read_mixed_batch` | 152.00 | 151.31 - 152.82 |
| `prod_signed_varints` | `buffered_zigzag_read_mixed_batch` | 162.34 | 161.56 - 163.39 |

## Speed Compared With Current Baselines

Speed change is computed from mean time as `baseline_mean / candidate_mean - 1`.
Positive values mean the candidate is faster than the baseline.

| Scenario | `wrapper vs ext` | `buffered vs ext` | `buffered vs wrapper` |
|---|---:|---:|---:|
| Binary write | -0.33% | -35.11% | -34.89% |
| Binary read | -11.03% | +37.76% | +54.84% |
| LEB128 mixed write | +0.82% | +31.62% | +30.55% |
| LEB128 mixed read | +0.64% | -3.98% | -4.59% |
| ZigZag mixed write | +4.33% | +32.73% | +27.22% |
| ZigZag mixed read | -0.29% | -6.64% | -6.37% |

## Buffered Speed Compared With Previous Baseline

Previous baseline: `benches/results/2026-05-23-buffered-stream-baseline.md`.
Speed change is computed as `old_buffered_mean / new_buffered_mean - 1`.
Positive values mean the new buffered implementation is faster.

| Scenario | Old buffered ms | New buffered ms | Speed change |
|---|---:|---:|---:|
| Binary write | 744.47 | 739.40 | +0.69% |
| Binary read | 566.07 | 182.56 | +210.07% |
| LEB128 mixed write | 153.37 | 151.38 | +1.32% |
| LEB128 mixed read | 160.30 | 157.16 | +2.00% |
| ZigZag mixed write | 155.04 | 153.08 | +1.28% |
| ZigZag mixed read | 164.28 | 162.34 | +1.20% |

## Current Interpretation

- The `BufferedInput` refill change primarily improves fixed-width binary
  reads. Buffered binary read is now faster than both the extension-trait and
  wrapper baselines in this benchmark.
- Buffered LEB128 and ZigZag writers remain faster than the corresponding
  extension-trait and wrapper baselines.
- Buffered LEB128 and ZigZag readers improved slightly compared with the
  previous buffered baseline, but remain slower than the current ext/wrapper
  read baselines.
