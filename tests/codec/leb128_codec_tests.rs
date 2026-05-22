use qubit_io::{
    Leb128Codec,
    Leb128DecodeError,
    Leb128DecodeErrorKind,
};

#[test]
fn test_new_and_strict_configuration_are_exposed() {
    let mut codec = Leb128Codec::new();

    assert!(!codec.strict());
    assert!(!Leb128Codec::default().strict());

    codec.set_strict(true);

    assert!(codec.strict());
    assert!(Leb128Codec::with_strict(true).strict());
}

#[test]
fn test_unsigned_leb128_round_trips_fixed_width_values() {
    let codec = Leb128Codec::new();
    let mut output = [0_u8; 64];
    let mut offset = 0;

    offset += codec
        .write_u16_at(&mut output, offset, 300)
        .expect("u16 should fit");
    offset += codec
        .write_u32_at(&mut output, offset, 0x1f600)
        .expect("u32 should fit");
    offset += codec
        .write_u64_at(&mut output, offset, 0x0102_0304_0506_0708)
        .expect("u64 should fit");
    offset += codec
        .write_u128_at(
            &mut output,
            offset,
            0x0102_0304_0506_0708_1112_1314_1516_1718,
        )
        .expect("u128 should fit");

    let mut index = 0;
    let (value, consumed) = codec
        .read_u16_at(&output[..offset], index)
        .expect("valid u16")
        .expect("complete u16");
    assert_eq!(300, value);
    index += consumed;

    let (value, consumed) = codec
        .read_u32_at(&output[..offset], index)
        .expect("valid u32")
        .expect("complete u32");
    assert_eq!(0x1f600, value);
    index += consumed;

    let (value, consumed) = codec
        .read_u64_at(&output[..offset], index)
        .expect("valid u64")
        .expect("complete u64");
    assert_eq!(0x0102_0304_0506_0708, value);
    index += consumed;

    let (value, consumed) = codec
        .read_u128_at(&output[..offset], index)
        .expect("valid u128")
        .expect("complete u128");
    assert_eq!(0x0102_0304_0506_0708_1112_1314_1516_1718, value);
    index += consumed;

    assert_eq!(offset, index);
}

#[test]
fn test_signed_leb128_round_trips_fixed_width_values() {
    let codec = Leb128Codec::new();
    let mut output = [0_u8; 64];
    let mut offset = 0;

    offset += codec
        .write_i16_at(&mut output, offset, -300)
        .expect("i16 should fit");
    offset += codec
        .write_i32_at(&mut output, offset, -0x1f600)
        .expect("i32 should fit");
    offset += codec
        .write_i64_at(&mut output, offset, -0x0102_0304_0506_0708)
        .expect("i64 should fit");
    offset += codec
        .write_i128_at(
            &mut output,
            offset,
            -0x0102_0304_0506_0708_1112_1314_1516_1718,
        )
        .expect("i128 should fit");

    let mut index = 0;
    let (value, consumed) = codec
        .read_i16_at(&output[..offset], index)
        .expect("valid i16")
        .expect("complete i16");
    assert_eq!(-300, value);
    index += consumed;

    let (value, consumed) = codec
        .read_i32_at(&output[..offset], index)
        .expect("valid i32")
        .expect("complete i32");
    assert_eq!(-0x1f600, value);
    index += consumed;

    let (value, consumed) = codec
        .read_i64_at(&output[..offset], index)
        .expect("valid i64")
        .expect("complete i64");
    assert_eq!(-0x0102_0304_0506_0708, value);
    index += consumed;

    let (value, consumed) = codec
        .read_i128_at(&output[..offset], index)
        .expect("valid i128")
        .expect("complete i128");
    assert_eq!(-0x0102_0304_0506_0708_1112_1314_1516_1718, value);
    index += consumed;

    assert_eq!(offset, index);
}

