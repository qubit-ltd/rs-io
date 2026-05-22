use std::io::{
    Error,
    ErrorKind,
    Write,
};

use qubit_io::{
    BinaryWriteExt,
    ByteOrder,
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

macro_rules! assert_write_ordered_integer {
    ($method:ident, $be:ident, $le:ident, $ty:ty, $value:expr) => {{
        let value: $ty = $value;
        let mut output = Vec::new();
        output
            .$method(value, ByteOrder::BigEndian)
            .expect("runtime BE write");
        assert_eq!(value.to_be_bytes().as_slice(), output.as_slice());
        let mut output = Vec::new();
        output
            .$method(value, ByteOrder::LittleEndian)
            .expect("runtime LE write");
        assert_eq!(value.to_le_bytes().as_slice(), output.as_slice());
        let mut output = Vec::new();
        output.$be(value).expect("fixed BE write");
        assert_eq!(value.to_be_bytes().as_slice(), output.as_slice());
        let mut output = Vec::new();
        output.$le(value).expect("fixed LE write");
        assert_eq!(value.to_le_bytes().as_slice(), output.as_slice());
    }};
}

macro_rules! assert_write_ordered_float {
    ($method:ident, $be:ident, $le:ident, $ty:ty, $value:expr) => {{
        let value: $ty = $value;
        let mut output = Vec::new();
        output
            .$method(value, ByteOrder::BigEndian)
            .expect("runtime BE write");
        assert_eq!(value.to_bits().to_be_bytes().as_slice(), output.as_slice());
        let mut output = Vec::new();
        output
            .$method(value, ByteOrder::LittleEndian)
            .expect("runtime LE write");
        assert_eq!(value.to_bits().to_le_bytes().as_slice(), output.as_slice());
        let mut output = Vec::new();
        output.$be(value).expect("fixed BE write");
        assert_eq!(value.to_bits().to_be_bytes().as_slice(), output.as_slice());
        let mut output = Vec::new();
        output.$le(value).expect("fixed LE write");
        assert_eq!(value.to_bits().to_le_bytes().as_slice(), output.as_slice());
    }};
}

#[test]
fn test_binary_write_ext_writes_all_scalar_methods() {
    let mut output = Vec::new();
    output
        .write_bytes([0x12, 0xfe])
        .expect("bytes should be written");
    output.write_u8(0x12).expect("u8 should be written");
    output.write_i8(-2).expect("i8 should be written");
    assert_eq!(vec![0x12, 0xfe, 0x12, 0xfe], output);

    assert_write_ordered_integer!(write_u16, write_u16_be, write_u16_le, u16, 0x1234);
    assert_write_ordered_integer!(write_u32, write_u32_be, write_u32_le, u32, 0x1234_5678);
    assert_write_ordered_integer!(
        write_u64,
        write_u64_be,
        write_u64_le,
        u64,
        0x0123_4567_89ab_cdef
    );
    assert_write_ordered_integer!(
        write_u128,
        write_u128_be,
        write_u128_le,
        u128,
        0x0123_4567_89ab_cdef_fedc_ba98_7654_3210
    );
    assert_write_ordered_integer!(write_i16, write_i16_be, write_i16_le, i16, -0x1234);
    assert_write_ordered_integer!(write_i32, write_i32_be, write_i32_le, i32, -0x0123_4567);
    assert_write_ordered_integer!(
        write_i64,
        write_i64_be,
        write_i64_le,
        i64,
        -0x0123_4567_89ab_cdef
    );
    assert_write_ordered_integer!(
        write_i128,
        write_i128_be,
        write_i128_le,
        i128,
        -0x0123_4567_89ab_cdef_fedc_ba98_7654_3210
    );
    assert_write_ordered_float!(write_f32, write_f32_be, write_f32_le, f32, 12.5);
    assert_write_ordered_float!(write_f64, write_f64_be, write_f64_le, f64, -25.25);
}

#[test]
fn test_binary_write_ext_writes_length_prefixed_strings() {
    let mut output = Vec::new();
    output
        .write_utf8_string_u16("hi", ByteOrder::BigEndian)
        .expect("u16 BE string should be written");
    output
        .write_utf8_string_u16("ok", ByteOrder::LittleEndian)
        .expect("u16 LE string should be written");
    output
        .write_utf8_string_u32("up", ByteOrder::BigEndian)
        .expect("u32 BE string should be written");
    output
        .write_utf8_string_u32("dn", ByteOrder::LittleEndian)
        .expect("u32 LE string should be written");

    assert_eq!(
        vec![
            0x00, 0x02, b'h', b'i', 0x02, 0x00, b'o', b'k', 0x00, 0x00, 0x00, 0x02, b'u', b'p',
            0x02, 0x00, 0x00, 0x00, b'd', b'n'
        ],
        output
    );
}

#[test]
fn test_binary_write_ext_reports_errors() {
    let mut output = Vec::new();
    let value = "x".repeat(usize::from(u16::MAX) + 1);
    assert_eq!(
        ErrorKind::InvalidInput,
        output
            .write_utf8_string_u16(&value, ByteOrder::BigEndian)
            .expect_err("oversized u16 string should fail")
            .kind()
    );

    let mut writer = FailingWriter;
    assert_eq!(
        ErrorKind::Other,
        writer
            .write_u16_be(0x1234)
            .expect_err("write should fail")
            .kind()
    );
}
