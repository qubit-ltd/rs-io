// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::collections::hash_map::DefaultHasher;
use std::io::Cursor;
use std::io::Error;
use std::io::ErrorKind;
use std::io::IoSliceMut;
use std::io::Read;
use std::io::Result;
use std::io::Seek;
use std::io::SeekFrom;

use qubit_io::ChecksumInput;
use qubit_io::Input;
use qubit_io::InputRef;
use qubit_io::LimitOutput;
use qubit_io::Seekable;
use qubit_io::StdRead;
use qubit_io::TeeInput;

struct ScalarOverrideInput {
    reported: usize,
    error: Option<ErrorKind>,
    calls: usize,
}

impl Input for ScalarOverrideInput {
    type Item = u8;

    unsafe fn read_unchecked(&mut self, _: &mut [u8], _: usize, _: usize) -> Result<usize> {
        panic!("StdRead must forward the safe scalar override")
    }

    fn read(&mut self, output: &mut [u8]) -> Result<usize> {
        self.calls += 1;
        if let Some(kind) = self.error {
            return Err(Error::new(kind, "safe scalar read error"));
        }
        let actual = self.reported.min(output.len());
        output[..actual].fill(b'x');
        Ok(self.reported)
    }
}

#[test]
fn test_std_read_scalar_override_validates_count_and_preserves_contract() {
    for (requested, reported, error, expected_error) in [
        (0, 1, None, Some(ErrorKind::InvalidData)),
        (2, 3, None, Some(ErrorKind::InvalidData)),
        (0, 0, None, None),
        (2, 0, None, None),
        (2, 1, None, None),
        (2, 2, None, None),
        (2, 0, Some(ErrorKind::Interrupted), Some(ErrorKind::Interrupted)),
        (
            2,
            0,
            Some(ErrorKind::PermissionDenied),
            Some(ErrorKind::PermissionDenied),
        ),
    ] {
        let mut reader = StdRead::new(ScalarOverrideInput {
            reported,
            error,
            calls: 0,
        });
        let mut bytes = vec![b'!'; requested];
        let result = Read::read(&mut reader, &mut bytes);
        if let Some(kind) = expected_error {
            let returned = result.expect_err("the boundary must report an error");
            assert_eq!(kind, returned.kind());
            if error.is_some() {
                assert_eq!("safe scalar read error", returned.to_string());
            }
        } else {
            assert_eq!(reported, result.expect("valid count must succeed"));
        }
        assert_eq!(1, reader.get_ref().calls, "empty and error requests still forward once");
        let actual = if error.is_some() { 0 } else { reported.min(requested) };
        assert_eq!(vec![b'x'; actual], bytes[..actual]);
        assert_eq!(vec![b'!'; requested - actual], bytes[actual..]);
    }
}

struct VectorInput;

impl Input for VectorInput {
    type Item = u8;

    unsafe fn read_unchecked(&mut self, _: &mut [u8], _: usize, _: usize) -> Result<usize> {
        panic!("scalar read called")
    }
    fn read_vectored(&mut self, buffers: &mut [&mut [u8]]) -> Result<usize> {
        let mut bytes = b"abcd".iter();
        let mut total = 0;
        for buffer in buffers {
            for slot in buffer.iter_mut() {
                let Some(byte) = bytes.next() else {
                    return Ok(total);
                };
                *slot = *byte;
                total += 1;
            }
        }
        Ok(total)
    }
}

#[test]
fn test_vectored_std_adapter_keeps_inner_override() {
    let mut reader = StdRead::new(VectorInput);
    let mut first = [0; 2];
    let mut second = [0; 2];
    assert_eq!(
        Read::read_vectored(
            &mut reader,
            &mut [IoSliceMut::new(&mut first), IoSliceMut::new(&mut second)]
        )
        .unwrap(),
        4
    );
    assert_eq!((&first, &second), (b"ab", b"cd"));
}

struct EdgeInput {
    reply: std::result::Result<usize, ErrorKind>,
    calls: usize,
}

