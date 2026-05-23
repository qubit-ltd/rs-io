use std::io::{Cursor, ErrorKind, Read};

use qubit_io::{BinaryWriteExt, BufferedBinaryReader, ByteOrder, LittleEndian};

fn encoded_values() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.write_u8(0xaa).expect("u8 should be encoded");
    bytes.write_i8(-2).expect("i8 should be encoded");
    bytes.write_u16_le(0x1234).expect("u16 should be encoded");
    bytes
        .write_u32_le(0x1234_5678)
        .expect("u32 should be encoded");
    bytes
        .write_u64_le(0x0123_4567_89ab_cdef)
        .expect("u64 should be encoded");
    bytes
        .write_u128_le(0x0123_4567_89ab_cdef_fedc_ba98_7654_3210)
        .expect("u128 should be encoded");
    bytes.write_i16_le(-0x1234).expect("i16 should be encoded");
    bytes
        .write_i32_le(-0x0123_4567)
        .expect("i32 should be encoded");
    bytes
        .write_i64_le(-0x0123_4567_89ab_cdef)
        .expect("i64 should be encoded");
    bytes
        .write_i128_le(-0x0123_4567_89ab_cdef_fedc_ba98_7654_3210)
        .expect("i128 should be encoded");
    bytes.write_f32_le(12.5).expect("f32 should be encoded");
    bytes.write_f64_le(-25.25).expect("f64 should be encoded");
    bytes
}

#[test]
fn test_buffered_binary_reader_reads_scalars_across_buffer_boundaries() {
    let mut reader =
        BufferedBinaryReader::<_, LittleEndian>::with_capacity(Cursor::new(encoded_values()), 9);

    assert_eq!(ByteOrder::LittleEndian, reader.byte_order());
    assert_eq!(0xaa, reader.read_u8().expect("u8 should be read"));
    assert_eq!(-2, reader.read_i8().expect("i8 should be read"));
    assert_eq!(0x1234, reader.read_u16().expect("u16 should be read"));
    assert_eq!(0x1234_5678, reader.read_u32().expect("u32 should be read"));
    assert_eq!(
        0x0123_4567_89ab_cdef,
        reader.read_u64().expect("u64 should be read")
    );
    assert_eq!(
        0x0123_4567_89ab_cdef_fedc_ba98_7654_3210,
        reader.read_u128().expect("u128 should be read")
    );
    assert_eq!(-0x1234, reader.read_i16().expect("i16 should be read"));
    assert_eq!(-0x0123_4567, reader.read_i32().expect("i32 should be read"));
    assert_eq!(
        -0x0123_4567_89ab_cdef,
        reader.read_i64().expect("i64 should be read")
    );
    assert_eq!(
        -0x0123_4567_89ab_cdef_fedc_ba98_7654_3210,
        reader.read_i128().expect("i128 should be read")
    );
    assert_eq!(12.5, reader.read_f32().expect("f32 should be read"));
    assert_eq!(-25.25, reader.read_f64().expect("f64 should be read"));
}

#[test]
fn test_buffered_binary_reader_reports_unexpected_eof() {
    let mut reader =
        BufferedBinaryReader::<_, LittleEndian>::with_capacity(Cursor::new(vec![0x34]), 8);

    let error = reader.read_u16().expect_err("truncated u16 should fail");

    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
}

#[test]
fn test_buffered_binary_reader_implements_read() {
    let mut reader =
        BufferedBinaryReader::<_, LittleEndian>::with_capacity(Cursor::new(vec![1, 2, 3, 4]), 2);
    let mut bytes = [0u8; 3];

    reader
        .read_exact(&mut bytes)
        .expect("raw bytes should be read");

    assert_eq!([1, 2, 3], bytes);
    assert_eq!(4, reader.read_u8().expect("remaining byte should be read"));
}
