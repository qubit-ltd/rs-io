use std::cell::RefCell;
use std::io::{
    Cursor,
    Error,
    ErrorKind,
    Read,
    Seek,
    SeekFrom,
};
use std::rc::Rc;

use qubit_io::{
    BinaryWriteExt,
    BufferedBinaryReader,
    ByteOrder,
    LittleEndian,
};

fn encoded_values() -> Vec<u8> {
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

struct ChunkedReader {
    bytes: Vec<u8>,
    position: usize,
    max_chunk_len: usize,
    request_lengths: Rc<RefCell<Vec<usize>>>,
}

impl ChunkedReader {
    fn new(bytes: Vec<u8>, max_chunk_len: usize, request_lengths: Rc<RefCell<Vec<usize>>>) -> Self {
        Self {
            bytes,
            position: 0,
            max_chunk_len,
            request_lengths,
        }
    }
}

impl Read for ChunkedReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.request_lengths.borrow_mut().push(buffer.len());
        let remaining = self.bytes.len() - self.position;
        let count = remaining.min(buffer.len()).min(self.max_chunk_len);
        buffer[..count].copy_from_slice(&self.bytes[self.position..self.position + count]);
        self.position += count;
        Ok(count)
    }
}

struct RejectingCurrentSeekReader {
    bytes: Vec<u8>,
    position: usize,
}

impl RejectingCurrentSeekReader {
    fn new(bytes: Vec<u8>) -> Self {
        Self { bytes, position: 0 }
    }
}

struct InterruptedOnceReader {
    bytes: Vec<u8>,
    position: usize,
    interrupted: bool,
}

struct InterruptedThenErrorReader {
    interrupted: bool,
}

impl InterruptedOnceReader {
    fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes,
            position: 0,
            interrupted: false,
        }
    }
}

impl InterruptedThenErrorReader {
    const fn new() -> Self {
        Self { interrupted: false }
    }
}

struct ByteThenErrorReader {
    byte: u8,
    returned: bool,
}

struct ErrorReader;

impl ByteThenErrorReader {
    const fn new(byte: u8) -> Self {
        Self { byte, returned: false }
    }
}

impl Read for InterruptedOnceReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(Error::new(ErrorKind::Interrupted, "interrupted once"));
        }
        let remaining = self.bytes.len() - self.position;
        let count = remaining.min(buffer.len());
        buffer[..count].copy_from_slice(&self.bytes[self.position..self.position + count]);
        self.position += count;
        Ok(count)
    }
}

impl Read for InterruptedThenErrorReader {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(Error::new(ErrorKind::Interrupted, "interrupted once"));
        }
        Err(Error::other("read failed after interruption"))
    }
}

impl Read for ByteThenErrorReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.returned {
            return Err(Error::other("read failed"));
        }
        self.returned = true;
        buffer[0] = self.byte;
        Ok(1)
    }
}

impl Read for ErrorReader {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        Err(Error::other("read failed"))
    }
}

impl Read for RejectingCurrentSeekReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let remaining = self.bytes.len() - self.position;
        let count = remaining.min(buffer.len());
        buffer[..count].copy_from_slice(&self.bytes[self.position..self.position + count]);
        self.position += count;
        Ok(count)
    }
}

impl Seek for RejectingCurrentSeekReader {
    fn seek(&mut self, position: SeekFrom) -> std::io::Result<u64> {
        match position {
            SeekFrom::Start(position) => {
                self.position = usize::try_from(position)
                    .map_err(|_| Error::new(ErrorKind::InvalidInput, "seek position exceeds usize"))?;
                Ok(position)
            }
            SeekFrom::Current(_) => Err(Error::other("current seek rejected")),
            SeekFrom::End(_) => Err(Error::new(ErrorKind::Unsupported, "unsupported seek")),
        }
    }
}

