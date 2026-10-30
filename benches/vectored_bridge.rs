// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Benchmarks standard and Qubit vectored I/O bridge paths.

use std::alloc::GlobalAlloc;
use std::alloc::Layout;
use std::alloc::System;
use std::cell::Cell;
use std::hint::black_box;
use std::io::Cursor;
use std::io::IoSlice;
use std::io::IoSliceMut;
use std::io::Read;
use std::io::Write;
use std::time::Duration;

use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;
use criterion::criterion_group;
use criterion::criterion_main;
use qubit_io::Input;
use qubit_io::Output;
use qubit_io::StdRead;
use qubit_io::StdWrite;

const DESCRIPTOR_COUNTS: [usize; 6] = [0, 1, 2, 4, 8, 16];
const SLICE_SIZES: [usize; 2] = [64, 4096];

thread_local! {
    static ALLOCATION_COUNT: Cell<usize> = const { Cell::new(0) };
    static COUNT_ALLOCATIONS: Cell<bool> = const { Cell::new(false) };
}

struct CountingAllocator;

// SAFETY: Every operation is forwarded to System with the same pointer,
// layout, and allocation size. The thread-local counter only records calls.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        // SAFETY: The caller provides a valid allocation layout.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        // SAFETY: The caller provides a valid allocation layout.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: The caller provides the pointer and layout from an earlier
        // allocation, and this allocator forwards both unchanged to System.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count_allocation();
        // SAFETY: The caller provides a pointer allocated with `layout`; the
        // request and its result are forwarded unchanged to System.
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn count_allocation() {
    let enabled = COUNT_ALLOCATIONS.try_with(Cell::get).unwrap_or(false);
    if enabled {
        let _ = ALLOCATION_COUNT.try_with(|count| count.set(count.get() + 1));
    }
}

/// Runs one operation while counting allocations made by the current thread.
fn count_operation_allocations(operation: impl FnOnce()) -> usize {
    ALLOCATION_COUNT.with(|count| count.set(0));
    COUNT_ALLOCATIONS.with(|enabled| enabled.set(true));
    operation();
    COUNT_ALLOCATIONS.with(|enabled| enabled.set(false));
    ALLOCATION_COUNT.with(Cell::get)
}

/// Returns the benchmark cases, including a multi-descriptor request with one
/// non-empty descriptor.
fn cases() -> Vec<(usize, usize, bool)> {
    let mut result = Vec::with_capacity(DESCRIPTOR_COUNTS.len() * SLICE_SIZES.len() + 1);
    for descriptor_count in DESCRIPTOR_COUNTS {
        for slice_size in SLICE_SIZES {
            result.push((descriptor_count, slice_size, false));
        }
    }
    result.push((4, 64, true));
    result
}

fn case_name(descriptor_count: usize, slice_size: usize, one_nonempty: bool) -> String {
    if one_nonempty {
        "one_nonempty_of_4/64".to_owned()
    } else {
        format!("{descriptor_count}/{slice_size}")
    }
}

/// Builds deterministic bytes for one request's non-empty slices.
fn source_bytes(descriptor_count: usize, slice_size: usize, one_nonempty: bool) -> Vec<u8> {
    let length = if one_nonempty {
        slice_size
    } else {
        descriptor_count * slice_size
    };
    (0..length)
        .map(|index| (index as u8).wrapping_mul(37).wrapping_add(11))
        .collect()
}

#[inline]
fn total_capacity(descriptor_count: usize, slice_size: usize) -> usize {
    descriptor_count * slice_size
}

fn make_mut_slices(
    storage: &mut [u8],
    descriptor_count: usize,
    slice_size: usize,
    one_nonempty: bool,
) -> Vec<&mut [u8]> {
    let mut rest = storage;
    let mut result = Vec::with_capacity(descriptor_count);
    for index in 0..descriptor_count {
        let (piece, tail) = rest.split_at_mut(slice_size);
        rest = tail;
        result.push(if one_nonempty && index != 2 {
            &mut piece[..0]
        } else {
            piece
        });
    }
    result
}

fn make_slices(storage: &[u8], descriptor_count: usize, slice_size: usize, one_nonempty: bool) -> Vec<&[u8]> {
    if one_nonempty {
        return (0..descriptor_count)
            .map(|index| if index == 2 { storage } else { &storage[..0] })
            .collect();
    }
    (0..descriptor_count)
        .map(|index| {
            let start = index * slice_size;
            &storage[start..start + slice_size]
        })
        .collect()
}

