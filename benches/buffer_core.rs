// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Microbenchmarks for buffered-copy primitives.
//!
//! Run with `cargo bench --bench buffer_core`. The benchmark separates loops
//! whose bounds are visible to the optimizer from loops whose indices are only
//! known at runtime, so unchecked copies can be compared with safe slices under
//! the same valid-range invariants.

use std::hint::black_box;

use criterion::{
    BatchSize,
    BenchmarkId,
    Criterion,
    Throughput,
    criterion_group,
    criterion_main,
};
use qubit_io::{
    Buffer,
    UncheckedSlice,
};

const BUFFER_CAPACITY: usize = 8 * 1024;
const DATA_LEN: usize = 256 * 1024;
const COPY_WIDTHS: [usize; 4] = [1, 8, 64, 1024];

/// Creates deterministic input data for copy benchmarks.
fn benchmark_fixture() -> Vec<u8> {
    (0..DATA_LEN)
        .map(|index| (index as u8).wrapping_mul(31))
        .collect()
}

/// Returns valid copy offsets for the supplied transfer width.
fn copy_offsets(width: usize) -> Vec<usize> {
    (0..DATA_LEN).step_by(width).collect()
}

/// Copies the fixture through `UncheckedSlice` with compiler-visible bounds.
fn copy_unchecked_with_visible_invariants(
    input: &[u8],
    output: &mut [u8],
    width: usize,
) {
    for offset in (0..input.len()).step_by(width) {
        // SAFETY: `COPY_WIDTHS` divides `DATA_LEN`; both ranges start at the
        // same valid offset and contain exactly `width` items.
        unsafe {
            UncheckedSlice::copy_nonoverlapping(
                input, offset, output, offset, width,
            );
        }
    }
}

/// Copies the fixture with safe slices while bounds remain visible to the
/// optimizer.
fn copy_safe_with_visible_invariants(
    input: &[u8],
    output: &mut [u8],
    width: usize,
) {
    for offset in (0..input.len()).step_by(width) {
        output[offset..offset + width]
            .copy_from_slice(&input[offset..offset + width]);
    }
}

/// Copies the fixture through `UncheckedSlice` with runtime-only offsets.
fn copy_unchecked_with_runtime_indexes(
    input: &[u8],
    output: &mut [u8],
    offsets: &[usize],
    width: usize,
) {
    for &offset in offsets {
        let offset = black_box(offset);
        // SAFETY: `offsets` is built from `copy_offsets`, so each source and
        // destination range remains valid even though the optimizer cannot
        // prove that fact after `black_box`.
        unsafe {
            UncheckedSlice::copy_nonoverlapping(
                input, offset, output, offset, width,
            );
        }
    }
}

/// Copies the fixture with safe slices and runtime-only offsets.
fn copy_safe_with_runtime_indexes(
    input: &[u8],
    output: &mut [u8],
    offsets: &[usize],
    width: usize,
) {
    for &offset in offsets {
        let offset = black_box(offset);
        output[offset..offset + width]
            .copy_from_slice(&input[offset..offset + width]);
    }
}

/// Appends the fixture through `Buffer::copy_from` using valid window
/// invariants.
fn append_with_buffer(input: &[u8], buffer: &mut Buffer<u8>, width: usize) {
    for input_index in (0..input.len()).step_by(width) {
        if buffer.spare_capacity() < width {
            buffer.clear();
        }
        // SAFETY: `COPY_WIDTHS` divides both `DATA_LEN` and `BUFFER_CAPACITY`.
        // The branch resets the buffer before an append can exceed spare space.
        unsafe {
            buffer.copy_from(input, input_index, width);
        }
    }
}

/// Appends the fixture with the corresponding safe-slice window implementation.
fn append_with_safe_slices(
    input: &[u8],
    output: &mut [u8],
    width: usize,
) -> usize {
    let mut limit = 0_usize;
    for input_index in (0..input.len()).step_by(width) {
        if output.len() - limit < width {
            limit = 0;
        }
        output[limit..limit + width]
            .copy_from_slice(&input[input_index..input_index + width]);
        limit += width;
    }
    limit
}

/// Copies one readable buffer window through `Buffer::copy_to`.
fn copy_from_buffer(buffer: &mut Buffer<u8>, output: &mut [u8], width: usize) {
    for output_index in (0..output.len()).step_by(width) {
        // SAFETY: `COPY_WIDTHS` divides `BUFFER_CAPACITY`; each iteration
        // copies no more than the remaining readable window and output range.
        unsafe {
            buffer.copy_to(output, output_index, width);
        }
    }
}

/// Copies one readable window through corresponding safe slices.
fn copy_from_safe_slices(input: &[u8], output: &mut [u8], width: usize) {
    for output_index in (0..output.len()).step_by(width) {
        output[output_index..output_index + width]
            .copy_from_slice(&input[output_index..output_index + width]);
    }
}