#[test]
fn test_read_returns_none_for_incomplete_values() {
    let codec = Leb128Codec::new();

    assert_eq!(None, codec.read_u16_at(&[], 0).expect("empty input"));
    assert_eq!(
        None,
        codec
            .read_u16_at(&[0x80], 0)
            .expect("truncated unsigned input"),
    );
    assert_eq!(
        None,
        codec
            .read_i16_at(&[0x80], 0)
            .expect("truncated signed input"),
    );
}

#[test]
fn test_read_rejects_out_of_range_and_noncanonical_values() {
    let codec = Leb128Codec::new();
    let error = codec
        .read_u16_at(&[0], 2)
        .expect_err("out-of-range index should fail");
    assert_eq!(2, error.index());
    assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());
    assert_eq!("malformed LEB128 integer", error.to_string());

    let error = codec
        .read_i16_at(&[0], 2)
        .expect_err("out-of-range signed index should fail");
    assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());

    let error = codec
        .read_u16_at(&[0x80, 0x80, 0x80, 0x80, 0x00], 0)
        .expect_err("too-wide u16 should fail");
    assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());
    let error = codec
        .read_u16_at(&[0x80, 0x80, 0x04], 0)
        .expect_err("too-wide final unsigned payload should fail");
    assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());
    let error = codec
        .read_i16_at(&[0x80, 0x80, 0x04], 0)
        .expect_err("too-wide final signed payload should fail");
    assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());
    let error = codec
        .read_i16_at(&[0x80, 0x80, 0x80], 0)
        .expect_err("unterminated signed value should fail");
    assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());

    let strict = Leb128Codec::with_strict(true);
    let error = strict
        .read_u16_at(&[0x80, 0x00], 0)
        .expect_err("non-canonical unsigned value should fail");
    assert_eq!(Leb128DecodeErrorKind::NonCanonical, error.kind());

    let error = strict
        .read_i16_at(&[0xff, 0x7f], 0)
        .expect_err("non-canonical signed value should fail");
    assert_eq!(Leb128DecodeErrorKind::NonCanonical, error.kind());
    assert_eq!(
        Leb128DecodeError::new(Leb128DecodeErrorKind::NonCanonical, 0),
        error,
    );
}

#[test]
fn test_write_returns_none_when_output_is_too_small() {
    let codec = Leb128Codec::new();
    let mut output = [0_u8; 1];

    assert_eq!(None, codec.write_u16_at(&mut output, 0, 300));
    assert_eq!(None, codec.write_i16_at(&mut output, 0, -300));
    assert_eq!(None, codec.write_u128_at(&mut output, usize::MAX, 0));
}

#[test]
fn test_read_from_array_decodes_max_width_arrays() {
    let codec = Leb128Codec::new();

    let (value, consumed) = codec
        .read_u16_from_array([0xac, 0x02, 0x00])
        .expect("u16 array should decode");
    assert_eq!(300, value);
    assert_eq!(2, consumed);

    let (value, consumed) = codec
        .read_i16_from_array([0xd4, 0x7d, 0x00])
        .expect("i16 array should decode");
    assert_eq!(-300, value);
    assert_eq!(2, consumed);

    let (bytes, expected_count) = codec.u32_bytes(0x1f600);
    let (value, consumed) = codec
        .read_u32_from_array(bytes)
        .expect("u32 array should decode");
    assert_eq!(0x1f600, value);
    assert_eq!(expected_count, consumed);

    let (bytes, expected_count) = codec.u64_bytes(0x0102_0304_0506_0708);
    let (value, consumed) = codec
        .read_u64_from_array(bytes)
        .expect("u64 array should decode");
    assert_eq!(0x0102_0304_0506_0708, value);
    assert_eq!(expected_count, consumed);

    let source = 0x0102_0304_0506_0708_1112_1314_1516_1718;
    let (bytes, expected_count) = codec.u128_bytes(source);
    let (value, consumed) = codec
        .read_u128_from_array(bytes)
        .expect("u128 array should decode");
    assert_eq!(source, value);
    assert_eq!(expected_count, consumed);

    let (bytes, expected_count) = codec.i32_bytes(-0x1f600);
    let (value, consumed) = codec
        .read_i32_from_array(bytes)
        .expect("i32 array should decode");
    assert_eq!(-0x1f600, value);
    assert_eq!(expected_count, consumed);

    let (bytes, expected_count) = codec.i64_bytes(-0x0102_0304_0506_0708);
    let (value, consumed) = codec
        .read_i64_from_array(bytes)
        .expect("i64 array should decode");
    assert_eq!(-0x0102_0304_0506_0708, value);
    assert_eq!(expected_count, consumed);

    let source = -0x0102_0304_0506_0708_1112_1314_1516_1718;
    let (bytes, expected_count) = codec.i128_bytes(source);
    let (value, consumed) = codec
        .read_i128_from_array(bytes)
        .expect("i128 array should decode");
    assert_eq!(source, value);
    assert_eq!(expected_count, consumed);
}

