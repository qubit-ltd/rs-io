use std::io::{
    Error,
    ErrorKind,
    Write,
};

use qubit_io::ZigZagWriter;

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
        Err(Error::other("write failed"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_zig_zag_writer_writes_all_methods_and_exposes_accessors() {
    let mut writer = ZigZagWriter::new(Vec::new());

    assert_eq!(0, writer.get_ref().len());
    writer.get_mut().extend_from_slice(&[]);
    writer.write_i8(i8::MIN).expect("i8 should be written");
    writer.write_i16(-300).expect("i16 should be written");
    writer.write_i32(-0x1f600).expect("i32 should be written");
    writer.write_i64(i64::MIN).expect("i64 should be written");
    writer
        .write_i128(i128::MIN)
        .expect("i128 should be written");
    writer
        .write_isize(isize::MIN)
        .expect("isize should be written");

    assert!(!writer.into_inner().is_empty());
}

#[test]
fn test_zig_zag_writer_returns_writer_error() {
    let mut writer = ZigZagWriter::new(FailingWriter);

    let error = writer
        .write_i16(-300)
        .expect_err("writer error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_zig_zag_writer_write_and_seek_delegate_to_inner_writer() {
    let mut writer = qubit_io::ZigZagWriter::new(std::io::Cursor::new(vec![0; 4]));

    std::io::Seek::seek(&mut writer, std::io::SeekFrom::Start(1))
        .expect("seeking through ZigZagWriter should succeed");
    std::io::Write::write_all(&mut writer, b"xy")
        .expect("writing through ZigZagWriter should succeed");
    std::io::Write::flush(&mut writer).expect("flushing through ZigZagWriter should succeed");

    let cursor = writer.into_inner();
    assert_eq!(cursor.into_inner(), vec![0, b'x', b'y', 0]);
}
