use std::cell::RefCell;
use std::io::{Error, ErrorKind, Write};
use std::rc::Rc;

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

struct ChunkedWriter {
    output: Rc<RefCell<Vec<u8>>>,
    request_lengths: Rc<RefCell<Vec<usize>>>,
    max_chunk_len: usize,
}

impl ChunkedWriter {
    fn new(
        output: Rc<RefCell<Vec<u8>>>,
        request_lengths: Rc<RefCell<Vec<usize>>>,
        max_chunk_len: usize,
    ) -> Self {
        Self {
            output,
            request_lengths,
            max_chunk_len,
        }
    }
}

impl Write for ChunkedWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.request_lengths.borrow_mut().push(buffer.len());
        let count = buffer.len().min(self.max_chunk_len);
        self.output.borrow_mut().extend_from_slice(&buffer[..count]);
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct PartialErrorWriter {
    output: Rc<RefCell<Vec<u8>>>,
    write_count: usize,
}

impl PartialErrorWriter {
    fn new(output: Rc<RefCell<Vec<u8>>>) -> Self {
        Self {
            output,
            write_count: 0,
        }
    }
}

impl Write for PartialErrorWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.write_count += 1;
        match self.write_count {
            1 => {
                let count = buffer.len().min(2);
                self.output.borrow_mut().extend_from_slice(&buffer[..count]);
                Ok(count)
            }
            2 => Err(Error::other("write failed after partial write")),
            _ => {
                self.output.borrow_mut().extend_from_slice(buffer);
                Ok(buffer.len())
            }
        }
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

#[test]
fn test_buffered_binary_writer_delegates_large_raw_write_once() {
    let output = Rc::new(RefCell::new(Vec::new()));
    let request_lengths = Rc::new(RefCell::new(Vec::new()));
    let inner = ChunkedWriter::new(Rc::clone(&output), Rc::clone(&request_lengths), 8);
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::with_capacity(inner, 19);
    let bytes: Vec<u8> = (0u8..32).collect();

    let count = writer.write(&bytes).expect("raw bytes should be written");

    assert_eq!(8, count);
    assert_eq!((0u8..8).collect::<Vec<_>>(), *output.borrow());
    assert_eq!(vec![32], *request_lengths.borrow());
}

#[test]
fn test_buffered_binary_writer_drops_flushed_prefix_after_error() {
    let output = Rc::new(RefCell::new(Vec::new()));
    let inner = PartialErrorWriter::new(Rc::clone(&output));
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::with_capacity(inner, 19);

    writer.write_u32(0x0102_0304).expect("value should buffer");
    let error = writer.flush().expect_err("partial flush should fail");
    assert_eq!(ErrorKind::Other, error.kind());
    writer
        .flush()
        .expect("remaining buffered bytes should flush");

    assert_eq!([4, 3, 2, 1], output.borrow().as_slice());
}