#[test]
fn test_buffered_binary_reader_reads_scalars_across_buffer_boundaries() {
    let mut reader = BufferedBinaryReader::<_, LittleEndian>::with_capacity(Cursor::new(encoded_values()), 9);

    assert_eq!(ByteOrder::LittleEndian, reader.byte_order());
    assert_eq!(0xaa, reader.read_u8().expect("u8 should be read"));
    assert_eq!(-2, reader.read_i8().expect("i8 should be read"));
    assert_eq!(0x1234, reader.read_u16().expect("u16 should be read"));
    assert_eq!(0x1234_5678, reader.read_u32().expect("u32 should be read"));
    assert_eq!(0x0123_4567_89ab_cdef, reader.read_u64().expect("u64 should be read"));
    assert_eq!(
        0x0123_4567_89ab_cdef_fedc_ba98_7654_3210,
        reader.read_u128().expect("u128 should be read")
    );
    assert_eq!(-0x1234, reader.read_i16().expect("i16 should be read"));
    assert_eq!(-0x0123_4567, reader.read_i32().expect("i32 should be read"));
    assert_eq!(-0x0123_4567_89ab_cdef, reader.read_i64().expect("i64 should be read"));
    assert_eq!(
        -0x0123_4567_89ab_cdef_fedc_ba98_7654_3210,
        reader.read_i128().expect("i128 should be read")
    );
    assert_eq!(12.5, reader.read_f32().expect("f32 should be read"));
    assert_eq!(-25.25, reader.read_f64().expect("f64 should be read"));
}

#[test]
fn test_buffered_binary_reader_reports_unexpected_eof() {
    let mut reader = BufferedBinaryReader::<_, LittleEndian>::with_capacity(Cursor::new(vec![0x34]), 8);

    let error = reader.read_u16().expect_err("truncated u16 should fail");

    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
}

