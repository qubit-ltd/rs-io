// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Cursor;

use qubit_io::BufferedInput;
use qubit_io::BufferedOutput;
use qubit_io::ChecksumInput;
use qubit_io::ChecksumOutput;
use qubit_io::CountingInput;
use qubit_io::CountingOutput;
use qubit_io::Input;
use qubit_io::LimitInput;
use qubit_io::LimitOutput;
use qubit_io::Output;
use qubit_io::TeeInput;
use qubit_io::TeeOutput;

fn read_two<I: Input<Item = u8>>(input: &mut I) {
    let mut first = [0; 2];
    let mut second = [b'!'; 2];
    assert_eq!(Input::read_vectored(input, &mut [&mut first, &mut second]).unwrap(), 2);
    assert_eq!(&first, b"ab");
    assert_eq!(&second, b"!!");
}

fn write_two<O: Output<Item = u8>>(output: &mut O) {
    assert_eq!(Output::write_vectored(output, &[b"ab", b"cd"]).unwrap(), 2);
}

#[test]
fn vectored_default_keeps_limit_count_buffering_and_tee_semantics() {
    let mut input = CountingInput::new(LimitInput::new(Cursor::new(b"abcdef"), 3));
    let mut first = [0; 4];
    let mut second = [b'!'; 2];
    assert_eq!(
        Input::read_vectored(&mut input, &mut [&mut first, &mut second]).unwrap(),
        3
    );
    assert_eq!(&first, b"abc\0");
    assert_eq!(&second, b"!!");
    assert_eq!(input.items_read(), 3);
    assert_eq!(input.inner().remaining(), 0);
    let mut output = CountingOutput::new(LimitOutput::new(Vec::<u8>::new(), 3));
    assert_eq!(Output::write_vectored(&mut output, &[b"abcd", b"ef"]).unwrap(), 3);
    assert_eq!(output.items_written(), 3);
    assert_eq!(output.inner().remaining(), 0);
    assert_eq!(output.inner().inner(), b"abc");

    let mut input = BufferedInput::with_capacity(Cursor::new(b"abcd"), 4);
    read_two(&mut input);
    let mut suffix = [0; 2];
    Input::read_exactly(&mut input, &mut suffix).unwrap();
    assert_eq!(&suffix, b"cd");
    let mut output = BufferedOutput::with_capacity(Vec::<u8>::new(), 4);
    write_two(&mut output);
    Output::flush(&mut output).unwrap();
    assert_eq!(output.inner(), b"ab");

    let mut input = TeeInput::new(Cursor::new(b"abcd"), Vec::<u8>::new());
    read_two(&mut input);
    assert_eq!(input.branch(), b"ab");
    let mut output = TeeOutput::new(Vec::<u8>::new(), Vec::<u8>::new());
    write_two(&mut output);
    assert_eq!(output.inner(), b"ab");
    assert_eq!(output.branch(), b"ab");
}

#[test]
fn vectored_default_updates_checksum_for_transferred_prefix_only() {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::Hasher;

    let mut expected = DefaultHasher::new();
    expected.write(b"ab");
    let expected = expected.finish();
    let mut input = ChecksumInput::new(Cursor::new(b"abcd"), DefaultHasher::new());
    read_two(&mut input);
    assert_eq!(input.checksum(), expected);
    let mut output = ChecksumOutput::new(Vec::<u8>::new(), DefaultHasher::new());
    write_two(&mut output);
    assert_eq!(output.checksum(), expected);
    assert_eq!(output.inner(), b"ab");
}

#[cfg(feature = "digest")]
#[test]
fn vectored_default_updates_digest_for_transferred_prefix_only() {
    use qubit_io::DigestInput;
    use qubit_io::DigestOutput;
    use sha2::Digest;
    use sha2::Sha256;

    let expected = Sha256::digest(b"ab");
    let mut input = DigestInput::new(Cursor::new(b"abcd"), Sha256::new());
    read_two(&mut input);
    assert_eq!(input.digest(), expected);
    let mut output = DigestOutput::new(Vec::<u8>::new(), Sha256::new());
    write_two(&mut output);
    assert_eq!(output.digest(), expected);
    assert_eq!(output.inner(), b"ab");
}
