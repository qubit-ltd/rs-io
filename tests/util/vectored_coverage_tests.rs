// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;
use std::io::IoSlice;
use std::io::IoSliceMut;
use std::io::Read;
use std::io::Write;

use qubit_io::Input;
use qubit_io::Output;
use qubit_io::StdRead;
use qubit_io::StdWrite;
use qubit_io::coverage_fail_next_reserve;
use qubit_io::coverage_reset_reserve_hooks;

struct PanicReader;

impl Read for PanicReader {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        panic!("underlying scalar read must not run")
    }

    fn read_vectored(&mut self, _: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        panic!("underlying vectored read must not run")
    }
}

struct PanicWriter;

impl Write for PanicWriter {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        panic!("underlying scalar write must not run")
    }

    fn write_vectored(&mut self, _: &[IoSlice<'_>]) -> io::Result<usize> {
        panic!("underlying vectored write must not run")
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct PanicInput;

impl Input for PanicInput {
    type Item = u8;

    unsafe fn read_unchecked(&mut self, _: &mut [u8], _: usize, _: usize) -> io::Result<usize> {
        panic!("underlying scalar input must not run")
    }

    fn read_vectored(&mut self, _: &mut [&mut [u8]]) -> io::Result<usize> {
        panic!("underlying vectored input must not run")
    }
}

struct PanicOutput;

impl Output for PanicOutput {
    type Item = u8;

    unsafe fn write_unchecked(&mut self, _: &[u8], _: usize, _: usize) -> io::Result<usize> {
        panic!("underlying scalar output must not run")
    }

    fn write_vectored(&mut self, _: &[&[u8]]) -> io::Result<usize> {
        panic!("underlying vectored output must not run")
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn assert_allocation_error(result: io::Result<usize>) {
    let error = result.expect_err("injected reserve failure should be returned");
    assert_eq!(error.kind(), io::ErrorKind::OutOfMemory);
}

#[test]
fn vectored_descriptor_reserve_failures_prevent_underlying_io() {
    let mut std_reader = PanicReader;
    let mut read_a = [0_u8; 1];
    let mut read_b = [0_u8; 1];
    coverage_fail_next_reserve();
    assert_allocation_error(Input::read_vectored(&mut std_reader, &mut [&mut read_a, &mut read_b]));
    coverage_reset_reserve_hooks();

    let mut std_writer = PanicWriter;
    coverage_fail_next_reserve();
    assert_allocation_error(Output::write_vectored(&mut std_writer, &[b"a", b"b"]));
    coverage_reset_reserve_hooks();

    let mut std_reader = StdRead::new(PanicInput);
    let mut first = [0_u8; 1];
    let mut second = [0_u8; 1];
    let mut std_buffers = [IoSliceMut::new(&mut first), IoSliceMut::new(&mut second)];
    coverage_fail_next_reserve();
    assert_allocation_error(Read::read_vectored(&mut std_reader, &mut std_buffers));
    coverage_reset_reserve_hooks();

    let mut std_writer = StdWrite::new(PanicOutput);
    let std_buffers = [IoSlice::new(b"a"), IoSlice::new(b"b")];
    coverage_fail_next_reserve();
    assert_allocation_error(Write::write_vectored(&mut std_writer, &std_buffers));
    coverage_reset_reserve_hooks();
}
