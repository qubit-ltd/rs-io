use qubit_io::{
    Leb128DecodeErrorKind,
    ZigZagCodec,
};

#[test]
fn test_new_and_strict_configuration_are_exposed() {
    let mut codec = ZigZagCodec::new();

    assert!(!codec.strict());
    assert!(!ZigZagCodec::default().strict());

    codec.set_strict(true);

    assert!(codec.strict());
    assert!(ZigZagCodec::with_strict(true).strict());
}

#[test]
fn test_mapping_round_trips_fixed_width_values() {
    assert_eq!(0_u16, ZigZagCodec::encode_i16(0));
    assert_eq!(1_u16, ZigZagCodec::encode_i16(-1));
    assert_eq!(2_u16, ZigZagCodec::encode_i16(1));
    assert_eq!(i16::MIN, ZigZagCodec::decode_u16(u16::MAX));

    assert_eq!(0_u32, ZigZagCodec::encode_i32(0));
    assert_eq!(1_u32, ZigZagCodec::encode_i32(-1));
    assert_eq!(2_u32, ZigZagCodec::encode_i32(1));
    assert_eq!(i32::MIN, ZigZagCodec::decode_u32(u32::MAX));

    assert_eq!(0_u64, ZigZagCodec::encode_i64(0));
    assert_eq!(1_u64, ZigZagCodec::encode_i64(-1));
    assert_eq!(2_u64, ZigZagCodec::encode_i64(1));
    assert_eq!(i64::MIN, ZigZagCodec::decode_u64(u64::MAX));

    assert_eq!(0_u128, ZigZagCodec::encode_i128(0));
    assert_eq!(1_u128, ZigZagCodec::encode_i128(-1));
    assert_eq!(2_u128, ZigZagCodec::encode_i128(1));
    assert_eq!(i128::MIN, ZigZagCodec::decode_u128(u128::MAX));
}

#[test]
fn test_zig_zag_round_trips_buffer_values() {
    let codec = ZigZagCodec::new();
    let mut output = [0_u8; 64];
    let mut offset = 0;

    offset += codec
        .write_i16_at(&mut output, offset, -300)
        .expect("i16 should fit");
    offset += codec
        .write_i32_at(&mut output, offset, -0x1f600)
        .expect("i32 should fit");
    offset += codec
        .write_i64_at(&mut output, offset, i64::MIN)
        .expect("i64 should fit");
    offset += codec
        .write_i128_at(&mut output, offset, i128::MIN)
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
    assert_eq!(i64::MIN, value);
    index += consumed;

    let (value, consumed) = codec
        .read_i128_at(&output[..offset], index)
        .expect("valid i128")
        .expect("complete i128");
    assert_eq!(i128::MIN, value);
    index += consumed;

    assert_eq!(offset, index);
}

#[test]
fn test_zig_zag_reports_incomplete_output_and_strict_read_errors() {
    let mut output = [0_u8; 1];
    let codec = ZigZagCodec::new();

    assert_eq!(None, codec.write_i16_at(&mut output, 0, -300));
    assert_eq!(
        None,
        codec
            .read_i16_at(&[0x80], 0)
            .expect("truncated ZigZag input"),
    );

    let strict = ZigZagCodec::with_strict(true);
    let error = strict
        .read_i16_at(&[0x80, 0x00], 0)
        .expect_err("non-canonical ZigZag payload should fail");
    assert_eq!(Leb128DecodeErrorKind::NonCanonical, error.kind());
}

#[test]
fn test_read_from_array_decodes_max_width_arrays() {
    let codec = ZigZagCodec::new();

    let (value, consumed) = codec
        .read_i16_from_array([0xd7, 0x04, 0x00])
        .expect("i16 array should decode");
    assert_eq!(-300, value);
    assert_eq!(2, consumed);

    let (bytes, expected_count) = codec.i32_bytes(-0x1f600);
    let (value, consumed) = codec
        .read_i32_from_array(bytes)
        .expect("i32 array should decode");
    assert_eq!(-0x1f600, value);
    assert_eq!(expected_count, consumed);

    let (bytes, expected_count) = codec.i64_bytes(i64::MIN);
    let (value, consumed) = codec
        .read_i64_from_array(bytes)
        .expect("i64 array should decode");
    assert_eq!(i64::MIN, value);
    assert_eq!(expected_count, consumed);

    let (bytes, expected_count) = codec.i128_bytes(i128::MIN);
    let (value, consumed) = codec
        .read_i128_from_array(bytes)
        .expect("i128 array should decode");
    assert_eq!(i128::MIN, value);
    assert_eq!(expected_count, consumed);
}