#[test]
fn test_to_bytes_returns_max_width_array_and_used_length() {
    let codec = Leb128Codec::new();

    let (bytes, count) = codec.u16_bytes(300);
    assert_eq!([0xac, 0x02, 0x00], bytes);
    assert_eq!(2, count);

    let (bytes, count) = codec.i16_bytes(-300);
    assert_eq!([0xd4, 0x7d, 0x00], bytes);
    assert_eq!(2, count);

    let (_, count) = codec.u32_bytes(0x1f600);
    assert!(count <= 5);
    let (_, count) = codec.u64_bytes(0x0102_0304_0506_0708);
    assert!(count <= 10);
    let (_, count) = codec.u128_bytes(0x0102_0304_0506_0708_1112_1314_1516_1718);
    assert!(count <= 19);
    let (_, count) = codec.i32_bytes(-0x1f600);
    assert!(count <= 5);
    let (_, count) = codec.i64_bytes(-0x0102_0304_0506_0708);
    assert!(count <= 10);
    let (_, count) = codec.i128_bytes(-0x0102_0304_0506_0708_1112_1314_1516_1718);
    assert!(count <= 19);
}

#[test]
fn test_unchecked_access_uses_caller_validated_ranges() {
    let codec = Leb128Codec::new();
    let input = [0x00, 0xac, 0x02, 0x00];
    let mut output = [0_u8; 4];

    let (value, consumed) = unsafe {
        // SAFETY: The input has the full three-byte u16 LEB128 range at index 1.
        codec
            .read_u16_at_unchecked(&input, 1)
            .expect("unchecked u16 should decode")
    };
    assert_eq!(300, value);
    assert_eq!(2, consumed);

    let consumed = unsafe {
        // SAFETY: The output has the full three-byte u16 LEB128 range at index 1.
        codec.write_u16_at_unchecked(&mut output, 1, 300)
    };
    assert_eq!(2, consumed);
    assert_eq!([0x00, 0xac, 0x02, 0x00], output);

    let (bytes, expected_count) = codec.u32_bytes(0x1f600);
    let mut input = vec![0_u8; bytes.len() + 1];
    input[1..1 + expected_count].copy_from_slice(&bytes[..expected_count]);
    let (value, consumed) = unsafe {
        // SAFETY: The input has the full five-byte u32 LEB128 range at index 1.
        codec
            .read_u32_at_unchecked(&input, 1)
            .expect("unchecked u32 should decode")
    };
    assert_eq!(0x1f600, value);
    assert_eq!(expected_count, consumed);
    let mut output = vec![0_u8; bytes.len() + 1];
    let consumed = unsafe {
        // SAFETY: The output has the full five-byte u32 LEB128 range at index 1.
        codec.write_u32_at_unchecked(&mut output, 1, 0x1f600)
    };
    assert_eq!(expected_count, consumed);
    assert_eq!(&bytes[..expected_count], &output[1..1 + consumed]);

    let (bytes, expected_count) = codec.u64_bytes(0x0102_0304_0506_0708);
    let mut input = vec![0_u8; bytes.len() + 1];
    input[1..1 + expected_count].copy_from_slice(&bytes[..expected_count]);
    let (value, consumed) = unsafe {
        // SAFETY: The input has the full ten-byte u64 LEB128 range at index 1.
        codec
            .read_u64_at_unchecked(&input, 1)
            .expect("unchecked u64 should decode")
    };
    assert_eq!(0x0102_0304_0506_0708, value);
    assert_eq!(expected_count, consumed);
    let mut output = vec![0_u8; bytes.len() + 1];
    let consumed = unsafe {
        // SAFETY: The output has the full ten-byte u64 LEB128 range at index 1.
        codec.write_u64_at_unchecked(&mut output, 1, 0x0102_0304_0506_0708)
    };
    assert_eq!(expected_count, consumed);
    assert_eq!(&bytes[..expected_count], &output[1..1 + consumed]);

    let source = 0x0102_0304_0506_0708_1112_1314_1516_1718;
    let (bytes, expected_count) = codec.u128_bytes(source);
    let mut input = vec![0_u8; bytes.len() + 1];
    input[1..1 + expected_count].copy_from_slice(&bytes[..expected_count]);
    let (value, consumed) = unsafe {
        // SAFETY: The input has the full nineteen-byte u128 LEB128 range at index 1.
        codec
            .read_u128_at_unchecked(&input, 1)
            .expect("unchecked u128 should decode")
    };
    assert_eq!(source, value);
    assert_eq!(expected_count, consumed);
    let mut output = vec![0_u8; bytes.len() + 1];
    let consumed = unsafe {
        // SAFETY: The output has the full nineteen-byte u128 LEB128 range at index 1.
        codec.write_u128_at_unchecked(&mut output, 1, source)
    };
    assert_eq!(expected_count, consumed);
    assert_eq!(&bytes[..expected_count], &output[1..1 + consumed]);

    let (bytes, expected_count) = codec.i16_bytes(-300);
    let mut input = vec![0_u8; bytes.len() + 1];
    input[1..1 + expected_count].copy_from_slice(&bytes[..expected_count]);
    let (value, consumed) = unsafe {
        // SAFETY: The input has the full three-byte i16 LEB128 range at index 1.
        codec
            .read_i16_at_unchecked(&input, 1)
            .expect("unchecked i16 should decode")
    };
    assert_eq!(-300, value);
    assert_eq!(expected_count, consumed);
    let mut output = vec![0_u8; bytes.len() + 1];
    let consumed = unsafe {
        // SAFETY: The output has the full three-byte i16 LEB128 range at index 1.
        codec.write_i16_at_unchecked(&mut output, 1, -300)
    };
    assert_eq!(expected_count, consumed);
    assert_eq!(&bytes[..expected_count], &output[1..1 + consumed]);

    let (bytes, expected_count) = codec.i32_bytes(-0x1f600);
    let mut input = vec![0_u8; bytes.len() + 1];
    input[1..1 + expected_count].copy_from_slice(&bytes[..expected_count]);
    let (value, consumed) = unsafe {
        // SAFETY: The input has the full five-byte i32 LEB128 range at index 1.
        codec
            .read_i32_at_unchecked(&input, 1)
            .expect("unchecked i32 should decode")
    };
    assert_eq!(-0x1f600, value);
    assert_eq!(expected_count, consumed);
    let mut output = vec![0_u8; bytes.len() + 1];
    let consumed = unsafe {
        // SAFETY: The output has the full five-byte i32 LEB128 range at index 1.
        codec.write_i32_at_unchecked(&mut output, 1, -0x1f600)
    };
    assert_eq!(expected_count, consumed);
    assert_eq!(&bytes[..expected_count], &output[1..1 + consumed]);

    let (bytes, expected_count) = codec.i64_bytes(-0x0102_0304_0506_0708);
    let mut input = vec![0_u8; bytes.len() + 1];
    input[1..1 + expected_count].copy_from_slice(&bytes[..expected_count]);
    let (value, consumed) = unsafe {
        // SAFETY: The input has the full ten-byte i64 LEB128 range at index 1.
        codec
            .read_i64_at_unchecked(&input, 1)
            .expect("unchecked i64 should decode")
    };
    assert_eq!(-0x0102_0304_0506_0708, value);
    assert_eq!(expected_count, consumed);
    let mut output = vec![0_u8; bytes.len() + 1];
    let consumed = unsafe {
        // SAFETY: The output has the full ten-byte i64 LEB128 range at index 1.
        codec.write_i64_at_unchecked(&mut output, 1, -0x0102_0304_0506_0708)
    };
    assert_eq!(expected_count, consumed);
    assert_eq!(&bytes[..expected_count], &output[1..1 + consumed]);

    let source = -0x0102_0304_0506_0708_1112_1314_1516_1718;
    let (bytes, expected_count) = codec.i128_bytes(source);
    let mut input = vec![0_u8; bytes.len() + 1];
    input[1..1 + expected_count].copy_from_slice(&bytes[..expected_count]);
    let (value, consumed) = unsafe {
        // SAFETY: The input has the full nineteen-byte i128 LEB128 range at index 1.
        codec
            .read_i128_at_unchecked(&input, 1)
            .expect("unchecked i128 should decode")
    };
    assert_eq!(source, value);
    assert_eq!(expected_count, consumed);
    let mut output = vec![0_u8; bytes.len() + 1];
    let consumed = unsafe {
        // SAFETY: The output has the full nineteen-byte i128 LEB128 range at index 1.
        codec.write_i128_at_unchecked(&mut output, 1, source)
    };
    assert_eq!(expected_count, consumed);
    assert_eq!(&bytes[..expected_count], &output[1..1 + consumed]);
}

