use std::io::{
    Cursor,
    ErrorKind,
};

use qubit_io::{
    BinaryReadExt,
    ByteOrder,
};

macro_rules! assert_read_ordered_integer {
    ($method:ident, $be:ident, $le:ident, $ty:ty, $value:expr) => {{
        let value: $ty = $value;
        let mut input = Cursor::new(value.to_be_bytes());
        assert_eq!(
            value,
            input
                .$method(ByteOrder::BigEndian)
                .expect("runtime BE read")
        );
        let mut input = Cursor::new(value.to_le_bytes());
        assert_eq!(
            value,
            input
                .$method(ByteOrder::LittleEndian)
                .expect("runtime LE read")
        );
        let mut input = Cursor::new(value.to_be_bytes());
        assert_eq!(value, input.$be().expect("fixed BE read"));
        let mut input = Cursor::new(value.to_le_bytes());
        assert_eq!(value, input.$le().expect("fixed LE read"));
    }};
}

macro_rules! assert_read_ordered_float {
    ($method:ident, $be:ident, $le:ident, $ty:ty, $value:expr) => {{
        let value: $ty = $value;
        let mut input = Cursor::new(value.to_bits().to_be_bytes());
        assert_eq!(
            value,
            input
                .$method(ByteOrder::BigEndian)
                .expect("runtime BE read")
        );
        let mut input = Cursor::new(value.to_bits().to_le_bytes());
        assert_eq!(
            value,
            input
                .$method(ByteOrder::LittleEndian)
                .expect("runtime LE read")
        );
        let mut input = Cursor::new(value.to_bits().to_be_bytes());
        assert_eq!(value, input.$be().expect("fixed BE read"));
        let mut input = Cursor::new(value.to_bits().to_le_bytes());
        assert_eq!(value, input.$le().expect("fixed LE read"));
    }};
}

#[test]
fn test_binary_read_ext_reads_all_scalar_methods() {
    let mut input = Cursor::new([0x12, 0xfe]);
    assert_eq!(
        [0x12, 0xfe],
        input.read_bytes::<2>().expect("bytes should be read")
    );
    let mut input = Cursor::new([0x12]);
    assert_eq!(0x12, input.read_u8().expect("u8 should be read"));
    let mut input = Cursor::new([0xfe]);
    assert_eq!(-2, input.read_i8().expect("i8 should be read"));

    assert_read_ordered_integer!(read_u16, read_u16_be, read_u16_le, u16, 0x1234);
    assert_read_ordered_integer!(read_u32, read_u32_be, read_u32_le, u32, 0x1234_5678);
    assert_read_ordered_integer!(
        read_u64,
        read_u64_be,
        read_u64_le,
        u64,
        0x0123_4567_89ab_cdef
    );
    assert_read_ordered_integer!(
        read_u128,
        read_u128_be,
        read_u128_le,
        u128,
        0x0123_4567_89ab_cdef_fedc_ba98_7654_3210
    );
    assert_read_ordered_integer!(read_i16, read_i16_be, read_i16_le, i16, -0x1234);
    assert_read_ordered_integer!(read_i32, read_i32_be, read_i32_le, i32, -0x0123_4567);
    assert_read_ordered_integer!(
        read_i64,
        read_i64_be,
        read_i64_le,
        i64,
        -0x0123_4567_89ab_cdef
    );
    assert_read_ordered_integer!(
        read_i128,
        read_i128_be,
        read_i128_le,
        i128,
        -0x0123_4567_89ab_cdef_fedc_ba98_7654_3210
    );
    assert_read_ordered_float!(read_f32, read_f32_be, read_f32_le, f32, 12.5);
    assert_read_ordered_float!(read_f64, read_f64_be, read_f64_le, f64, -25.25);
}

#[test]
fn test_binary_read_ext_reports_errors() {
    let mut input = Cursor::new([0x12, 0x34, 0x56]);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        input
            .read_u32_be()
            .expect_err("truncated u32 should fail")
            .kind()
    );
}
