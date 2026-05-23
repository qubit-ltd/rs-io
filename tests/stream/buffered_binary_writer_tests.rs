use std::io::{Error, ErrorKind, Write};

use qubit_io::{BinaryWriteExt, BufferedBinaryWriter, ByteOrder, LittleEndian};

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
        Err(Error::other("write failed"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn expected_values() -> Vec<u8> {
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
fn test_buffered_binary_writer_writes_scalars_across_buffer_boundaries() {
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::with_capacity(Vec::new(), 9);

    assert_eq!(ByteOrder::LittleEndian, writer.byte_order());
    writer.write_u8(0xaa).expect("u8 should be written");
    writer.write_i8(-2).expect("i8 should be written");
    writer.write_u16(0x1234).expect("u16 should be written");
    writer
        .write_u32(0x1234_5678)
        .expect("u32 should be written");
    writer
        .write_u64(0x0123_4567_89ab_cdef)
        .expect("u64 should be written");
    writer
        .write_u128(0x0123_4567_89ab_cdef_fedc_ba98_7654_3210)
        .expect("u128 should be written");
    writer.write_i16(-0x1234).expect("i16 should be written");
    writer
        .write_i32(-0x0123_4567)
        .expect("i32 should be written");
    writer
        .write_i64(-0x0123_4567_89ab_cdef)
        .expect("i64 should be written");
    writer
        .write_i128(-0x0123_4567_89ab_cdef_fedc_ba98_7654_3210)
        .expect("i128 should be written");
    writer.write_f32(12.5).expect("f32 should be written");
    writer.write_f64(-25.25).expect("f64 should be written");

    assert_eq!(
        expected_values(),
        writer.into_inner().expect("writer should flush")
    );
}

#[test]
fn test_buffered_binary_writer_returns_writer_error() {
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::with_capacity(FailingWriter, 8);

    writer.write_u64(0x1234).expect("value should be buffered");
    let error = writer.flush().expect_err("flush should fail");

    assert_eq!(ErrorKind::Other, error.kind());
}