#[test]
fn test_unchecked_read_reports_malformed_and_noncanonical_values() {
    let codec = Leb128Codec::new();

    let error = unsafe {
        // SAFETY: The input has the full three-byte u16 LEB128 range at index 0.
        codec.read_u16_at_unchecked(&[0x80, 0x80, 0x04], 0)
    }
    .expect_err("too-wide final unsigned payload should fail");
    assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());

    let error = unsafe {
        // SAFETY: The input has the full three-byte u16 LEB128 range at index 0.
        codec.read_u16_at_unchecked(&[0x80, 0x80, 0x80], 0)
    }
    .expect_err("unterminated unsigned value should fail");
    assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());

    let error = unsafe {
        // SAFETY: The input has the full three-byte i16 LEB128 range at index 0.
        codec.read_i16_at_unchecked(&[0x80, 0x80, 0x04], 0)
    }
    .expect_err("too-wide positive signed payload should fail");
    assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());

    let error = unsafe {
        // SAFETY: The input has the full three-byte i16 LEB128 range at index 0.
        codec.read_i16_at_unchecked(&[0x80, 0x80, 0x02], 0)
    }
    .expect_err("too-wide negative signed payload should fail");
    assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());

    let error = unsafe {
        // SAFETY: The input has the full three-byte i16 LEB128 range at index 0.
        codec.read_i16_at_unchecked(&[0x80, 0x80, 0x80], 0)
    }
    .expect_err("unterminated signed value should fail");
    assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());

    let strict = Leb128Codec::with_strict(true);
    let error = unsafe {
        // SAFETY: The input has the full three-byte u16 LEB128 range at index 0.
        strict.read_u16_at_unchecked(&[0x80, 0x00, 0x00], 0)
    }
    .expect_err("non-canonical unchecked unsigned value should fail");
    assert_eq!(Leb128DecodeErrorKind::NonCanonical, error.kind());

    let error = unsafe {
        // SAFETY: The input has the full three-byte i16 LEB128 range at index 0.
        strict.read_i16_at_unchecked(&[0xff, 0x7f, 0x00], 0)
    }
    .expect_err("non-canonical unchecked signed value should fail");
    assert_eq!(Leb128DecodeErrorKind::NonCanonical, error.kind());
}