impl Input for EdgeInput {
    type Item = u8;

    unsafe fn read_unchecked(&mut self, _: &mut [u8], _: usize, _: usize) -> Result<usize> {
        panic!("scalar read called")
    }

    fn read_vectored(&mut self, buffers: &mut [&mut [u8]]) -> Result<usize> {
        self.calls += 1;
        let count = self.reply.map_err(Error::from)?;
        let mut remaining = count;
        for buffer in buffers {
            for slot in buffer.iter_mut() {
                if remaining == 0 {
                    return Ok(count);
                }
                *slot = b'x';
                remaining -= 1;
            }
        }
        Ok(count)
    }
}

#[test]
fn test_vectored_std_adapter_preserves_empty_short_error_and_count_contracts() {
    let mut reader = StdRead::new(EdgeInput { reply: Ok(0), calls: 0 });
    assert_eq!(Read::read_vectored(&mut reader, &mut []).unwrap(), 0);
    assert_eq!(
        Read::read_vectored(&mut reader, &mut [IoSliceMut::new(&mut [])]).unwrap(),
        0
    );
    assert_eq!(reader.get_ref().calls, 0);

    let mut reader = StdRead::new(EdgeInput { reply: Ok(1), calls: 0 });
    let mut first = [b'!'; 2];
    let mut second = [b'?'; 2];
    assert_eq!(
        Read::read_vectored(
            &mut reader,
            &mut [IoSliceMut::new(&mut first), IoSliceMut::new(&mut second)]
        )
        .unwrap(),
        1
    );
    assert_eq!(first, [b'x', b'!']);
    assert_eq!(second, [b'?', b'?']);
    assert_eq!(reader.get_ref().calls, 1);

    for kind in [ErrorKind::Interrupted, ErrorKind::PermissionDenied] {
        let mut reader = StdRead::new(EdgeInput {
            reply: Err(kind),
            calls: 0,
        });
        assert_eq!(
            Read::read_vectored(&mut reader, &mut [IoSliceMut::new(&mut [0])])
                .unwrap_err()
                .kind(),
            kind
        );
        assert_eq!(reader.get_ref().calls, 1);
    }

    let mut reader = StdRead::new(EdgeInput { reply: Ok(5), calls: 0 });
    let mut first = [0; 2];
    let mut second = [0; 2];
    assert_eq!(
        Read::read_vectored(
            &mut reader,
            &mut [IoSliceMut::new(&mut first), IoSliceMut::new(&mut second)]
        )
        .unwrap_err()
        .kind(),
        ErrorKind::InvalidData
    );
    assert_eq!(reader.get_ref().calls, 1);
}

#[test]
fn test_vectored_std_read_adapter_nested_bridge_does_not_recurse() {
    let mut reader = StdRead::new(StdRead::new(VectorInput));
    let mut first = [0; 2];
    let mut second = [0; 2];
    assert_eq!(
        Read::read_vectored(
            &mut reader,
            &mut [IoSliceMut::new(&mut first), IoSliceMut::new(&mut second)]
        )
        .unwrap(),
        4
    );
    assert_eq!((&first, &second), (b"ab", b"cd"));
}

#[test]
fn test_std_read_delegates_to_input() {
    let mut reader = Cursor::new(b"abcd").into_std_read();
    let mut bytes = [0; 4];
    reader.read_exact(&mut bytes).expect("reader should fill output");
    assert_eq!(&bytes, b"abcd");
    assert_eq!(reader.into_inner().position(), 4);
}

#[test]
fn test_std_read_reports_tee_primary_progress_before_branch_failure() {
    let tee = TeeInput::new(Cursor::new(b"abcd"), LimitOutput::new(Vec::<u8>::new(), 2));
    let mut reader = StdRead::new(tee);
    let mut bytes = [0; 3];
    assert_eq!(Read::read(&mut reader, &mut bytes).unwrap(), 3);
    assert_eq!(&bytes, b"abc");
    let mut next = [9];
    assert_eq!(
        Read::read(&mut reader, &mut next).unwrap_err().kind(),
        ErrorKind::WriteZero
    );
    assert_eq!(next, [9]);
    assert_eq!(reader.get_ref().inner().position(), 3);
}

