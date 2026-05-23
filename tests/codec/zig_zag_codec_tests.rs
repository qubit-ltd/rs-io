use qubit_io::{
    Leb128DecodeErrorKind,
    NonStrict,
    Strict,
    ZigZagCodec,
};

#[test]
fn test_zig_zag_codec_exposes_required_min_buffer_len() {
    assert_eq!(2, ZigZagCodec::<i8, NonStrict>::REQUIRED_MIN_BUFFER_LEN);
    assert_eq!(3, ZigZagCodec::<i16, NonStrict>::REQUIRED_MIN_BUFFER_LEN);
    assert_eq!(5, ZigZagCodec::<i32, NonStrict>::REQUIRED_MIN_BUFFER_LEN);
    assert_eq!(10, ZigZagCodec::<i64, NonStrict>::REQUIRED_MIN_BUFFER_LEN);
    assert_eq!(19, ZigZagCodec::<i128, NonStrict>::REQUIRED_MIN_BUFFER_LEN);
    assert_eq!(
        (isize::BITS as usize).div_ceil(7),
        ZigZagCodec::<isize, Strict>::REQUIRED_MIN_BUFFER_LEN
    );
}

#[test]
fn test_zig_zag_codec_reads_and_writes_values_unchecked() {
    let mut output = [0u8; ZigZagCodec::<i16, NonStrict>::REQUIRED_MIN_BUFFER_LEN + 2];
    let len = unsafe { ZigZagCodec::<i16, NonStrict>::write_unchecked(&mut output, 1, -300) };

    assert_eq!(2, len);
    assert_eq!([0x00, 0xd7, 0x04, 0x00, 0x00], output);

    let decoded =
        unsafe { ZigZagCodec::<i16, NonStrict>::read_unchecked(&output, 1) }.expect("valid i16 should decode");
    assert_eq!((-300, 2), decoded);
}

#[test]
fn test_zig_zag_codec_handles_signed_extremes() {
    let mut output = [0u8; ZigZagCodec::<i128, NonStrict>::REQUIRED_MIN_BUFFER_LEN];
    let len = unsafe { ZigZagCodec::<i128, NonStrict>::write_unchecked(&mut output, 0, i128::MIN) };

    let decoded =
        unsafe { ZigZagCodec::<i128, NonStrict>::read_unchecked(&output, 0) }.expect("valid i128 should decode");
    assert_eq!((i128::MIN, len), decoded);
}

#[test]
fn test_zig_zag_codec_rejects_noncanonical_strict_values() {
    let error = unsafe { ZigZagCodec::<i16, Strict>::read_unchecked(&[0x80, 0x00, 0x00], 0) }
        .expect_err("non-canonical value should fail");

    assert_eq!(Leb128DecodeErrorKind::NonCanonical, error.kind());
    assert_eq!(0, error.index());
}
