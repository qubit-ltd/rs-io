use std::io::{
    Cursor,
    Error,
    ErrorKind,
    Read,
    Seek,
};

use qubit_io::{
    BufferedLeb128Reader,
    Leb128WriteExt,
    NonStrict,
    Strict,
};

struct ByteThenErrorReader {
    byte: u8,
    returned: bool,
}

struct InterruptedOnceReader {
    bytes: Vec<u8>,
    position: usize,
    interrupted: bool,
}

struct ChunkedReader {
    bytes: Vec<u8>,
    position: usize,
    max_chunk_len: usize,
}

impl ByteThenErrorReader {
    const fn new(byte: u8) -> Self {
        Self { byte, returned: false }
    }
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

impl ChunkedReader {
    fn new(bytes: Vec<u8>, max_chunk_len: usize) -> Self {
        Self {
            bytes,
            position: 0,
            max_chunk_len,
        }
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

impl Read for ChunkedReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let remaining = self.bytes.len() - self.position;
        let count = remaining.min(buffer.len()).min(self.max_chunk_len);
        buffer[..count].copy_from_slice(&self.bytes[self.position..self.position + count]);
        self.position += count;
        Ok(count)
    }
}

#[test]
fn test_buffered_leb128_reader_reads_values_across_buffer_boundaries() {
    let mut bytes = Vec::new();
    bytes.write_uleb_u8(u8::MAX).expect("u8 should be encoded");
    bytes.write_uleb_u16(300).expect("u16 should be encoded");
    bytes.write_uleb_u32(0x1f600).expect("u32 should be encoded");
    bytes
        .write_uleb_u64(0x0102_0304_0506_0708)
        .expect("u64 should be encoded");
    bytes
        .write_uleb_u128(0x0102_0304_0506_0708_1112_1314_1516_1718)
        .expect("u128 should be encoded");
    bytes.write_uleb_usize(usize::MAX).expect("usize should be encoded");
    bytes.write_sleb_i8(i8::MIN).expect("i8 should be encoded");
    bytes.write_sleb_i16(-300).expect("i16 should be encoded");
    bytes.write_sleb_i32(-0x1f600).expect("i32 should be encoded");
    bytes
        .write_sleb_i64(-0x0102_0304_0506_0708)
        .expect("i64 should be encoded");
    bytes
        .write_sleb_i128(-0x0102_0304_0506_0708_1112_1314_1516_1718)
        .expect("i128 should be encoded");
    bytes.write_sleb_isize(isize::MIN).expect("isize should be encoded");

    let mut reader = BufferedLeb128Reader::<_, NonStrict>::with_capacity(Cursor::new(bytes), 3);

    assert!(!reader.is_strict());
    assert_eq!(u8::MAX, reader.read_u8().expect("u8 should be read"));
    assert_eq!(300, reader.read_u16().expect("u16 should be read"));
    assert_eq!(0x1f600, reader.read_u32().expect("u32 should be read"));
    assert_eq!(0x0102_0304_0506_0708, reader.read_u64().expect("u64 should be read"));
    assert_eq!(
        0x0102_0304_0506_0708_1112_1314_1516_1718,
        reader.read_u128().expect("u128 should be read")
    );
    assert_eq!(usize::MAX, reader.read_usize().expect("usize should be read"));
    assert_eq!(i8::MIN, reader.read_i8().expect("i8 should be read"));
    assert_eq!(-300, reader.read_i16().expect("i16 should be read"));
    assert_eq!(-0x1f600, reader.read_i32().expect("i32 should be read"));
    assert_eq!(-0x0102_0304_0506_0708, reader.read_i64().expect("i64 should be read"));
    assert_eq!(
        -0x0102_0304_0506_0708_1112_1314_1516_1718,
        reader.read_i128().expect("i128 should be read")
    );
    assert_eq!(isize::MIN, reader.read_isize().expect("isize should be read"));
}

#[test]
fn test_buffered_leb128_reader_accessors_raw_seek_string_and_into_inner() {
    let mut reader = BufferedLeb128Reader::<_, NonStrict>::new(Cursor::new(vec![3, b'a', b'b', b'c', 9]));

    assert!(!reader.is_strict());
    assert_eq!(0, reader.get_ref().position());
    reader.get_mut().set_position(0);
    assert_eq!("abc", reader.read_utf8_string(3).expect("string should be read"));
    assert_eq!(4, reader.stream_position().expect("current seek should succeed"));
    let mut byte = [0_u8; 1];
    reader.read_exact(&mut byte).expect("raw byte should be read");
    assert_eq!([9], byte);

    let inner = reader.into_inner();

    assert!(inner.position() >= 5);
}

#[test]
fn test_buffered_leb128_reader_reports_invalid_and_truncated_values() {
    let mut reader = BufferedLeb128Reader::<_, Strict>::with_capacity(Cursor::new(vec![0x80, 0x00]), 2);
    assert!(reader.is_strict());
    assert_eq!(
        ErrorKind::InvalidData,
        reader.read_u16().expect_err("non-canonical value should fail").kind()
    );

    let mut reader = BufferedLeb128Reader::<_, NonStrict>::with_capacity(Cursor::new(vec![0x80]), 2);
    assert_eq!(
        ErrorKind::UnexpectedEof,
        reader.read_u64().expect_err("truncated value should report EOF").kind()
    );
}

#[test]
fn test_buffered_leb128_reader_reports_refill_error_after_partial_payload() {
    let mut reader = BufferedLeb128Reader::<_, NonStrict>::with_capacity(ByteThenErrorReader::new(0x80), 2);

    let error = reader
        .read_u16()
        .expect_err("reader error after a partial payload should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_buffered_leb128_reader_refills_without_backshift_when_tail_has_room() {
    let inner = ChunkedReader::new(vec![0x80, 0x01], 1);
    let mut reader = BufferedLeb128Reader::<_, NonStrict>::with_capacity(inner, 19);

    assert_eq!(128, reader.read_u16().expect("split value should be read"));
}

#[test]
fn test_buffered_leb128_reader_retries_interrupted_refill() {
    let inner = InterruptedOnceReader::new(vec![0x80, 0x01]);
    let mut reader = BufferedLeb128Reader::<_, NonStrict>::with_capacity(inner, 19);

    assert_eq!(128, reader.read_u16().expect("read should retry interruption"));
}

#[test]
fn test_buffered_leb128_reader_consumes_invalid_payload_before_reporting_error() {
    let mut reader = BufferedLeb128Reader::<_, Strict>::with_capacity(Cursor::new(vec![0x80, 0x00, 0x01]), 2);

    assert_eq!(
        ErrorKind::InvalidData,
        reader.read_u16().expect_err("non-canonical value should fail").kind()
    );
    assert_eq!(1, reader.read_u8().expect("next value should remain readable"));

    let mut reader = BufferedLeb128Reader::<_, NonStrict>::with_capacity(Cursor::new(vec![0x80, 0x02, 0x01]), 2);
    assert_eq!(
        ErrorKind::InvalidData,
        reader
            .read_u8()
            .expect_err("out-of-range u8 encoding should fail")
            .kind()
    );
    assert_eq!(1, reader.read_u8().expect("next value should remain readable"));
}