fn assert_transfer(count: usize, requested: usize, path: &str, name: &str) {
    if requested == 0 {
        assert_eq!(count, 0, "{path} {name}: empty request must transfer zero bytes");
    } else {
        assert!(
            count > 0 && count <= requested,
            "{path} {name}: invalid transfer count {count}/{requested}"
        );
    }
}

fn assert_read_prefix(slices: &[&[u8]], source: &[u8], count: usize, path: &str, name: &str) {
    let mut remaining = count;
    let mut offset = 0;
    for slice in slices {
        let length = remaining.min(slice.len());
        assert_eq!(
            &slice[..length],
            &source[offset..offset + length],
            "{path} {name}: read prefix differs"
        );
        offset += length;
        remaining -= length;
    }
    assert_eq!(
        remaining, 0,
        "{path} {name}: descriptors did not contain the transferred prefix"
    );
}

fn assert_write_prefix(output: &[u8], slices: &[&[u8]], count: usize, path: &str, name: &str) {
    let mut expected = Vec::with_capacity(count);
    for slice in slices {
        let length = (count - expected.len()).min(slice.len());
        expected.extend_from_slice(&slice[..length]);
        if expected.len() == count {
            break;
        }
    }
    assert_eq!(&output[..count], expected, "{path} {name}: write prefix differs");
}

/// Confirms every read route transfers the same prefix for one benchmark case.
fn assert_read_paths_match(descriptor_count: usize, slice_size: usize, one_nonempty: bool) {
    let name = case_name(descriptor_count, slice_size, one_nonempty);
    let source = source_bytes(descriptor_count, slice_size, one_nonempty);
    let requested = source.len();

    let mut native_storage = vec![0_u8; total_capacity(descriptor_count, slice_size)];
    let native_slices = make_mut_slices(&mut native_storage, descriptor_count, slice_size, one_nonempty);
    let mut native_buffers: Vec<IoSliceMut<'_>> = native_slices.into_iter().map(IoSliceMut::new).collect();
    let mut native_input = Cursor::new(source.as_slice());
    let native_count = Read::read_vectored(&mut native_input, &mut native_buffers).expect("native read should succeed");
    assert_transfer(native_count, requested, "native", &name);
    let native_observed: Vec<&[u8]> = native_buffers.iter().map(|slice| &slice[..]).collect();
    assert_read_prefix(&native_observed, &source, native_count, "native", &name);

    let mut blanket_storage = vec![0_u8; total_capacity(descriptor_count, slice_size)];
    let mut blanket_buffers = make_mut_slices(&mut blanket_storage, descriptor_count, slice_size, one_nonempty);
    let mut blanket_input = Cursor::new(source.as_slice());
    let blanket_count =
        Input::read_vectored(&mut blanket_input, &mut blanket_buffers).expect("blanket read should succeed");
    assert_transfer(blanket_count, requested, "blanket", &name);
    let blanket_observed: Vec<&[u8]> = blanket_buffers.iter().map(|slice| &slice[..]).collect();
    assert_read_prefix(&blanket_observed, &source, blanket_count, "blanket", &name);

    let mut roundtrip_storage = vec![0_u8; total_capacity(descriptor_count, slice_size)];
    let roundtrip_slices = make_mut_slices(&mut roundtrip_storage, descriptor_count, slice_size, one_nonempty);
    let mut roundtrip_buffers: Vec<IoSliceMut<'_>> = roundtrip_slices.into_iter().map(IoSliceMut::new).collect();
    let mut roundtrip_input = StdRead::new(Cursor::new(source.as_slice()));
    let roundtrip_count =
        Read::read_vectored(&mut roundtrip_input, &mut roundtrip_buffers).expect("roundtrip read should succeed");
    assert_transfer(roundtrip_count, requested, "roundtrip", &name);
    let roundtrip_observed: Vec<&[u8]> = roundtrip_buffers.iter().map(|slice| &slice[..]).collect();
    assert_read_prefix(&roundtrip_observed, &source, roundtrip_count, "roundtrip", &name);

    assert_eq!(
        native_count, blanket_count,
        "{name}: native and blanket read counts differ"
    );
    assert_eq!(
        native_count, roundtrip_count,
        "{name}: native and roundtrip read counts differ"
    );
}

