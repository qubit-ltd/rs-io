// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::io::Cursor;
use std::io::ErrorKind;
use std::io::IoSlice;
use std::io::IoSliceMut;
use std::io::Read;
use std::io::Write;

use qubit_io::BufferedInput;
use qubit_io::BufferedOutput;
use qubit_io::ChecksumInput;
use qubit_io::ChecksumOutput;
use qubit_io::CountingInput;
use qubit_io::CountingOutput;
#[cfg(feature = "digest")]
use qubit_io::DigestInput;
#[cfg(feature = "digest")]
use qubit_io::DigestOutput;
use qubit_io::Input;
use qubit_io::LimitInput;
use qubit_io::LimitOutput;
use qubit_io::Output;
use qubit_io::StdRead;
use qubit_io::StdWrite;
use qubit_io::TeeInput;
use qubit_io::TeeOutput;
#[cfg(feature = "digest")]
use sha2::Digest;
#[cfg(feature = "digest")]
use sha2::Sha256;

#[derive(Clone, Copy)]
enum BridgeReply {
    Count(usize),
    Error(ErrorKind),
}

struct BridgeReader {
    reply: BridgeReply,
    calls: usize,
}

impl Read for BridgeReader {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        panic!("scalar read must not be called")
    }

    fn read_vectored(&mut self, buffers: &mut [IoSliceMut<'_>]) -> std::io::Result<usize> {
        self.calls += 1;
        match self.reply {
            BridgeReply::Count(count) => {
                let Some(buffer) = buffers.first_mut() else {
                    return Ok(0);
                };
                let filled = count.min(buffer.len());
                buffer[..filled].fill(b'x');
                Ok(count)
            }
            BridgeReply::Error(kind) => Err(std::io::Error::new(kind, "bridge read payload")),
        }
    }
}

struct BridgeWriter {
    reply: BridgeReply,
    calls: usize,
}

