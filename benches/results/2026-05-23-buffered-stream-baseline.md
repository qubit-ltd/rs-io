# Buffered Stream Benchmark Baseline - 2026-05-23

This file records the current `benches/stream.rs` result before further
buffered reader/writer changes.

## Environment

- Command: `cargo bench --bench stream`
- Date: 2026-05-23
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
| `prod_binary_pipeline` | `ext_write_record_batch` | 477.21 | 474.50 - 480.17 |
| `prod_binary_pipeline` | `wrapper_write_record_batch` | 494.29 | 492.37 - 496.24 |
| `prod_binary_pipeline` | `buffered_write_record_batch` | 744.47 | 741.05 - 748.74 |
| `prod_binary_pipeline` | `ext_read_record_batch` | 242.71 | 241.48 - 244.20 |
| `prod_binary_pipeline` | `wrapper_read_record_batch` | 277.29 | 273.50 - 281.69 |
| `prod_binary_pipeline` | `buffered_read_record_batch` | 566.07 | 565.48 - 566.72 |
| `prod_varints` | `ext_leb128_write_mixed_batch` | 199.08 | 197.27 - 201.65 |
| `prod_varints` | `wrapper_leb128_write_mixed_batch` | 198.11 | 195.94 - 200.53 |
| `prod_varints` | `buffered_leb128_write_mixed_batch` | 153.37 | 150.76 - 156.30 |
| `prod_varints` | `ext_leb128_read_mixed_batch` | 150.73 | 150.21 - 151.31 |
| `prod_varints` | `wrapper_leb128_read_mixed_batch` | 148.42 | 147.45 - 149.54 |
| `prod_varints` | `buffered_leb128_read_mixed_batch` | 160.30 | 159.36 - 161.47 |
| `prod_signed_varints` | `ext_zigzag_write_mixed_batch` | 205.77 | 203.04 - 209.06 |
| `prod_signed_varints` | `wrapper_zigzag_write_mixed_batch` | 193.01 | 192.49 - 193.66 |
| `prod_signed_varints` | `buffered_zigzag_write_mixed_batch` | 155.04 | 153.19 - 157.27 |
| `prod_signed_varints` | `ext_zigzag_read_mixed_batch` | 151.75 | 151.31 - 152.27 |
| `prod_signed_varints` | `wrapper_zigzag_read_mixed_batch` | 150.94 | 150.51 - 151.38 |
| `prod_signed_varints` | `buffered_zigzag_read_mixed_batch` | 164.28 | 163.67 - 164.98 |

## Time Ratios

Ratios are mean time ratios. Lower than `1.000x` means the numerator is faster.

| Scenario | `wrapper / ext` | `buffered / ext` | `buffered / wrapper` |
|---|---:|---:|---:|
| Binary write | 1.036x | 1.560x | 1.506x |
| Binary read | 1.142x | 2.332x | 2.041x |
| LEB128 mixed write | 0.995x | 0.770x | 0.774x |
| LEB128 mixed read | 0.985x | 1.063x | 1.080x |
| ZigZag mixed write | 0.938x | 0.753x | 0.803x |
| ZigZag mixed read | 0.995x | 1.083x | 1.088x |

## Current Interpretation

- Buffered fixed-width binary reader/writer is slower than the `BufReader` /
  `BufWriter` extension-trait baseline.
- Buffered LEB128 and ZigZag writers are faster in random mixed-type field
  streams.
- Buffered LEB128 and ZigZag readers are slower in random mixed-type field
  streams.
