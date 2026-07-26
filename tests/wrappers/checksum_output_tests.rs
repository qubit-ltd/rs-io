//! Tests for [`qubit_io::ChecksumOutput`].

use std::{
    collections::hash_map::DefaultHasher,
    hash::Hasher,
};

use qubit_io::{
    ChecksumOutput,
    Output,
};

#[test]
fn test_checksum_output_hashes_successful_byte_prefix() {
    let mut expected = DefaultHasher::new();
    expected.write(b"ab");
    let mut output = ChecksumOutput::new(Vec::new(), DefaultHasher::new());

    assert_eq!(2, output.write(b"ab").expect("write should succeed"));
    assert_eq!(expected.finish(), output.checksum());
}
