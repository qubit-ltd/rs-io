use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::io::{Cursor, Error, ErrorKind, Seek, SeekFrom, Write};

use qubit_io::ChecksumWriter;

struct ShortWriter {
    data: Vec<u8>,
    max_chunk: usize,
}

impl ShortWriter {
    fn new(max_chunk: usize) -> Self {
        Self {
            data: Vec::new(),
            max_chunk,
        }
    }
}

impl Write for ShortWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let count = buffer.len().min(self.max_chunk);
        self.data.extend_from_slice(&buffer[..count]);
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _buffer: &[u8]) -> std::io::Result<usize> {
        Err(Error::other("write failed"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Err(Error::other("flush failed"))
    }
}

fn expected_checksum(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(bytes);
    hasher.finish()
}

#[test]
fn test_checksum_writer_hashes_successfully_written_bytes_and_exposes_accessors() {
    let primary = ShortWriter::new(3);
    let mut writer = ChecksumWriter::new(primary, DefaultHasher::new());
    assert_eq!(expected_checksum(b""), writer.checksum());

    writer.get_mut().data.extend_from_slice(b"x");
    writer.hasher_mut().write(b"y");
    let count = writer
        .write(b"abcdef")
        .expect("checksum write should succeed");
    writer.flush().expect("flush should succeed");

    assert_eq!(3, count);
    assert_eq!(b"xabc", writer.get_ref().data.as_slice());
    assert_eq!(expected_checksum(b"yabc"), writer.checksum());
    assert_eq!(expected_checksum(b"yabc"), writer.hasher_ref().finish());

    let (primary, hasher) = writer.into_inner();
    assert_eq!(b"xabc", primary.data.as_slice());
    assert_eq!(expected_checksum(b"yabc"), hasher.finish());
}

#[test]
fn test_checksum_writer_does_not_hash_failed_writes_and_flush_errors() {
    let mut writer = ChecksumWriter::new(FailingWriter, DefaultHasher::new());

    let error = writer
        .write(b"abc")
        .expect_err("write error should be returned");
    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!(expected_checksum(b""), writer.checksum());

    let error = writer.flush().expect_err("flush error should be returned");
    assert_eq!(ErrorKind::Other, error.kind());
}

#[test]
fn test_checksum_writer_forwards_seek() {
    let primary = Cursor::new(Vec::new());
    let mut writer = ChecksumWriter::new(primary, DefaultHasher::new());

    writer.write_all(b"abc").expect("write should succeed");
    writer
        .seek(SeekFrom::Start(1))
        .expect("seek should be forwarded");
    writer.write_all(b"z").expect("write should succeed");

    let (primary, hasher) = writer.into_inner();
    assert_eq!(b"azc", primary.into_inner().as_slice());
    assert_eq!(expected_checksum(b"abcz"), hasher.finish());
}
