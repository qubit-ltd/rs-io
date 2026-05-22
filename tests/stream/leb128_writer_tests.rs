use std::io::{
    Error,
    ErrorKind,
    Write,
};

use qubit_io::Leb128Writer;

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
fn test_leb128_writer_writes_all_methods_and_exposes_accessors() {
    let mut writer = Leb128Writer::new(Vec::new());

    assert_eq!(0, writer.get_ref().len());
    writer.get_mut().extend_from_slice(&[]);
    writer.write_u8(u8::MAX).expect("u8 should be written");
    writer.write_u16(300).expect("u16 should be written");
    writer.write_u32(0x1f600).expect("u32 should be written");
    writer
        .write_u64(0x0102_0304_0506_0708)
        .expect("u64 should be written");
    writer
        .write_u128(0x0102_0304_0506_0708_1112_1314_1516_1718)
        .expect("u128 should be written");
    writer
        .write_usize(usize::MAX)
        .expect("usize should be written");
    writer.write_i8(i8::MIN).expect("i8 should be written");
    writer.write_i16(-300).expect("i16 should be written");
    writer.write_i32(-0x1f600).expect("i32 should be written");
    writer
        .write_i64(-0x0102_0304_0506_0708)
        .expect("i64 should be written");
    writer
        .write_i128(-0x0102_0304_0506_0708_1112_1314_1516_1718)
        .expect("i128 should be written");
    writer
        .write_isize(isize::MIN)
        .expect("isize should be written");

    assert!(!writer.into_inner().is_empty());
}

#[test]
fn test_leb128_writer_returns_writer_error() {
    let mut writer = Leb128Writer::new(FailingWriter);

    let error = writer
        .write_u16(300)
        .expect_err("writer error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
}
