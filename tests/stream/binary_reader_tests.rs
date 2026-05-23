use std::io::{Cursor, ErrorKind};

use qubit_io::{BigEndian, BinaryReader, ByteOrder, LittleEndian};

fn push_be_values(output: &mut Vec<u8>) {
    output.extend_from_slice(&[0xaa, 0xbb]);
    output.push(0x12);
    output.push((-2_i8) as u8);
    output.extend_from_slice(&0x1234_u16.to_be_bytes());
    output.extend_from_slice(&0x1234_5678_u32.to_be_bytes());
    output.extend_from_slice(&0x0123_4567_89ab_cdef_u64.to_be_bytes());
    output.extend_from_slice(&0x0123_4567_89ab_cdef_fedc_ba98_7654_3210_u128.to_be_bytes());
    output.extend_from_slice(&(-0x1234_i16).to_be_bytes());
    output.extend_from_slice(&(-0x0123_4567_i32).to_be_bytes());
    output.extend_from_slice(&(-0x0123_4567_89ab_cdef_i64).to_be_bytes());
    output.extend_from_slice(&(-0x0123_4567_89ab_cdef_fedc_ba98_7654_3210_i128).to_be_bytes());
    output.extend_from_slice(&12.5_f32.to_bits().to_be_bytes());
    output.extend_from_slice(&(-25.25_f64).to_bits().to_be_bytes());
    output.extend_from_slice(&2_u16.to_be_bytes());
    output.extend_from_slice(b"hi");
    output.extend_from_slice(&2_u32.to_be_bytes());
    output.extend_from_slice(b"ok");
}

#[test]
fn test_binary_reader_reads_all_big_endian_methods() {
    let mut bytes = Vec::new();
    push_be_values(&mut bytes);
    let mut reader = BinaryReader::<_, BigEndian>::new(Cursor::new(bytes));

    assert_eq!(ByteOrder::BigEndian, reader.byte_order());
    let mut prefix = [0u8; 2];
    std::io::Read::read_exact(&mut reader, &mut prefix).expect("bytes should be read");
    assert_eq!([0xaa, 0xbb], prefix);
    assert_eq!(0x12, reader.read_u8().expect("u8 should be read"));
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
    assert_eq!(
        "hi",
        reader
            .read_utf8_string_u16(usize::MAX)
            .expect("u16 string should be read")
    );
    assert_eq!(
        "ok",
        reader
            .read_utf8_string_u32(usize::MAX)
            .expect("u32 string should be read")
    );
}

#[test]
fn test_binary_reader_reads_little_endian_and_exposes_accessors() {
    let mut reader = BinaryReader::<_, LittleEndian>::new(Cursor::new(vec![
        0xaa, 0xbb, 0x34, 0x12, 0x02, 0x00, b'h', b'i', 0x02, 0x00, 0x00, 0x00, b'o', b'k',
    ]));

    assert_eq!(ByteOrder::LittleEndian, reader.byte_order());
    assert_eq!(0, reader.get_ref().position());
    reader.get_mut().set_position(0);
    let mut prefix = [0u8; 2];
    std::io::Read::read_exact(&mut reader, &mut prefix).expect("bytes should be read");
    assert_eq!([0xaa, 0xbb], prefix);
    assert_eq!(0x1234, reader.read_u16().expect("u16 should be read"));
    assert_eq!(
        "hi",
        reader
            .read_utf8_string_u16(usize::MAX)
            .expect("u16 string should be read")
    );
    assert_eq!(
        "ok",
        reader
            .read_utf8_string_u32(usize::MAX)
            .expect("u32 string should be read")
    );
    assert_eq!(14, reader.into_inner().position());
}

#[test]
fn test_binary_reader_reports_read_and_utf8_errors() {
    let mut reader = BinaryReader::<_, BigEndian>::new(Cursor::new(vec![0x12]));
    assert_eq!(
        ErrorKind::UnexpectedEof,
        reader
            .read_u16()
            .expect_err("truncated u16 should fail")
            .kind()
    );

    let mut reader = BinaryReader::<_, BigEndian>::new(Cursor::new(vec![0x00, 0x02, 0xff, 0xff]));
    assert_eq!(
        ErrorKind::InvalidData,
        reader
            .read_utf8_string_u16(usize::MAX)
            .expect_err("invalid UTF-8 should fail")
            .kind()
    );

    let mut reader =
        BinaryReader::<_, BigEndian>::new(Cursor::new(vec![0x00, 0x03, b'a', b'b', b'c']));
    assert_eq!(
        ErrorKind::InvalidData,
        reader
            .read_utf8_string_u16(2)
            .expect_err("oversized u16 string should fail")
            .kind()
    );

    let mut reader = BinaryReader::<_, BigEndian>::new(Cursor::new(vec![
        0x00, 0x00, 0x00, 0x03, b'a', b'b', b'c',
    ]));
    assert_eq!(
        ErrorKind::InvalidData,
        reader
            .read_utf8_string_u32(2)
            .expect_err("oversized u32 string should fail")
            .kind()
    );
}

#[test]
fn test_binary_reader_read_and_seek_delegate_to_inner_reader() {
    let mut reader =
        qubit_io::BinaryReader::<_, qubit_io::LittleEndian>::new(std::io::Cursor::new(vec![
            1, 2, 3, 4,
        ]));

    std::io::Seek::seek(&mut reader, std::io::SeekFrom::Start(1))
        .expect("seeking through BinaryReader should succeed");
    let mut bytes = [0_u8; 2];
    std::io::Read::read_exact(&mut reader, &mut bytes)
        .expect("reading through BinaryReader should succeed");

    assert_eq!(bytes, [2, 3]);
}