/// Confirms every write route transfers the same prefix for one benchmark case.
fn assert_write_paths_match(descriptor_count: usize, slice_size: usize, one_nonempty: bool) {
    let name = case_name(descriptor_count, slice_size, one_nonempty);
    let input_bytes = source_bytes(descriptor_count, slice_size, one_nonempty);
    let requested = input_bytes.len();
    let capacity = total_capacity(descriptor_count, slice_size);
    let slices = make_slices(&input_bytes, descriptor_count, slice_size, one_nonempty);

    let native_buffers: Vec<IoSlice<'_>> = slices.iter().map(|slice| IoSlice::new(slice)).collect();
    let mut native_storage = vec![0_u8; capacity];
    let mut native_output = Cursor::new(native_storage.as_mut_slice());
    let native_count = Write::write_vectored(&mut native_output, &native_buffers).expect("native write should succeed");
    assert_transfer(native_count, requested, "native", &name);
    assert_write_prefix(native_output.get_ref(), &slices, native_count, "native", &name);

    let mut blanket_storage = vec![0_u8; capacity];
    let mut blanket_output = Cursor::new(blanket_storage.as_mut_slice());
    let blanket_count = Output::write_vectored(&mut blanket_output, &slices).expect("blanket write should succeed");
    assert_transfer(blanket_count, requested, "blanket", &name);
    assert_write_prefix(blanket_output.get_ref(), &slices, blanket_count, "blanket", &name);

    let roundtrip_buffers: Vec<IoSlice<'_>> = slices.iter().map(|slice| IoSlice::new(slice)).collect();
    let mut roundtrip_storage = vec![0_u8; capacity];
    let mut roundtrip_output = StdWrite::new(Cursor::new(roundtrip_storage.as_mut_slice()));
    let roundtrip_count =
        Write::write_vectored(&mut roundtrip_output, &roundtrip_buffers).expect("roundtrip write should succeed");
    assert_transfer(roundtrip_count, requested, "roundtrip", &name);
    assert_write_prefix(
        roundtrip_output.get_ref().get_ref(),
        &slices,
        roundtrip_count,
        "roundtrip",
        &name,
    );

    assert_eq!(
        native_count, blanket_count,
        "{name}: native and blanket write counts differ"
    );
    assert_eq!(
        native_count, roundtrip_count,
        "{name}: native and roundtrip write counts differ"
    );
}

/// Benchmarks direct standard `Read::read_vectored` calls.
fn benchmark_native_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("vectored_bridge/read/native");
    group.sample_size(100);
    group.measurement_time(Duration::from_secs(2));
    for (descriptor_count, slice_size, one_nonempty) in cases() {
        assert_read_paths_match(descriptor_count, slice_size, one_nonempty);
        let name = case_name(descriptor_count, slice_size, one_nonempty);
        let source = source_bytes(descriptor_count, slice_size, one_nonempty);
        let requested = source.len();
        let mut storage = vec![0_u8; total_capacity(descriptor_count, slice_size)];
        let slices = make_mut_slices(&mut storage, descriptor_count, slice_size, one_nonempty);
        let mut buffers: Vec<IoSliceMut<'_>> = slices.into_iter().map(IoSliceMut::new).collect();
        let mut input = Cursor::new(source.as_slice());
        input.set_position(0);
        let count = Read::read_vectored(&mut input, &mut buffers).expect("native read should succeed");
        assert_transfer(count, requested, "native", &name);
        let observed: Vec<&[u8]> = buffers.iter().map(|slice| &slice[..]).collect();
        assert_read_prefix(&observed, &source, count, "native", &name);
        let allocations = count_operation_allocations(|| {
            input.set_position(0);
            let count = Read::read_vectored(&mut input, &mut buffers).expect("native read should succeed");
            black_box(count);
        });
        eprintln!("allocations read/native/{name}: {allocations}");
        group.throughput(Throughput::Bytes(count as u64));
        group.bench_with_input(BenchmarkId::new("descriptors", &name), &(), |bencher, ()| {
            bencher.iter(|| {
                input.set_position(0);
                let count =
                    Read::read_vectored(&mut input, black_box(&mut buffers)).expect("native read should succeed");
                black_box(count);
                black_box(&buffers);
            });
        });
    }
    group.finish();
}

