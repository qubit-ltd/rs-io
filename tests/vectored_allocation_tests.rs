// =============================================================================
//    Copyright (c) 2025 - 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::alloc::GlobalAlloc;
use std::alloc::Layout;
use std::alloc::System;
use std::cell::Cell;
use std::io::Cursor;
use std::io::IoSlice;
use std::io::IoSliceMut;

use qubit_io::Input;
use qubit_io::Output;
use qubit_io::StdRead;
use qubit_io::StdWrite;

thread_local! {
    static TRACKING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

struct CountingAllocator;

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

// SAFETY: All operations are forwarded unchanged to the system allocator.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        // SAFETY: This allocator forwards the unchanged layout to System.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: This allocator forwards the unchanged pointer and layout to System.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count_allocation();
        // SAFETY: This allocator forwards the unchanged arguments to System.
        unsafe { System.realloc(pointer, layout, new_size) }
    }
}

fn count_allocation() {
    if TRACKING.try_with(Cell::get).unwrap_or(false) {
        let _ = ALLOCATIONS.try_with(|allocations| allocations.set(allocations.get() + 1));
    }
}

struct TrackingGuard;

impl Drop for TrackingGuard {
    fn drop(&mut self) {
        TRACKING.with(|tracking| tracking.set(false));
    }
}

fn allocations_during(operation: impl FnOnce()) -> usize {
    TRACKING.with(|tracking| tracking.set(false));
    ALLOCATIONS.with(|allocations| allocations.set(0));
    TRACKING.with(|tracking| tracking.set(true));
    let guard = TrackingGuard;
    operation();
    drop(guard);
    ALLOCATIONS.with(Cell::get)
}

#[test]
fn single_descriptor_bridges_do_not_allocate_descriptor_storage() {
    let mut input = Cursor::new(*b"data");
    let mut destination = [0_u8; 4];
    let count = allocations_during(|| {
        let mut buffers = [&mut destination[..]];
        assert_eq!(4, Input::read_vectored(&mut input, &mut buffers).unwrap());
    });
    assert_eq!(0, count);

    let mut output = Cursor::new([0_u8; 4]);
    let count = allocations_during(|| {
        assert_eq!(4, Output::write_vectored(&mut output, &[&b"data"[..]]).unwrap());
    });
    assert_eq!(0, count);

    let mut adapter = StdRead::new(Cursor::new(*b"data"));
    let mut destination = [0_u8; 4];
    let count = allocations_during(|| {
        let mut buffers = [IoSliceMut::new(&mut destination)];
        std::io::Read::read_vectored(&mut adapter, &mut buffers).unwrap();
    });
    assert_eq!(0, count);

    let mut adapter = StdWrite::new(Cursor::new([0_u8; 4]));
    let count = allocations_during(|| {
        std::io::Write::write_vectored(&mut adapter, &[IoSlice::new(b"data")]).unwrap();
    });
    assert_eq!(0, count);
}
