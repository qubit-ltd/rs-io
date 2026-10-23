// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::hint::black_box;
use std::io::Cursor;

use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use qubit_io::std_io::ext::BufReadExt;

fn benchmark_bounded_delimiter(c: &mut Criterion) {
    let mut group = c.benchmark_group("bounded_delimiter");
    for input_len in [1024, 1024 * 1024, 16 * 1024 * 1024] {
        let data = vec![b'x'; input_len];
        for limit in [0, 1, 64] {
            group.bench_function(format!("read/{input_len}/{limit}"), |bencher| {
                bencher.iter(|| {
                    let mut input = Cursor::new(black_box(data.as_slice()));
                    let result = input.read_until_limited(black_box(b'\n'), black_box(limit));
                    let error_kind = result.err().map(|error| error.kind());
                    black_box((error_kind, input.position()));
                });
            });
            group.bench_function(format!("discard/{input_len}/{limit}"), |bencher| {
                bencher.iter(|| {
                    let mut input = Cursor::new(black_box(data.as_slice()));
                    let result = input.discard_until_limited(black_box(b'\n'), black_box(limit));
                    let error_kind = result.err().map(|error| error.kind());
                    black_box((error_kind, input.position()));
                });
            });
        }
    }
    group.finish();
}

criterion_group!(benches, benchmark_bounded_delimiter);
criterion_main!(benches);
