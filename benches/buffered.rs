// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Microbenchmarks for synchronous buffered item I/O.
//!
//! Run with `cargo bench --bench buffered`. The benchmark keeps fixture setup
//! outside the measured closure and varies transfer widths to exercise both
//! buffered copies, the large-transfer direct path, and standard-library
//! buffered and unbuffered baselines.

use std::hint::black_box;
use std::io::{BufReader, BufWriter, Cursor, Read, Write};

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use qubit_io::{BufferedInput, BufferedOutput};

const BUFFER_CAPACITY: usize = 8 * 1024;
const DATA_LEN: usize = 256 * 1024;
const TRANSFER_WIDTHS: [usize; 5] = [1, 4, 8, 64, BUFFER_CAPACITY];

/// Creates a deterministic byte fixture for the buffered benchmarks.
fn benchmark_fixture() -> Vec<u8> {
    (0..DATA_LEN)
        .map(|index| (index as u8).wrapping_mul(31))
        .collect()
}

/// Benchmarks sequential reads at widths that hit buffered and direct paths.
fn benchmark_buffered_input(criterion: &mut Criterion) {
    let fixture = benchmark_fixture();
    let mut group = criterion.benchmark_group("buffered_input");
    group.measurement_time(std::time::Duration::from_secs(2));
    group.sample_size(10);
    group.throughput(Throughput::Bytes(DATA_LEN as u64));

    for width in TRANSFER_WIDTHS {
        group.bench_with_input(
            BenchmarkId::new("qubit", width),
            &width,
            |bencher, &width| {
                bencher.iter_batched(
                    || BufferedInput::with_capacity(Cursor::new(fixture.clone()), BUFFER_CAPACITY),
                    |mut input| {
                        let mut output = [0_u8; BUFFER_CAPACITY];
                        let mut total = 0_usize;
                        loop {
                            let count = input
                                .read(&mut output[..width])
                                .expect("buffered input benchmark read");
                            if count == 0 {
                                break;
                            }
                            total += count;
                        }
                        black_box((total, output));
                    },
                    BatchSize::SmallInput,
                );
            },
        );
        group.bench_with_input(
            BenchmarkId::new("std_bufreader", width),
            &width,
            |bencher, &width| {
                bencher.iter_batched(
                    || BufReader::with_capacity(BUFFER_CAPACITY, Cursor::new(fixture.clone())),
                    |mut input| {
                        let mut output = [0_u8; BUFFER_CAPACITY];
                        let mut total = 0_usize;
                        loop {
                            let count = input
                                .read(&mut output[..width])
                                .expect("std buffered input benchmark read");
                            if count == 0 {
                                break;
                            }
                            total += count;
                        }
                        black_box((total, output));
                    },
                    BatchSize::SmallInput,
                );
            },
        );
        group.bench_with_input(
            BenchmarkId::new("unbuffered", width),
            &width,
            |bencher, &width| {
                bencher.iter_batched(
                    || Cursor::new(fixture.clone()),
                    |mut input| {
                        let mut output = [0_u8; BUFFER_CAPACITY];
                        let mut total = 0_usize;
                        loop {
                            let count = input
                                .read(&mut output[..width])
                                .expect("unbuffered input benchmark read");
                            if count == 0 {
                                break;
                            }
                            total += count;
                        }
                        black_box((total, output));
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

/// Benchmarks sequential writes at widths that hit buffered and direct paths.
fn benchmark_buffered_output(criterion: &mut Criterion) {
    let fixture = benchmark_fixture();
    let mut group = criterion.benchmark_group("buffered_output");
    group.measurement_time(std::time::Duration::from_secs(2));
    group.sample_size(10);
    group.throughput(Throughput::Bytes(DATA_LEN as u64));

    for width in TRANSFER_WIDTHS {
        group.bench_with_input(
            BenchmarkId::new("qubit", width),
            &width,
            |bencher, &width| {
                bencher.iter_batched(
                    || {
                        BufferedOutput::with_capacity(
                            Cursor::new(Vec::with_capacity(DATA_LEN)),
                            BUFFER_CAPACITY,
                        )
                    },
                    |mut output| {
                        for chunk in fixture.chunks_exact(width) {
                            output
                                .write_fully(chunk)
                                .expect("buffered output benchmark write");
                        }
                        output.flush().expect("buffered output benchmark flush");
                        let (writer, pending) = output.into_parts();
                        let bytes = writer.into_inner();
                        black_box((bytes, pending.available()));
                    },
                    BatchSize::SmallInput,
                );
            },
        );
        group.bench_with_input(
            BenchmarkId::new("std_bufwriter", width),
            &width,
            |bencher, &width| {
                bencher.iter_batched(
                    || {
                        BufWriter::with_capacity(
                            BUFFER_CAPACITY,
                            Cursor::new(Vec::with_capacity(DATA_LEN)),
                        )
                    },
                    |mut output| {
                        for chunk in fixture.chunks_exact(width) {
                            output
                                .write_all(chunk)
                                .expect("std buffered output benchmark write");
                        }
                        output.flush().expect("std buffered output benchmark flush");
                        let bytes = output
                            .into_inner()
                            .expect("std buffered output should flush into cursor")
                            .into_inner();
                        black_box(bytes);
                    },
                    BatchSize::SmallInput,
                );
            },
        );
        group.bench_with_input(
            BenchmarkId::new("unbuffered", width),
            &width,
            |bencher, &width| {
                bencher.iter_batched(
                    || Cursor::new(Vec::with_capacity(DATA_LEN)),
                    |mut output| {
                        for chunk in fixture.chunks_exact(width) {
                            output
                                .write_all(chunk)
                                .expect("unbuffered output benchmark write");
                        }
                        output.flush().expect("unbuffered output benchmark flush");
                        black_box(output.into_inner());
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    group.finish();
}

criterion_group!(benches, benchmark_buffered_input, benchmark_buffered_output,);
criterion_main!(benches);
