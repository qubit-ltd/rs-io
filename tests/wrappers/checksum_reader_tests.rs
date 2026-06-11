use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::io::{Cursor, Error, ErrorKind, Read, Seek, SeekFrom};

use qubit_io::ChecksumReader;

struct FailingReader;

impl Read for FailingReader {
    fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
        Err(Error::other("read failed"))
    }
}

fn expected_checksum(bytes: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(bytes);
    hasher.finish()
}

#[test]
fn test_checksum_reader_hashes_successfully_read_bytes_and_exposes_accessors() {
    let source = Cursor::new(b"abcdef".to_vec());
    let mut reader = ChecksumReader::new(source, DefaultHasher::new());
    assert_eq!(expected_checksum(b""), reader.checksum());
    assert_eq!(0, reader.get_ref().position());

    reader.get_mut().set_position(1);
    reader.hasher_mut().write(b"x");
    let mut buffer = [0; 2];
    let count = reader
        .read(&mut buffer)
        .expect("checksum read should succeed");

    assert_eq!(2, count);
    assert_eq!(b"bc", &buffer);
    assert_eq!(expected_checksum(b"xbc"), reader.checksum());
    assert_eq!(expected_checksum(b"xbc"), reader.hasher_ref().finish());

    let (source, hasher) = reader.into_inner();
    assert_eq!(3, source.position());
    assert_eq!(expected_checksum(b"xbc"), hasher.finish());
}

#[test]
fn test_checksum_reader_does_not_hash_failed_reads() {
    let mut reader = ChecksumReader::new(FailingReader, DefaultHasher::new());
    let mut buffer = [0; 2];

    let error = reader
        .read(&mut buffer)
        .expect_err("read error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!(expected_checksum(b""), reader.checksum());
}

#[test]
fn test_checksum_reader_forwards_seek() {
    let source = Cursor::new(b"abcdef".to_vec());
    let mut reader = ChecksumReader::new(source, DefaultHasher::new());

    reader
        .seek(SeekFrom::Start(2))
        .expect("seek should be forwarded");
    let mut buffer = [0; 2];
    reader.read_exact(&mut buffer).expect("read should succeed");

    assert_eq!(b"cd", &buffer);
    assert_eq!(expected_checksum(b"cd"), reader.checksum());
}
