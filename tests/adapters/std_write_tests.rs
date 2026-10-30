// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Cursor;
use std::io::IoSlice;
use std::io::Seek;
use std::io::SeekFrom;
use std::io::Write;

use qubit_io::ChecksumOutput;
use qubit_io::LimitOutput;
use qubit_io::Output;
use qubit_io::OutputRef;
use qubit_io::TeeOutput;

#[derive(Default)]
struct VectorOutput(Vec<u8>);

impl Output for VectorOutput {
    type Item = u8;
    unsafe fn write_unchecked(&mut self, _: &[u8], _: usize, _: usize) -> std::io::Result<usize> {
        panic!("scalar write called")
    }
    fn write_vectored(&mut self, buffers: &[&[u8]]) -> std::io::Result<usize> {
        let before = self.0.len();
        for buffer in buffers {
            self.0.extend_from_slice(buffer);
        }
        Ok(self.0.len() - before)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn vectored_std_adapter_keeps_inner_override() {
    let mut writer = qubit_io::StdWrite::new(VectorOutput::default());
    assert_eq!(
        Write::write_vectored(&mut writer, &[IoSlice::new(b"ab"), IoSlice::new(b"cd")]).unwrap(),
        4
    );
    assert_eq!(writer.into_inner().0, b"abcd");
}

struct EdgeOutput {
    reply: Result<usize, std::io::ErrorKind>,
    bytes: Vec<u8>,
    calls: usize,
}

impl Output for EdgeOutput {
    type Item = u8;

    unsafe fn write_unchecked(&mut self, _: &[u8], _: usize, _: usize) -> std::io::Result<usize> {
        panic!("scalar write called")
    }

    fn write_vectored(&mut self, buffers: &[&[u8]]) -> std::io::Result<usize> {
        self.calls += 1;
        let count = self.reply.map_err(std::io::Error::from)?;
        let mut remaining = count;
        for buffer in buffers {
            let copied = remaining.min(buffer.len());
            self.bytes.extend_from_slice(&buffer[..copied]);
            remaining -= copied;
            if remaining == 0 {
                break;
            }
        }
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn vectored_std_write_adapter_preserves_empty_short_error_and_count_contracts() {
    let mut writer = qubit_io::StdWrite::new(EdgeOutput {
        reply: Ok(0),
        bytes: Vec::new(),
        calls: 0,
    });
    assert_eq!(Write::write_vectored(&mut writer, &[]).unwrap(), 0);
    assert_eq!(
        Write::write_vectored(&mut writer, &[IoSlice::new(b""), IoSlice::new(b"")]).unwrap(),
        0
    );
    assert_eq!(writer.get_ref().calls, 0);

    let mut writer = qubit_io::StdWrite::new(EdgeOutput {
        reply: Ok(1),
        bytes: Vec::new(),
        calls: 0,
    });
    assert_eq!(
        Write::write_vectored(&mut writer, &[IoSlice::new(b"ab"), IoSlice::new(b"cd")]).unwrap(),
        1
    );
    assert_eq!(writer.get_ref().bytes, b"a");
    assert_eq!(writer.get_ref().calls, 1);

    for kind in [std::io::ErrorKind::Interrupted, std::io::ErrorKind::PermissionDenied] {
        let mut writer = qubit_io::StdWrite::new(EdgeOutput {
            reply: Err(kind),
            bytes: Vec::new(),
            calls: 0,
        });
        assert_eq!(
            Write::write_vectored(&mut writer, &[IoSlice::new(b"a")])
                .unwrap_err()
                .kind(),
            kind
        );
        assert_eq!(writer.get_ref().calls, 1);
    }

    let mut writer = qubit_io::StdWrite::new(EdgeOutput {
        reply: Ok(5),
        bytes: Vec::new(),
        calls: 0,
    });
    assert_eq!(
        Write::write_vectored(&mut writer, &[IoSlice::new(b"ab"), IoSlice::new(b"cd")])
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::InvalidData
    );
    assert_eq!(writer.get_ref().calls, 1);
}

#[test]
fn vectored_std_write_adapter_nested_bridge_does_not_recurse() {
    let mut writer = qubit_io::StdWrite::new(qubit_io::StdWrite::new(VectorOutput::default()));
    assert_eq!(
        Write::write_vectored(&mut writer, &[IoSlice::new(b"ab"), IoSlice::new(b"cd")]).unwrap(),
        4
    );
    assert_eq!(writer.into_inner().into_inner().0, b"abcd");
}

#[test]
fn test_std_write_delegates_to_output() {
    let mut writer = Cursor::new(Vec::<u8>::new()).into_std_write();
    writer.write_all(b"abcd").expect("writer should accept input");
    Write::flush(&mut writer).expect("writer should flush");
    assert_eq!(writer.get_ref().get_ref(), b"abcd");
    assert_eq!(writer.into_inner().into_inner(), b"abcd");
}

#[test]
fn test_std_write_reports_tee_primary_progress_before_branch_failure() {
    let tee = TeeOutput::new(Vec::<u8>::new(), LimitOutput::new(Vec::<u8>::new(), 2));
    let mut writer = qubit_io::StdWrite::new(tee);
    assert_eq!(std::io::Write::write(&mut writer, b"abc").unwrap(), 3);
    assert_eq!(writer.get_ref().inner(), b"abc");
    assert_eq!(
        std::io::Write::write(&mut writer, b"d").unwrap_err().kind(),
        std::io::ErrorKind::WriteZero
    );
    assert_eq!(writer.get_ref().inner(), b"abc");
}

#[test]
fn test_into_std_write_preserves_checksum_state_and_output() {
    let output = ChecksumOutput::new(
        Cursor::new(Vec::new()),
        std::collections::hash_map::DefaultHasher::new(),
    );
    let mut writer = output.into_std_write();
    writer.write_all(b"abcd").expect("write should succeed");
    let output = writer.into_inner();
    assert_eq!(b"abcd", output.inner().get_ref().as_slice());
    assert_ne!(0, output.checksum());
}

#[test]
fn test_into_std_write_supports_borrowed_and_dyn_outputs() {
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = OutputRef::new(&mut cursor).into_std_write();
        writer.write_all(b"abc").expect("write should succeed");
        assert_eq!(1, writer.seek(SeekFrom::Start(1)).expect("seek should succeed"));
    }
    assert_eq!(b"abc", cursor.get_ref().as_slice());

    let mut cursor = Cursor::new(Vec::new());
    let output: &mut dyn Output<Item = u8> = &mut cursor;
    let mut writer = OutputRef::new(output).into_std_write();
    writer.write_all(b"dyn").expect("write should succeed");
    drop(writer);
    assert_eq!(b"dyn", cursor.get_ref().as_slice());
}

#[test]
fn test_std_write_seek_forwards_position_and_errors() {
    let mut writer = Cursor::new(Vec::<u8>::new()).into_std_write();
    assert_eq!(3, writer.seek(SeekFrom::Start(3)).expect("seek should succeed"));

    let mut writer = ErrorSeekOutput.into_std_write();
    assert_eq!("seek failed", writer.seek(SeekFrom::Start(1)).unwrap_err().to_string());
}

#[test]
fn test_std_write_supports_write_only_inner() {
    fn assert_write<T: Write>(_: &T) {}
    let writer = qubit_io::StdWrite::new(WriteOnlyOutput);
    assert_write(&writer);
}

#[test]
fn test_into_std_write_handles_short_writes_zero_progress_and_flush_errors() {
    let mut writer = ShortOutput { bytes: Vec::new() }.into_std_write();
    writer.write_all(b"abc").expect("write_all should retry short writes");
    assert_eq!(b"abc", writer.into_inner().bytes.as_slice());

    let mut writer = ZeroOutput.into_std_write();
    assert_eq!(
        std::io::ErrorKind::WriteZero,
        writer.write_all(b"x").unwrap_err().kind()
    );

    let mut writer = FlushErrorOutput.into_std_write();
    assert_eq!("flush failed", Write::flush(&mut writer).unwrap_err().to_string());
}

struct WriteOnlyOutput;

impl Output for WriteOnlyOutput {
    type Item = u8;

    unsafe fn write_unchecked(&mut self, _input: &[u8], _index: usize, count: usize) -> std::io::Result<usize> {
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct ErrorSeekOutput;

impl Output for ErrorSeekOutput {
    type Item = u8;

    unsafe fn write_unchecked(&mut self, _input: &[u8], _index: usize, count: usize) -> std::io::Result<usize> {
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl qubit_io::Seekable for ErrorSeekOutput {
    type Unit = u8;

    fn seek_to(&mut self, _position: SeekFrom) -> std::io::Result<u64> {
        Err(std::io::Error::other("seek failed"))
    }
}

struct ShortOutput {
    bytes: Vec<u8>,
}

impl Output for ShortOutput {
    type Item = u8;

    unsafe fn write_unchecked(&mut self, input: &[u8], index: usize, count: usize) -> std::io::Result<usize> {
        let written = count.min(1);
        self.bytes.extend_from_slice(&input[index..index + written]);
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct ZeroOutput;

impl Output for ZeroOutput {
    type Item = u8;

    unsafe fn write_unchecked(&mut self, _input: &[u8], _index: usize, _count: usize) -> std::io::Result<usize> {
        Ok(0)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct FlushErrorOutput;

impl Output for FlushErrorOutput {
    type Item = u8;

    unsafe fn write_unchecked(&mut self, _input: &[u8], _index: usize, count: usize) -> std::io::Result<usize> {
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::other("flush failed"))
    }
}