/// Benchmarks the standard `Read` blanket implementation for `Input`.
fn benchmark_blanket_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("vectored_bridge/read/blanket");
    group.sample_size(100);
    group.measurement_time(Duration::from_secs(2));
    for (descriptor_count, slice_size, one_nonempty) in cases() {
        assert_read_paths_match(descriptor_count, slice_size, one_nonempty);
        let name = case_name(descriptor_count, slice_size, one_nonempty);
        let source = source_bytes(descriptor_count, slice_size, one_nonempty);
        let requested = source.len();
        let mut storage = vec![0_u8; total_capacity(descriptor_count, slice_size)];
        let mut buffers = make_mut_slices(&mut storage, descriptor_count, slice_size, one_nonempty);
        let mut input = Cursor::new(source.as_slice());
        input.set_position(0);
        let count = Input::read_vectored(&mut input, &mut buffers).expect("blanket read should succeed");
        assert_transfer(count, requested, "blanket", &name);
        let observed: Vec<&[u8]> = buffers.iter().map(|slice| &slice[..]).collect();
        assert_read_prefix(&observed, &source, count, "blanket", &name);
        let allocations = count_operation_allocations(|| {
            input.set_position(0);
            let count = Input::read_vectored(&mut input, black_box(&mut buffers)).expect("blanket read should succeed");
            black_box(count);
        });
        eprintln!("allocations read/blanket/{name}: {allocations}");
        group.throughput(Throughput::Bytes(count as u64));
        group.bench_with_input(BenchmarkId::new("descriptors", &name), &(), |bencher, ()| {
            bencher.iter(|| {
                input.set_position(0);
                let count =
                    Input::read_vectored(&mut input, black_box(&mut buffers)).expect("blanket read should succeed");
                black_box(count);
                black_box(&buffers);
            });
        });
    }
    group.finish();
}

/// Benchmarks a standard read through `StdRead` and back to `Read`.
fn benchmark_roundtrip_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("vectored_bridge/read/roundtrip");
    group.sample_size(100);
    group.measurement_time(Duration::from_secs(2));
    for (descriptor_count, slice_size, one_nonempty) in cases() {
        assert_read_paths_match(descriptor_count, slice_size, one_nonempty);
        let name = case_name(descriptor_count, slice_size, one_nonempty);
        let source = source_bytes(descriptor_count, slice_size, one_nonempty);
        let requested = source.len();
        let mut storage = vec![0_u8; total_capacity(descriptor_count, slice_size)];
        let slices = make_mut_slices(&mut storage, descriptor_count, slice_size, one_nonempty);
        let mut buffers: Vec<IoSliceMut<'_>> = slices.into_iter().map(IoSliceMut::new).collect();
        let mut input = StdRead::new(Cursor::new(source.as_slice()));
        input.get_mut().set_position(0);
        let count = Read::read_vectored(&mut input, &mut buffers).expect("roundtrip read should succeed");
        assert_transfer(count, requested, "roundtrip", &name);
        let observed: Vec<&[u8]> = buffers.iter().map(|slice| &slice[..]).collect();
        assert_read_prefix(&observed, &source, count, "roundtrip", &name);
        let allocations = count_operation_allocations(|| {
            input.get_mut().set_position(0);
            let count =
                Read::read_vectored(&mut input, black_box(&mut buffers)).expect("roundtrip read should succeed");
            black_box(count);
        });
        eprintln!("allocations read/roundtrip/{name}: {allocations}");
        group.throughput(Throughput::Bytes(count as u64));
        group.bench_with_input(BenchmarkId::new("descriptors", &name), &(), |bencher, ()| {
            bencher.iter(|| {
                input.get_mut().set_position(0);
                let count =
                    Read::read_vectored(&mut input, black_box(&mut buffers)).expect("roundtrip read should succeed");
                black_box(count);
                black_box(&buffers);
            });
        });
    }
    group.finish();
}

/// Benchmarks direct standard `Write::write_vectored` calls.
fn benchmark_native_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("vectored_bridge/write/native");
    group.sample_size(100);
    group.measurement_time(Duration::from_secs(2));
    for (descriptor_count, slice_size, one_nonempty) in cases() {
        assert_write_paths_match(descriptor_count, slice_size, one_nonempty);
        let name = case_name(descriptor_count, slice_size, one_nonempty);
        let input_bytes = source_bytes(descriptor_count, slice_size, one_nonempty);
        let slices = make_slices(&input_bytes, descriptor_count, slice_size, one_nonempty);
        let buffers: Vec<IoSlice<'_>> = slices.iter().map(|slice| IoSlice::new(slice)).collect();
        let requested = input_bytes.len();
        let mut storage = vec![0_u8; total_capacity(descriptor_count, slice_size)];
        let mut output = Cursor::new(storage.as_mut_slice());
        output.set_position(0);
        let count = Write::write_vectored(&mut output, &buffers).expect("native write should succeed");
        assert_transfer(count, requested, "native", &name);
        assert_write_prefix(output.get_ref(), &slices, count, "native", &name);
        let allocations = count_operation_allocations(|| {
            output.set_position(0);
            let count = Write::write_vectored(&mut output, black_box(&buffers)).expect("native write should succeed");
            black_box(count);
        });
        eprintln!("allocations write/native/{name}: {allocations}");
        group.throughput(Throughput::Bytes(count as u64));
        group.bench_with_input(BenchmarkId::new("descriptors", &name), &(), |bencher, ()| {
            bencher.iter(|| {
                output.set_position(0);
                let count =
                    Write::write_vectored(&mut output, black_box(&buffers)).expect("native write should succeed");
                black_box(count);
                black_box(output.get_ref());
            });
        });
    }
    group.finish();
}