impl Write for BridgeWriter {
    fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
        panic!("scalar write must not be called")
    }

    fn write_vectored(&mut self, _buffers: &[IoSlice<'_>]) -> std::io::Result<usize> {
        self.calls += 1;
        match self.reply {
            BridgeReply::Count(count) => Ok(count),
            BridgeReply::Error(kind) => Err(std::io::Error::new(kind, "bridge write payload")),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct BridgeInput {
    reply: BridgeReply,
    calls: usize,
}

impl Input for BridgeInput {
    type Item = u8;

    unsafe fn read_unchecked(&mut self, _output: &mut [u8], _index: usize, _count: usize) -> std::io::Result<usize> {
        panic!("scalar read must not be called")
    }

    fn read_vectored(&mut self, buffers: &mut [&mut [u8]]) -> std::io::Result<usize> {
        self.calls += 1;
        match self.reply {
            BridgeReply::Count(count) => {
                let Some(buffer) = buffers.first_mut() else {
                    return Ok(0);
                };
                let filled = count.min(buffer.len());
                buffer[..filled].fill(b'x');
                Ok(count)
            }
            BridgeReply::Error(kind) => Err(std::io::Error::new(kind, "adapter read payload")),
        }
    }
}

struct BridgeOutput {
    reply: BridgeReply,
    calls: usize,
}

impl Output for BridgeOutput {
    type Item = u8;

    unsafe fn write_unchecked(&mut self, _input: &[u8], _index: usize, _count: usize) -> std::io::Result<usize> {
        panic!("scalar write must not be called")
    }

    fn write_vectored(&mut self, _buffers: &[&[u8]]) -> std::io::Result<usize> {
        self.calls += 1;
        match self.reply {
            BridgeReply::Count(count) => Ok(count),
            BridgeReply::Error(kind) => Err(std::io::Error::new(kind, "adapter write payload")),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn read_two<I: Input<Item = u8>>(input: &mut I) {
    let mut first = [0; 2];
    let mut second = [b'!'; 2];
    assert_eq!(Input::read_vectored(input, &mut [&mut first, &mut second]).unwrap(), 2);
    assert_eq!(&first, b"ab");
    assert_eq!(&second, b"!!");
}

fn write_two<O: Output<Item = u8>>(output: &mut O) {
    assert_eq!(Output::write_vectored(output, &[b"ab", b"cd"]).unwrap(), 2);
}

#[test]
fn test_single_descriptor_std_reader_bridge_keeps_vectored_dispatch_and_validates_once() {
    let mut reader = BridgeReader {
        reply: BridgeReply::Count(1),
        calls: 0,
    };
    let mut output = [0_u8; 2];
    assert_eq!(Input::read_vectored(&mut reader, &mut [&mut output]).unwrap(), 1);
    assert_eq!(reader.calls, 1);
    assert_eq!(output, [b'x', 0]);

    let mut reader = BridgeReader {
        reply: BridgeReply::Count(3),
        calls: 0,
    };
    let mut output = [0_u8; 2];
    assert_eq!(
        Input::read_vectored(&mut reader, &mut [&mut output])
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidData
    );
    assert_eq!(reader.calls, 1);

    let mut reader = BridgeReader {
        reply: BridgeReply::Error(ErrorKind::PermissionDenied),
        calls: 0,
    };
    let mut output = [0_u8; 2];
    let error = Input::read_vectored(&mut reader, &mut [&mut output]).unwrap_err();
    assert_eq!(ErrorKind::PermissionDenied, error.kind());
    assert_eq!("bridge read payload", error.to_string());
    assert_eq!(reader.calls, 1);
}

#[test]
fn test_single_descriptor_std_writer_bridge_keeps_vectored_dispatch_and_validates_once() {
    let mut writer = BridgeWriter {
        reply: BridgeReply::Count(1),
        calls: 0,
    };
    assert_eq!(Output::write_vectored(&mut writer, &[b"ab"]).unwrap(), 1);
    assert_eq!(writer.calls, 1);

    let mut writer = BridgeWriter {
        reply: BridgeReply::Count(3),
        calls: 0,
    };
    assert_eq!(
        Output::write_vectored(&mut writer, &[b"ab"]).unwrap_err().kind(),
        ErrorKind::InvalidData
    );
    assert_eq!(writer.calls, 1);

    let mut writer = BridgeWriter {
        reply: BridgeReply::Error(ErrorKind::PermissionDenied),
        calls: 0,
    };
    let error = Output::write_vectored(&mut writer, &[b"ab"]).unwrap_err();
    assert_eq!(ErrorKind::PermissionDenied, error.kind());
    assert_eq!("bridge write payload", error.to_string());
    assert_eq!(writer.calls, 1);
}

#[test]
fn test_single_descriptor_std_read_adapter_keeps_vectored_dispatch_and_validates_once() {
    let mut reader = StdRead::new(BridgeInput {
        reply: BridgeReply::Count(1),
        calls: 0,
    });
    let mut output = [0_u8; 2];
    assert_eq!(
        Read::read_vectored(&mut reader, &mut [IoSliceMut::new(&mut output)]).unwrap(),
        1
    );
    assert_eq!(reader.get_ref().calls, 1);
    assert_eq!(output, [b'x', 0]);

    let mut reader = StdRead::new(BridgeInput {
        reply: BridgeReply::Count(3),
        calls: 0,
    });
    let mut output = [0_u8; 2];
    assert_eq!(
        Read::read_vectored(&mut reader, &mut [IoSliceMut::new(&mut output)])
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidData
    );
    assert_eq!(reader.get_ref().calls, 1);

    let mut reader = StdRead::new(BridgeInput {
        reply: BridgeReply::Error(ErrorKind::PermissionDenied),
        calls: 0,
    });
    let mut output = [0_u8; 2];
    let error = Read::read_vectored(&mut reader, &mut [IoSliceMut::new(&mut output)]).unwrap_err();
    assert_eq!(ErrorKind::PermissionDenied, error.kind());
    assert_eq!("adapter read payload", error.to_string());
    assert_eq!(reader.get_ref().calls, 1);
}

#[test]
fn test_single_descriptor_std_write_adapter_keeps_vectored_dispatch_and_validates_once() {
    let mut writer = StdWrite::new(BridgeOutput {
        reply: BridgeReply::Count(1),
        calls: 0,
    });
    assert_eq!(Write::write_vectored(&mut writer, &[IoSlice::new(b"ab")]).unwrap(), 1);
    assert_eq!(writer.get_ref().calls, 1);

    let mut writer = StdWrite::new(BridgeOutput {
        reply: BridgeReply::Count(3),
        calls: 0,
    });
    assert_eq!(
        Write::write_vectored(&mut writer, &[IoSlice::new(b"ab")])
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidData
    );
    assert_eq!(writer.get_ref().calls, 1);

    let mut writer = StdWrite::new(BridgeOutput {
        reply: BridgeReply::Error(ErrorKind::PermissionDenied),
        calls: 0,
    });
    let error = Write::write_vectored(&mut writer, &[IoSlice::new(b"ab")]).unwrap_err();
    assert_eq!(ErrorKind::PermissionDenied, error.kind());
    assert_eq!("adapter write payload", error.to_string());
    assert_eq!(writer.get_ref().calls, 1);
}

#[test]
fn test_vectored_default_keeps_limit_count_buffering_and_tee_semantics() {
    let mut input = CountingInput::new(LimitInput::new(Cursor::new(b"abcdef"), 3));
    let mut first = [0; 4];
    let mut second = [b'!'; 2];
    assert_eq!(
        Input::read_vectored(&mut input, &mut [&mut first, &mut second]).unwrap(),
        3
    );
    assert_eq!(&first, b"abc\0");
    assert_eq!(&second, b"!!");
    assert_eq!(input.items_read(), 3);
    assert_eq!(input.inner().remaining(), 0);
    let mut output = CountingOutput::new(LimitOutput::new(Vec::<u8>::new(), 3));
    assert_eq!(Output::write_vectored(&mut output, &[b"abcd", b"ef"]).unwrap(), 3);
    assert_eq!(output.items_written(), 3);
    assert_eq!(output.inner().remaining(), 0);
    assert_eq!(output.inner().inner(), b"abc");

    let mut input = BufferedInput::with_capacity(Cursor::new(b"abcd"), 4);
    read_two(&mut input);
    let mut suffix = [0; 2];
    Input::read_exactly(&mut input, &mut suffix).unwrap();
    assert_eq!(&suffix, b"cd");
    let mut output = BufferedOutput::with_capacity(Vec::<u8>::new(), 4);
    write_two(&mut output);
    Output::flush(&mut output).unwrap();
    assert_eq!(output.inner(), b"ab");

    let mut input = TeeInput::new(Cursor::new(b"abcd"), Vec::<u8>::new());
    read_two(&mut input);
    assert_eq!(input.branch(), b"ab");
    let mut output = TeeOutput::new(Vec::<u8>::new(), Vec::<u8>::new());
    write_two(&mut output);
    assert_eq!(output.inner(), b"ab");
    assert_eq!(output.branch(), b"ab");
}

#[test]
fn test_vectored_default_updates_checksum_for_transferred_prefix_only() {
    let mut expected = DefaultHasher::new();
    expected.write(b"ab");
    let expected = expected.finish();
    let mut input = ChecksumInput::new(Cursor::new(b"abcd"), DefaultHasher::new());
    read_two(&mut input);
    assert_eq!(input.checksum(), expected);
    let mut output = ChecksumOutput::new(Vec::<u8>::new(), DefaultHasher::new());
    write_two(&mut output);
    assert_eq!(output.checksum(), expected);
    assert_eq!(output.inner(), b"ab");
}

#[cfg(feature = "digest")]
#[test]
fn test_vectored_default_updates_digest_for_transferred_prefix_only() {
    let expected = Sha256::digest(b"ab");
    let mut input = DigestInput::new(Cursor::new(b"abcd"), Sha256::new());
    read_two(&mut input);
    assert_eq!(input.digest(), expected);
    let mut output = DigestOutput::new(Vec::<u8>::new(), Sha256::new());
    write_two(&mut output);
    assert_eq!(output.digest(), expected);
    assert_eq!(output.inner(), b"ab");
}
