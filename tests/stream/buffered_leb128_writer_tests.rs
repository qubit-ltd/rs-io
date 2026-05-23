use std::io::{Error, ErrorKind, Write};

use qubit_io::{BufferedLeb128Writer, Leb128WriteExt};

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
fn test_buffered_leb128_writer_writes_values_across_buffer_boundaries() {
    let mut expected = Vec::new();
    expected
        .write_uleb_u8(u8::MAX)
        .expect("u8 should be encoded");
    expected.write_uleb_u16(300).expect("u16 should be encoded");
    expected
        .write_uleb_u32(0x1f600)
        .expect("u32 should be encoded");
    expected
        .write_uleb_u64(0x0102_0304_0506_0708)
        .expect("u64 should be encoded");
    expected
        .write_sleb_i8(i8::MIN)
        .expect("i8 should be encoded");
    expected
        .write_sleb_i16(-300)
        .expect("i16 should be encoded");
    expected
        .write_sleb_i32(-0x1f600)
        .expect("i32 should be encoded");
    expected
        .write_sleb_i64(-0x0102_0304_0506_0708)
        .expect("i64 should be encoded");

    let mut writer = BufferedLeb128Writer::with_capacity(Vec::new(), 3);
    writer.write_u8(u8::MAX).expect("u8 should be written");
    writer.write_u16(300).expect("u16 should be written");
    writer.write_u32(0x1f600).expect("u32 should be written");
    writer
        .write_u64(0x0102_0304_0506_0708)
        .expect("u64 should be written");
    writer.write_i8(i8::MIN).expect("i8 should be written");
    writer.write_i16(-300).expect("i16 should be written");
    writer.write_i32(-0x1f600).expect("i32 should be written");
    writer
        .write_i64(-0x0102_0304_0506_0708)
        .expect("i64 should be written");

    assert_eq!(expected, writer.into_inner().expect("writer should flush"));
}

#[test]
fn test_buffered_leb128_writer_returns_writer_error() {
    let mut writer = BufferedLeb128Writer::with_capacity(FailingWriter, 8);

    writer.write_u64(300).expect("value should be buffered");
    let error = writer.flush().expect_err("flush should fail");

    assert_eq!(ErrorKind::Other, error.kind());
}
