use std::io::{
    Error,
    ErrorKind,
    Write,
};

use qubit_io::StringWriteExt;

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
fn test_string_write_ext_writes_all_length_prefix_kinds() {
    let mut output = Vec::new();

    output
        .write_utf8_string_uleb("hi")
        .expect("ULEB string should be written");
    output
        .write_utf8_string_u16_be("be")
        .expect("u16 BE string should be written");
    output
        .write_utf8_string_u16_le("le")
        .expect("u16 LE string should be written");
    output
        .write_utf8_string_u32_be("up")
        .expect("u32 BE string should be written");
    output
        .write_utf8_string_u32_le("dn")
        .expect("u32 LE string should be written");

    assert_eq!(
        vec![
            0x02, b'h', b'i', 0x00, 0x02, b'b', b'e', 0x02, 0x00, b'l', b'e', 0x00, 0x00, 0x00,
            0x02, b'u', b'p', 0x02, 0x00, 0x00, 0x00, b'd', b'n'
        ],
        output
    );
}

#[test]
fn test_string_write_ext_reports_length_and_writer_errors() {
    let mut output = Vec::new();
    let value = "x".repeat(usize::from(u16::MAX) + 1);
    assert_eq!(
        ErrorKind::InvalidInput,
        output
            .write_utf8_string_u16_be(&value)
            .expect_err("oversized u16 BE string should fail")
            .kind()
    );
    assert_eq!(
        ErrorKind::InvalidInput,
        output
            .write_utf8_string_u16_le(&value)
            .expect_err("oversized u16 LE string should fail")
            .kind()
    );

    let mut writer = FailingWriter;
    assert_eq!(
        ErrorKind::Other,
        writer
            .write_utf8_string_uleb("hi")
            .expect_err("writer error should be returned")
            .kind()
    );
}
