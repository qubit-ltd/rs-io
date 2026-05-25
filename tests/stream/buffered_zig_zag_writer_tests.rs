use std::io::{
    Cursor,
    Error,
    ErrorKind,
    Seek,
    Write,
};

use qubit_io::{
    BufferedZigZagWriter,
    ZigZagWriteExt,
};

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
fn test_buffered_zig_zag_writer_writes_values_across_buffer_boundaries() {
    let mut expected = Vec::new();
    expected.write_zig_zag_i8(i8::MIN).expect("i8 should be encoded");
    expected.write_zig_zag_i16(-300).expect("i16 should be encoded");
    expected.write_zig_zag_i32(-0x1f600).expect("i32 should be encoded");
    expected
        .write_zig_zag_i64(-0x0102_0304_0506_0708)
        .expect("i64 should be encoded");
    expected
        .write_zig_zag_i128(-0x0102_0304_0506_0708_1112_1314_1516_1718)
        .expect("i128 should be encoded");
    expected
        .write_zig_zag_isize(isize::MIN)
        .expect("isize should be encoded");

    let mut writer = BufferedZigZagWriter::with_capacity(Vec::new(), 3);
    writer.write_i8(i8::MIN).expect("i8 should be written");
    writer.write_i16(-300).expect("i16 should be written");
    writer.write_i32(-0x1f600).expect("i32 should be written");
    writer.write_i64(-0x0102_0304_0506_0708).expect("i64 should be written");
    writer
        .write_i128(-0x0102_0304_0506_0708_1112_1314_1516_1718)
        .expect("i128 should be written");
    writer.write_isize(isize::MIN).expect("isize should be written");

    assert_eq!(expected, writer.into_inner().expect("writer should flush"));
}

#[test]
fn test_buffered_zig_zag_writer_accessors_write_all_seek_and_into_inner() {
    let mut writer = BufferedZigZagWriter::new(Cursor::new(Vec::new()));

    assert_eq!(0, writer.inner().position());
    writer.inner_mut().set_position(0);
    writer.write_i8(-1).expect("ZigZag value should be buffered");
    assert_eq!(1, writer.write(&[9]).expect("raw byte should be buffered"));
    writer.write_all(&[10]).expect("raw byte should be buffered");
    assert_eq!(3, writer.stream_position().expect("seek should flush pending bytes"));

    let inner = writer.into_inner().expect("into_inner should flush");

    assert_eq!(vec![1, 9, 10], inner.into_inner());
}

#[test]
fn test_buffered_zig_zag_writer_returns_writer_error() {
    let mut writer = BufferedZigZagWriter::with_capacity(FailingWriter, 8);

    writer.write_i64(-300).expect("value should be buffered");
    let error = writer.flush().expect_err("flush should fail");

    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_buffered_zig_zag_writer_flushes_before_encoded_value_when_full() {
    let mut writer = BufferedZigZagWriter::with_capacity(Vec::new(), 19);

    writer.write_all(&[1; 18]).expect("initial bytes should be buffered");
    writer.write_i8(-1).expect("encoded value should flush then buffer");

    let mut expected = vec![1; 18];
    expected.push(1);
    assert_eq!(expected, writer.into_inner().expect("writer should flush"));
}
