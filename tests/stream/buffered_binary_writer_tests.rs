use std::cell::RefCell;
use std::io::{
    Cursor,
    Error,
    ErrorKind,
    Seek,
    SeekFrom,
    Write,
};
use std::rc::Rc;

use qubit_io::{
    BinaryWriteExt,
    BufferedBinaryWriter,
    ByteOrder,
    LittleEndian,
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

struct FailingSeekWriter;

struct SeekErrorWriter {
    output: Vec<u8>,
}

struct FlushErrorWriter;

impl Write for FailingSeekWriter {
    fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
        Err(Error::other("write failed"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl SeekErrorWriter {
    fn new() -> Self {
        Self { output: Vec::new() }
    }
}

impl Write for SeekErrorWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.output.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Seek for SeekErrorWriter {
    fn seek(&mut self, _position: SeekFrom) -> std::io::Result<u64> {
        Err(Error::other("seek failed"))
    }
}

impl Write for FlushErrorWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(Error::other("flush failed"))
    }
}

impl Seek for FailingSeekWriter {
    fn seek(&mut self, _position: SeekFrom) -> std::io::Result<u64> {
        Ok(0)
    }
}

struct ZeroWriter;

impl Write for ZeroWriter {
    fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
        Ok(0)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct InterruptedOnceWriter {
    output: Vec<u8>,
    interrupted: bool,
}

struct InterruptedThenErrorWriter {
    interrupted: bool,
}

impl InterruptedOnceWriter {
    fn new() -> Self {
        Self {
            output: Vec::new(),
            interrupted: false,
        }
    }
}

impl InterruptedThenErrorWriter {
    const fn new() -> Self {
        Self { interrupted: false }
    }
}

impl Write for InterruptedOnceWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(Error::new(ErrorKind::Interrupted, "interrupted once"));
        }
        self.output.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Write for InterruptedThenErrorWriter {
    fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(Error::new(ErrorKind::Interrupted, "interrupted once"));
        }
        Err(Error::other("write failed after interruption"))
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
    fn new(output: Rc<RefCell<Vec<u8>>>, request_lengths: Rc<RefCell<Vec<usize>>>, max_chunk_len: usize) -> Self {
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
        Self { output, write_count: 0 }
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
    bytes.write_u32_le(0x1234_5678).expect("u32 should be encoded");
    bytes
        .write_u64_le(0x0123_4567_89ab_cdef)
        .expect("u64 should be encoded");
    bytes
        .write_u128_le(0x0123_4567_89ab_cdef_fedc_ba98_7654_3210)
        .expect("u128 should be encoded");
    bytes.write_i16_le(-0x1234).expect("i16 should be encoded");
    bytes.write_i32_le(-0x0123_4567).expect("i32 should be encoded");
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
    writer.write_u32(0x1234_5678).expect("u32 should be written");
    writer.write_u64(0x0123_4567_89ab_cdef).expect("u64 should be written");
    writer
        .write_u128(0x0123_4567_89ab_cdef_fedc_ba98_7654_3210)
        .expect("u128 should be written");
    writer.write_i16(-0x1234).expect("i16 should be written");
    writer.write_i32(-0x0123_4567).expect("i32 should be written");
    writer.write_i64(-0x0123_4567_89ab_cdef).expect("i64 should be written");
    writer
        .write_i128(-0x0123_4567_89ab_cdef_fedc_ba98_7654_3210)
        .expect("i128 should be written");
    writer.write_f32(12.5).expect("f32 should be written");
    writer.write_f64(-25.25).expect("f64 should be written");

    assert_eq!(expected_values(), writer.into_inner().expect("writer should flush"));
}

#[test]
fn test_buffered_binary_writer_accessors_write_all_seek_and_into_inner() {
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::new(Cursor::new(Vec::new()));

    assert_eq!(ByteOrder::LittleEndian, writer.byte_order());
    assert_eq!(0, writer.get_ref().position());
    writer.get_mut().set_position(0);
    writer.write_u8(1).expect("u8 should be buffered");
    assert_eq!(2, writer.write(&[2, 3]).expect("raw bytes should be buffered"));
    assert_eq!(
        3,
        writer.seek(SeekFrom::End(0)).expect("seek should flush pending bytes")
    );
    writer.write_u8(4).expect("u8 should be buffered after seek");

    let inner = writer.into_inner().expect("into_inner should flush");

    assert_eq!(vec![1, 2, 3, 4], inner.into_inner());
}

#[test]
fn test_buffered_binary_writer_cursor_cold_paths_and_fixed_flush() {
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::with_capacity(Cursor::new(Vec::new()), 19);

    writer.write_all(&[1; 14]).expect("initial bytes should be buffered");
    writer
        .write_all(&[2; 5])
        .expect("exact spare write_all should stay buffered");
    writer.write_u16(0x0403).expect("fixed value should flush full buffer");
    writer
        .write_all(&[5; 16])
        .expect("bytes should fill most of the buffer");
    assert_eq!(5, writer.write(&[6; 5]).expect("write should flush then buffer"));

    let cursor = writer.into_inner().expect("writer should flush");

    let mut expected = vec![1; 14];
    expected.extend_from_slice(&[2; 5]);
    expected.extend_from_slice(&[3, 4]);
    expected.extend_from_slice(&[5; 16]);
    expected.extend_from_slice(&[6; 5]);
    assert_eq!(expected, cursor.into_inner());
}

#[test]
fn test_buffered_binary_writer_write_all_direct_and_buffered_slow_paths() {
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::with_capacity(Vec::new(), 19);
    let large: Vec<u8> = (0u8..32).collect();

    writer.write_all(&large).expect("large write_all should delegate");
    writer.write_all(&[1; 18]).expect("small write_all should buffer");
    writer.write_all(&[2; 5]).expect("write_all should flush then buffer");

    let mut expected = large;
    expected.extend_from_slice(&[1; 18]);
    expected.extend_from_slice(&[2; 5]);
    assert_eq!(expected, writer.into_inner().expect("writer should flush"));
}

#[test]
fn test_buffered_binary_writer_write_all_exact_spare_uses_cold_no_flush_path() {
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::with_capacity(Vec::new(), 19);

    writer.write_all(&[1; 14]).expect("initial bytes should be buffered");
    writer
        .write_all(&[2; 5])
        .expect("exact spare write_all should stay buffered");

    let mut expected = vec![1; 14];
    expected.extend_from_slice(&[2; 5]);
    assert_eq!(expected, writer.into_inner().expect("writer should flush"));
}

#[test]
fn test_buffered_binary_writer_write_flushes_then_buffers_small_input() {
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::with_capacity(Vec::new(), 19);

    writer.write_all(&[1; 18]).expect("initial bytes should be buffered");
    assert_eq!(5, writer.write(&[2; 5]).expect("small write should flush then buffer"));

    let mut expected = vec![1; 18];
    expected.extend_from_slice(&[2; 5]);
    assert_eq!(expected, writer.into_inner().expect("writer should flush"));
}

#[test]
fn test_buffered_binary_writer_write_exact_spare_uses_cold_no_flush_path() {
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::with_capacity(Vec::new(), 19);

    writer.write_all(&[1; 14]).expect("initial bytes should be buffered");
    assert_eq!(
        5,
        writer.write(&[2; 5]).expect("exact spare write should stay buffered")
    );

    let mut expected = vec![1; 14];
    expected.extend_from_slice(&[2; 5]);
    assert_eq!(expected, writer.into_inner().expect("writer should flush"));
}

#[test]
fn test_buffered_binary_writer_flushes_before_fixed_value_when_full() {
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::with_capacity(Vec::new(), 19);

    writer.write_all(&[1; 18]).expect("initial bytes should be buffered");
    writer.write_u16(0x0203).expect("fixed value should flush then buffer");

    let mut expected = vec![1; 18];
    expected.extend_from_slice(&[3, 2]);
    assert_eq!(expected, writer.into_inner().expect("writer should flush"));
}

#[test]
fn test_buffered_binary_writer_reports_flush_error_before_fixed_value() {
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::with_capacity(FailingWriter, 19);

    writer.write_all(&[1; 18]).expect("initial bytes should be buffered");
    let error = writer
        .write_u16(0x0203)
        .expect_err("fixed value should report flush error");

    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_buffered_binary_writer_write_all_reports_flush_error() {
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::with_capacity(FailingWriter, 19);

    writer.write_all(&[1; 18]).expect("initial bytes should be buffered");
    let error = writer
        .write_all(&[2; 5])
        .expect_err("write_all should report flush error");

    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_buffered_binary_writer_write_reports_flush_error() {
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::with_capacity(FailingWriter, 19);

    writer.write_all(&[1; 18]).expect("initial bytes should be buffered");
    let error = writer.write(&[2; 5]).expect_err("write should report flush error");

    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_buffered_binary_writer_large_write_reports_inner_error_without_pending_buffer() {
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::with_capacity(FailingWriter, 19);

    let write_error = writer
        .write(&[1; 19])
        .expect_err("large write should report writer error");
    let write_all_error = writer
        .write_all(&[1; 19])
        .expect_err("large write_all should report writer error");

    assert_eq!(ErrorKind::Other, write_error.kind());
    assert_eq!(ErrorKind::Other, write_all_error.kind());
}

#[test]
fn test_buffered_binary_writer_seek_reports_flush_error() {
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::with_capacity(FailingSeekWriter, 19);

    writer.write_all(&[1; 18]).expect("initial bytes should be buffered");
    let error = writer
        .seek(SeekFrom::Start(0))
        .expect_err("seek should report flush error");

    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_buffered_binary_writer_reports_inner_seek_error_after_flush() {
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::with_capacity(SeekErrorWriter::new(), 19);

    writer.write_u8(1).expect("value should be buffered");
    let error = writer
        .seek(SeekFrom::Start(0))
        .expect_err("inner seek error should be returned after flushing");

    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_buffered_binary_writer_reports_inner_flush_error_without_pending_buffer() {
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::with_capacity(FlushErrorWriter, 19);

    let error = writer.flush().expect_err("inner flush error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_buffered_binary_writer_into_inner_returns_flush_error() {
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::with_capacity(FailingWriter, 8);

    writer.write_u32(0x0102_0304).expect("value should be buffered");
    let error = match writer.into_inner() {
        Ok(_) => panic!("into_inner should fail while flushing"),
        Err(error) => error,
    };

    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_buffered_binary_writer_reports_write_zero_while_flushing() {
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::with_capacity(ZeroWriter, 8);

    writer.write_u32(0x0102_0304).expect("value should be buffered");
    let error = writer.flush().expect_err("zero write should fail");

    assert_eq!(ErrorKind::WriteZero, error.kind());
}

#[test]
fn test_buffered_binary_writer_retries_interrupted_flush() {
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::with_capacity(InterruptedOnceWriter::new(), 8);

    writer.write_u32(0x0102_0304).expect("value should be buffered");
    let inner = writer.into_inner().expect("flush should retry interruption");

    assert_eq!(vec![4, 3, 2, 1], inner.output);
}

#[test]
fn test_buffered_binary_writer_returns_error_after_interrupted_flush() {
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::with_capacity(InterruptedThenErrorWriter::new(), 8);

    writer.write_u32(0x0102_0304).expect("value should be buffered");
    let error = writer
        .flush()
        .expect_err("non-interrupted write error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
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
fn test_buffered_binary_writer_buffers_small_write_with_chunked_writer() {
    let output = Rc::new(RefCell::new(Vec::new()));
    let request_lengths = Rc::new(RefCell::new(Vec::new()));
    let inner = ChunkedWriter::new(Rc::clone(&output), Rc::clone(&request_lengths), 8);
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::with_capacity(inner, 19);

    writer.write_all(&[1, 2, 3]).expect("small write should be buffered");
    assert!(output.borrow().is_empty());
    assert!(request_lengths.borrow().is_empty());
    writer.flush().expect("flush should write buffered bytes");

    assert_eq!(vec![1, 2, 3], *output.borrow());
    assert_eq!(vec![3], *request_lengths.borrow());
}

#[test]
fn test_buffered_binary_writer_drops_flushed_prefix_after_error() {
    let output = Rc::new(RefCell::new(Vec::new()));
    let inner = PartialErrorWriter::new(Rc::clone(&output));
    let mut writer = BufferedBinaryWriter::<_, LittleEndian>::with_capacity(inner, 19);

    writer.write_u32(0x0102_0304).expect("value should buffer");
    let error = writer.flush().expect_err("partial flush should fail");
    assert_eq!(ErrorKind::Other, error.kind());
    writer.flush().expect("remaining buffered bytes should flush");

    assert_eq!([4, 3, 2, 1], output.borrow().as_slice());
}