/// Benchmarks the standard `Write` blanket implementation for `Output`.
fn benchmark_blanket_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("vectored_bridge/write/blanket");
    group.sample_size(100);
    group.measurement_time(Duration::from_secs(2));
    for (descriptor_count, slice_size, one_nonempty) in cases() {
        assert_write_paths_match(descriptor_count, slice_size, one_nonempty);
        let name = case_name(descriptor_count, slice_size, one_nonempty);
        let input_bytes = source_bytes(descriptor_count, slice_size, one_nonempty);
        let buffers = make_slices(&input_bytes, descriptor_count, slice_size, one_nonempty);
        let requested = input_bytes.len();
        let mut storage = vec![0_u8; total_capacity(descriptor_count, slice_size)];
        let mut output = Cursor::new(storage.as_mut_slice());
        output.set_position(0);
        let count = Output::write_vectored(&mut output, &buffers).expect("blanket write should succeed");
        assert_transfer(count, requested, "blanket", &name);
        assert_write_prefix(output.get_ref(), &buffers, count, "blanket", &name);
        let allocations = count_operation_allocations(|| {
            output.set_position(0);
            let count = Output::write_vectored(&mut output, black_box(&buffers)).expect("blanket write should succeed");
            black_box(count);
        });
        eprintln!("allocations write/blanket/{name}: {allocations}");
        group.throughput(Throughput::Bytes(count as u64));
        group.bench_with_input(BenchmarkId::new("descriptors", &name), &(), |bencher, ()| {
            bencher.iter(|| {
                output.set_position(0);
                let count =
                    Output::write_vectored(&mut output, black_box(&buffers)).expect("blanket write should succeed");
                black_box(count);
                black_box(output.get_ref());
            });
        });
    }
    group.finish();
}

/// Benchmarks a standard write through `StdWrite` and back to `Write`.
fn benchmark_roundtrip_write(c: &mut Criterion) {
    let mut group = c.benchmark_group("vectored_bridge/write/roundtrip");
    group.sample_size(100);
    group.measurement_time(Duration::from_secs(2));
    for (descriptor_count, slice_size, one_nonempty) in cases() {
        assert_write_paths_match(descriptor_count, slice_size, one_nonempty);
        let name = case_name(descriptor_count, slice_size, one_nonempty);
        let input_bytes = source_bytes(descriptor_count, slice_size, one_nonempty);
        let slices = make_slices(&input_bytes, descriptor_count, slice_size, one_nonempty);
        let buffers: Vec<IoSlice<'_>> = slices.iter().map(|slice| IoSlice::new(slice)).collect();
        let requested = input_bytes.len();
        let mut storage = vec![0_u8; total_capacity(descriptor_count, slice_size)];
        let mut output = StdWrite::new(Cursor::new(storage.as_mut_slice()));
        output.get_mut().set_position(0);
        let count = Write::write_vectored(&mut output, &buffers).expect("roundtrip write should succeed");
        assert_transfer(count, requested, "roundtrip", &name);
        assert_write_prefix(output.get_ref().get_ref(), &slices, count, "roundtrip", &name);
        let allocations = count_operation_allocations(|| {
            output.get_mut().set_position(0);
            let count =
                Write::write_vectored(&mut output, black_box(&buffers)).expect("roundtrip write should succeed");
            black_box(count);
        });
        eprintln!("allocations write/roundtrip/{name}: {allocations}");
        group.throughput(Throughput::Bytes(count as u64));
        group.bench_with_input(BenchmarkId::new("descriptors", &name), &(), |bencher, ()| {
            bencher.iter(|| {
                output.get_mut().set_position(0);
                let count =
                    Write::write_vectored(&mut output, black_box(&buffers)).expect("roundtrip write should succeed");
                black_box(count);
                black_box(output.get_ref().get_ref());
            });
        });
    }
    group.finish();
}

fn benchmark_vectored_bridge(c: &mut Criterion) {
    benchmark_native_read(c);
    benchmark_blanket_read(c);
    benchmark_roundtrip_read(c);
    benchmark_native_write(c);
    benchmark_blanket_write(c);
    benchmark_roundtrip_write(c);
}

criterion_group!(benches, benchmark_vectored_bridge);
criterion_main!(benches);