#[test]
fn test_to_bytes_returns_max_width_array_and_used_length() {
    let codec = ZigZagCodec::new();

    let (bytes, count) = codec.i16_bytes(-300);
    assert_eq!([0xd7, 0x04, 0x00], bytes);
    assert_eq!(2, count);

    let (_, count) = codec.i32_bytes(-0x1f600);
    assert!(count <= 5);
    let (_, count) = codec.i64_bytes(i64::MIN);
    assert!(count <= 10);
    let (_, count) = codec.i128_bytes(i128::MIN);
    assert!(count <= 19);
}

#[test]
fn test_unchecked_access_uses_caller_validated_ranges() {
    let codec = ZigZagCodec::new();
    let input = [0x00, 0xd7, 0x04, 0x00];
    let mut output = [0_u8; 4];

    let (value, consumed) = unsafe {
        // SAFETY: The input has the full three-byte i16 ZigZag range at index 1.
        codec
            .read_i16_at_unchecked(&input, 1)
            .expect("unchecked i16 should decode")
    };
    assert_eq!(-300, value);
    assert_eq!(2, consumed);

    let consumed = unsafe {
        // SAFETY: The output has the full three-byte i16 ZigZag range at index 1.
        codec.write_i16_at_unchecked(&mut output, 1, -300)
    };
    assert_eq!(2, consumed);
    assert_eq!([0x00, 0xd7, 0x04, 0x00], output);

    let (bytes, expected_count) = codec.i32_bytes(-0x1f600);
    let mut input = vec![0_u8; bytes.len() + 1];
    input[1..1 + expected_count].copy_from_slice(&bytes[..expected_count]);
    let (value, consumed) = unsafe {
        // SAFETY: The input has the full five-byte i32 ZigZag range at index 1.
        codec
            .read_i32_at_unchecked(&input, 1)
            .expect("unchecked i32 should decode")
    };
    assert_eq!(-0x1f600, value);
    assert_eq!(expected_count, consumed);
    let mut output = vec![0_u8; bytes.len() + 1];
    let consumed = unsafe {
        // SAFETY: The output has the full five-byte i32 ZigZag range at index 1.
        codec.write_i32_at_unchecked(&mut output, 1, -0x1f600)
    };
    assert_eq!(expected_count, consumed);
    assert_eq!(&bytes[..expected_count], &output[1..1 + consumed]);

    let (bytes, expected_count) = codec.i64_bytes(i64::MIN);
    let mut input = vec![0_u8; bytes.len() + 1];
    input[1..1 + expected_count].copy_from_slice(&bytes[..expected_count]);
    let (value, consumed) = unsafe {
        // SAFETY: The input has the full ten-byte i64 ZigZag range at index 1.
        codec
            .read_i64_at_unchecked(&input, 1)
            .expect("unchecked i64 should decode")
    };
    assert_eq!(i64::MIN, value);
    assert_eq!(expected_count, consumed);
    let mut output = vec![0_u8; bytes.len() + 1];
    let consumed = unsafe {
        // SAFETY: The output has the full ten-byte i64 ZigZag range at index 1.
        codec.write_i64_at_unchecked(&mut output, 1, i64::MIN)
    };
    assert_eq!(expected_count, consumed);
    assert_eq!(&bytes[..expected_count], &output[1..1 + consumed]);

    let (bytes, expected_count) = codec.i128_bytes(i128::MIN);
    let mut input = vec![0_u8; bytes.len() + 1];
    input[1..1 + expected_count].copy_from_slice(&bytes[..expected_count]);
    let (value, consumed) = unsafe {
        // SAFETY: The input has the full nineteen-byte i128 ZigZag range at index 1.
        codec
            .read_i128_at_unchecked(&input, 1)
            .expect("unchecked i128 should decode")
    };
    assert_eq!(i128::MIN, value);
    assert_eq!(expected_count, consumed);
    let mut output = vec![0_u8; bytes.len() + 1];
    let consumed = unsafe {
        // SAFETY: The output has the full nineteen-byte i128 ZigZag range at index 1.
        codec.write_i128_at_unchecked(&mut output, 1, i128::MIN)
    };
    assert_eq!(expected_count, consumed);
    assert_eq!(&bytes[..expected_count], &output[1..1 + consumed]);
}
