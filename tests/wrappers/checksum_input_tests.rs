//! Tests for [`qubit_io::ChecksumInput`].

use std::{
    collections::hash_map::DefaultHasher,
    hash::Hasher,
    io::Cursor,
};

use qubit_io::{
    ChecksumInput,
    Input,
};

#[test]
fn test_checksum_input_hashes_successful_byte_prefix() {
    let mut expected = DefaultHasher::new();
    expected.write(b"ab");
    let mut input =
        ChecksumInput::new(Cursor::new(b"abc".to_vec()), DefaultHasher::new());
    let mut output = [0_u8; 2];

    assert_eq!(2, input.read(&mut output).expect("read should succeed"));
    assert_eq!(expected.finish(), input.checksum());
}