/// Benchmarks primitive copies with loop invariants visible to the optimizer.
fn benchmark_visible_invariants(criterion: &mut Criterion) {
    let input = benchmark_fixture();
    let mut group = criterion.benchmark_group("buffer_core/visible_invariants");
    group.measurement_time(std::time::Duration::from_secs(2));
    group.sample_size(10);
    group.throughput(Throughput::Bytes(DATA_LEN as u64));

    for width in COPY_WIDTHS {
        group.bench_with_input(
            BenchmarkId::new("unchecked_slice", width),
            &width,
            |bencher, &width| {
                bencher.iter_batched(
                    || vec![0_u8; DATA_LEN],
                    |mut output| {
                        copy_unchecked_with_visible_invariants(
                            &input,
                            &mut output,
                            width,
                        );
                        black_box(output);
                    },
                    BatchSize::LargeInput,
                );
            },
        );
        group.bench_with_input(
            BenchmarkId::new("safe_slices", width),
            &width,
            |bencher, &width| {
                bencher.iter_batched(
                    || vec![0_u8; DATA_LEN],
                    |mut output| {
                        copy_safe_with_visible_invariants(
                            &input,
                            &mut output,
                            width,
                        );
                        black_box(output);
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }
    group.finish();
}

/// Benchmarks primitive copies when valid indexes are opaque at the call site.
fn benchmark_runtime_indexes(criterion: &mut Criterion) {
    let input = benchmark_fixture();
    let mut group = criterion.benchmark_group("buffer_core/runtime_indexes");
    group.measurement_time(std::time::Duration::from_secs(2));
    group.sample_size(10);
    group.throughput(Throughput::Bytes(DATA_LEN as u64));

    for width in COPY_WIDTHS {
        let offsets = copy_offsets(width);
        group.bench_with_input(
            BenchmarkId::new("unchecked_slice", width),
            &width,
            |bencher, &width| {
                bencher.iter_batched(
                    || vec![0_u8; DATA_LEN],
                    |mut output| {
                        copy_unchecked_with_runtime_indexes(
                            &input,
                            &mut output,
                            &offsets,
                            width,
                        );
                        black_box(output);
                    },
                    BatchSize::LargeInput,
                );
            },
        );
        group.bench_with_input(
            BenchmarkId::new("safe_slices", width),
            &width,
            |bencher, &width| {
                bencher.iter_batched(
                    || vec![0_u8; DATA_LEN],
                    |mut output| {
                        copy_safe_with_runtime_indexes(
                            &input,
                            &mut output,
                            &offsets,
                            width,
                        );
                        black_box(output);
                    },
                    BatchSize::LargeInput,
                );
            },
        );
    }
    group.finish();
}

/// Benchmarks the `Buffer::copy_from` append path against safe window copies.
fn benchmark_buffer_append(criterion: &mut Criterion) {
    let input = benchmark_fixture();
    let mut group = criterion.benchmark_group("buffer_core/buffer_append");
    group.measurement_time(std::time::Duration::from_secs(2));
    group.sample_size(10);
    group.throughput(Throughput::Bytes(DATA_LEN as u64));

    for width in COPY_WIDTHS {
        group.bench_with_input(
            BenchmarkId::new("buffer_unchecked", width),
            &width,
            |bencher, &width| {
                bencher.iter_batched(
                    || Buffer::with_capacity(BUFFER_CAPACITY),
                    |mut buffer| {
                        append_with_buffer(&input, &mut buffer, width);
                        black_box(buffer.available());
                    },
                    BatchSize::SmallInput,
                );
            },
        );
        group.bench_with_input(
            BenchmarkId::new("safe_slices", width),
            &width,
            |bencher, &width| {
                bencher.iter_batched(
                    || vec![0_u8; BUFFER_CAPACITY],
                    |mut output| {
                        let limit =
                            append_with_safe_slices(&input, &mut output, width);
                        black_box((output, limit));
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

/// Benchmarks the `Buffer::copy_to` consume path against safe window copies.
fn benchmark_buffer_consume(criterion: &mut Criterion) {
    let input = benchmark_fixture();
    let mut group = criterion.benchmark_group("buffer_core/buffer_consume");
    group.measurement_time(std::time::Duration::from_secs(2));
    group.sample_size(10);
    group.throughput(Throughput::Bytes(BUFFER_CAPACITY as u64));

    for width in COPY_WIDTHS {
        group.bench_with_input(
            BenchmarkId::new("buffer_unchecked", width),
            &width,
            |bencher, &width| {
                bencher.iter_batched(
                    || {
                        let mut buffer = Buffer::with_capacity(BUFFER_CAPACITY);
                        // SAFETY: the fixture has at least `BUFFER_CAPACITY`
                        // items and the new buffer has the same spare capacity.
                        unsafe {
                            buffer.copy_from(&input, 0, BUFFER_CAPACITY);
                        }
                        (buffer, vec![0_u8; BUFFER_CAPACITY])
                    },
                    |(mut buffer, mut output)| {
                        copy_from_buffer(&mut buffer, &mut output, width);
                        black_box((buffer.available(), output));
                    },
                    BatchSize::SmallInput,
                );
            },
        );
        group.bench_with_input(
            BenchmarkId::new("safe_slices", width),
            &width,
            |bencher, &width| {
                bencher.iter_batched(
                    || vec![0_u8; BUFFER_CAPACITY],
                    |mut output| {
                        copy_from_safe_slices(
                            &input[..BUFFER_CAPACITY],
                            &mut output,
                            width,
                        );
                        black_box(output);
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    benchmark_visible_invariants,
    benchmark_runtime_indexes,
    benchmark_buffer_append,
    benchmark_buffer_consume,
);
criterion_main!(benches);
