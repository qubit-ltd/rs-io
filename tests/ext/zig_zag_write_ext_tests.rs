use std::io::{
    Error,
    ErrorKind,
    Write,
};

use qubit_io::ZigZagWriteExt;

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
fn test_zig_zag_write_ext_writes_all_methods() {
    let mut output = Vec::new();

    output
        .write_zig_zag_i8(i8::MIN)
        .expect("i8 should be written");
    output
        .write_zig_zag_i16(-300)
        .expect("i16 should be written");
    output
        .write_zig_zag_i32(-0x1f600)
        .expect("i32 should be written");
    output
        .write_zig_zag_i64(i64::MIN)
        .expect("i64 should be written");
    output
        .write_zig_zag_i128(i128::MIN)
        .expect("i128 should be written");
    output
        .write_zig_zag_isize(isize::MIN)
        .expect("isize should be written");

    assert!(!output.is_empty());
}

#[test]
fn test_zig_zag_write_ext_returns_writer_error() {
    let mut writer = FailingWriter;

    let error = writer
        .write_zig_zag_i16(-300)
        .expect_err("writer error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
}