#[test]
fn test_buffered_binary_reader_reports_refill_error_after_partial_scalar() {
    let mut reader = BufferedBinaryReader::<_, LittleEndian>::with_capacity(ByteThenErrorReader::new(0x34), 4);

    let error = reader
        .read_u16()
        .expect_err("reader error after a partial scalar should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_buffered_binary_reader_implements_read() {
    let mut reader = BufferedBinaryReader::<_, LittleEndian>::with_capacity(Cursor::new(vec![1, 2, 3, 4]), 2);
    let mut bytes = [0u8; 3];

    reader.read_exact(&mut bytes).expect("raw bytes should be read");

    assert_eq!([1, 2, 3], bytes);
    assert_eq!(4, reader.read_u8().expect("remaining byte should be read"));
}

#[test]
fn test_buffered_binary_reader_bypasses_buffer_for_large_raw_read() {
    let request_lengths = Rc::new(RefCell::new(Vec::new()));
    let inner = ChunkedReader::new((0u8..32).collect(), usize::MAX, Rc::clone(&request_lengths));
    let mut reader = BufferedBinaryReader::<_, LittleEndian>::with_capacity(inner, 19);
    let mut bytes = [0u8; 32];

    let count = reader.read(&mut bytes).expect("raw bytes should be read");

    assert_eq!(32, count);
    assert_eq!((0u8..32).collect::<Vec<_>>(), bytes);
    assert_eq!(vec![32], *request_lengths.borrow());
}

#[test]
fn test_buffered_binary_reader_reports_eof_for_empty_small_raw_read() {
    let mut reader = BufferedBinaryReader::<_, LittleEndian>::with_capacity(Cursor::new(Vec::<u8>::new()), 19);
    let mut byte = [0_u8; 1];

    assert_eq!(0, reader.read(&mut byte).expect("empty reader should report EOF"));
}

#[test]
fn test_buffered_binary_reader_returns_refill_error_for_small_raw_read() {
    let mut reader = BufferedBinaryReader::<_, LittleEndian>::with_capacity(ErrorReader, 19);
    let mut byte = [0_u8; 1];

    let error = reader
        .read(&mut byte)
        .expect_err("small raw read should return refill error");

    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_buffered_binary_reader_retries_interrupted_small_raw_read() {
    let mut reader = BufferedBinaryReader::<_, LittleEndian>::with_capacity(InterruptedOnceReader::new(vec![7]), 19);
    let mut byte = [0_u8; 1];

    assert_eq!(1, reader.read(&mut byte).expect("small raw read should retry"));
    assert_eq!([7], byte);
}

#[test]
fn test_buffered_binary_reader_appends_before_backshifting() {
    let request_lengths = Rc::new(RefCell::new(Vec::new()));
    let inner = ChunkedReader::new((0u8..40).collect(), 20, Rc::clone(&request_lengths));
    let mut reader = BufferedBinaryReader::<_, LittleEndian>::with_capacity(inner, 32);

    let _ = reader.read_u128().expect("u128 should be read");
    let _ = reader.read_u64().expect("u64 should be read");

    assert_eq!(vec![32, 12], *request_lengths.borrow());
}

#[test]
fn test_buffered_binary_reader_refills_fixed_value_when_tail_has_room() {
    let request_lengths = Rc::new(RefCell::new(Vec::new()));
    let inner = ChunkedReader::new(vec![0x34, 0x12], 1, Rc::clone(&request_lengths));
    let mut reader = BufferedBinaryReader::<_, LittleEndian>::with_capacity(inner, 19);

    assert_eq!(0x1234, reader.read_u16().expect("split u16 should be read"));
    assert_eq!(vec![19, 18], *request_lengths.borrow());
}

#[test]
fn test_buffered_binary_reader_accessors_raw_read_seek_and_into_inner() {
    let mut reader = BufferedBinaryReader::<_, LittleEndian>::new(Cursor::new(vec![1, 2, 3, 4, 5]));

    assert_eq!(ByteOrder::LittleEndian, reader.byte_order());
    assert_eq!(0, reader.get_ref().position());
    reader.get_mut().set_position(1);
    assert_eq!(0, reader.read(&mut []).expect("empty read should succeed"));
    assert_eq!(0x0302, reader.read_u16().expect("u16 should be read"));
    assert_eq!(
        3,
        reader.stream_position().expect("logical current seek should succeed")
    );
    assert_eq!(4, reader.read_u8().expect("byte after current seek should be read"));
    assert_eq!(
        1,
        reader.seek(SeekFrom::Start(1)).expect("absolute seek should succeed")
    );

    let inner = reader.into_inner();

    assert!(inner.position() >= 1);
}

#[test]
fn test_buffered_binary_reader_retries_interrupted_refill() {
    let inner = InterruptedOnceReader::new(vec![9]);
    let mut reader = BufferedBinaryReader::<_, LittleEndian>::with_capacity(inner, 4);

    assert_eq!(9, reader.read_u8().expect("read should retry interruption"));
}

#[test]
fn test_buffered_binary_reader_returns_error_after_interrupted_refill() {
    let inner = InterruptedThenErrorReader::new();
    let mut reader = BufferedBinaryReader::<_, LittleEndian>::with_capacity(inner, 4);

    let error = reader
        .read_u8()
        .expect_err("non-interrupted refill error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_buffered_binary_reader_current_seek_without_unread_buffer() {
    let mut reader = BufferedBinaryReader::<_, LittleEndian>::with_capacity(Cursor::new(vec![1, 2, 3]), 4);

    assert_eq!(
        2,
        reader
            .seek(SeekFrom::Current(2))
            .expect("current seek without unread bytes should succeed")
    );
    assert_eq!(3, reader.read_u8().expect("seek target byte should be read"));
}

#[test]
fn test_buffered_binary_reader_current_seek_reports_underflow() {
    let mut reader = BufferedBinaryReader::<_, LittleEndian>::with_capacity(Cursor::new(vec![1, 2, 3, 4]), 4);

    assert_eq!(1, reader.read_u8().expect("first byte should be read"));

    let error = reader
        .seek(SeekFrom::Current(i64::MIN))
        .expect_err("seek underflow should be reported");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
    assert_eq!(
        2,
        reader.read_u8().expect("unread buffered byte should remain readable")
    );
}

#[test]
fn test_buffered_binary_reader_preserves_buffer_when_seek_fails() {
    let inner = RejectingCurrentSeekReader::new(vec![1, 2, 3, 4, 5]);
    let mut reader = BufferedBinaryReader::<_, LittleEndian>::with_capacity(inner, 4);

    assert_eq!(1, reader.read_u8().expect("first byte should be read"));

    let error = reader.stream_position().expect_err("inner seek should fail");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!(2, reader.read_u8().expect("unread buffered byte should be preserved"));
}

#[test]
fn test_buffered_binary_reader_seek_end_discards_buffer_after_success() {
    let mut reader = BufferedBinaryReader::<_, LittleEndian>::with_capacity(Cursor::new(vec![1, 2, 3, 4]), 4);

    assert_eq!(1, reader.read_u8().expect("first byte should be read"));
    assert_eq!(4, reader.seek(SeekFrom::End(0)).expect("seek to end should succeed"));

    let mut byte = [0_u8; 1];
    assert_eq!(0, reader.read(&mut byte).expect("reader should be at EOF"));
}