#[test]
fn test_into_std_read_preserves_checksum_state_and_stream_position() {
    let input = ChecksumInput::new(Cursor::new(b"abcd"), DefaultHasher::new());
    let mut reader = input.into_std_read();
    let mut bytes = [0; 2];
    reader.read_exact(&mut bytes).expect("reader should fill output");
    assert_eq!(&bytes, b"ab");
    let input = reader.into_inner();
    assert_eq!(2, input.inner().position());
    assert_ne!(0, input.checksum());
}

#[test]
fn test_into_std_read_supports_borrowed_and_dyn_inputs() {
    let mut cursor = Cursor::new(b"abc");
    {
        let borrowed = InputRef::new(&mut cursor).into_std_read();
        let mut reader = borrowed;
        let mut bytes = [0; 1];
        reader.read_exact(&mut bytes).expect("read should succeed");
        assert_eq!(&bytes, b"a");
        assert_eq!(2, reader.seek(SeekFrom::Current(1)).expect("seek should succeed"));
    }
    assert_eq!(2, cursor.position());

    let mut cursor = Cursor::new(b"dyn");
    let input: &mut dyn Input<Item = u8> = &mut cursor;
    let mut reader = InputRef::new(input).into_std_read();
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes).expect("read should succeed");
    assert_eq!(bytes, b"dyn");
}

#[test]
fn test_std_read_seek_forwards_position_and_errors() {
    let mut reader = Cursor::new(b"abcd").into_std_read();
    assert_eq!(3, reader.seek(SeekFrom::Start(3)).expect("seek should succeed"));

    let mut reader = ErrorSeekInput.into_std_read();
    assert_eq!("seek failed", reader.seek(SeekFrom::Start(1)).unwrap_err().to_string());
}

#[test]
fn test_std_read_supports_read_only_inner() {
    fn assert_read<T: Read>(_: &T) {}
    let reader = StdRead::new(ReadOnlyInput);
    assert_read(&reader);
    let mut reader = StdRead::new(ReadOnlyInput);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        reader.read_exact(&mut [0]).unwrap_err().kind()
    );
}

#[test]
fn test_into_std_read_retries_interrupted_and_handles_short_reads() {
    let input = ShortInput {
        bytes: b"abc".to_vec(),
        interrupted: true,
    };
    let mut reader = input.into_std_read();
    let mut bytes = [0; 3];
    reader.read_exact(&mut bytes).expect("read_exact should retry and fill");
    assert_eq!(&bytes, b"abc");
}

struct ReadOnlyInput;

impl Input for ReadOnlyInput {
    type Item = u8;

    unsafe fn read_unchecked(&mut self, _output: &mut [u8], _index: usize, _count: usize) -> Result<usize> {
        Ok(0)
    }
}

struct ErrorSeekInput;

impl Input for ErrorSeekInput {
    type Item = u8;

    unsafe fn read_unchecked(&mut self, _output: &mut [u8], _index: usize, _count: usize) -> Result<usize> {
        Ok(0)
    }
}

impl Seekable for ErrorSeekInput {
    type Unit = u8;

    fn seek_to(&mut self, _position: SeekFrom) -> Result<u64> {
        Err(Error::other("seek failed"))
    }
}

struct ShortInput {
    bytes: Vec<u8>,
    interrupted: bool,
}

impl Input for ShortInput {
    type Item = u8;

    unsafe fn read_unchecked(&mut self, output: &mut [u8], index: usize, count: usize) -> Result<usize> {
        if self.interrupted {
            self.interrupted = false;
            return Err(ErrorKind::Interrupted.into());
        }
        let read = count.min(1).min(self.bytes.len());
        output[index..index + read].copy_from_slice(&self.bytes[..read]);
        self.bytes.drain(..read);
        Ok(read)
    }
}
