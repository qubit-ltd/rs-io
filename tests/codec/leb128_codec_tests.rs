use qubit_io::{
    Leb128Codec,
    Leb128DecodeErrorKind,
    NonStrict,
    Strict,
};

#[test]
fn test_leb128_codec_exposes_required_min_buffer_len() {
    assert_eq!(2, Leb128Codec::<u8, NonStrict>::REQUIRED_MIN_BUFFER_LEN);
    assert_eq!(3, Leb128Codec::<u16, NonStrict>::REQUIRED_MIN_BUFFER_LEN);
    assert_eq!(5, Leb128Codec::<u32, NonStrict>::REQUIRED_MIN_BUFFER_LEN);
    assert_eq!(10, Leb128Codec::<u64, NonStrict>::REQUIRED_MIN_BUFFER_LEN);
    assert_eq!(19, Leb128Codec::<u128, NonStrict>::REQUIRED_MIN_BUFFER_LEN);
    assert_eq!(
        (usize::BITS as usize).div_ceil(7),
        Leb128Codec::<usize, NonStrict>::REQUIRED_MIN_BUFFER_LEN
    );
    assert_eq!(2, Leb128Codec::<i8, Strict>::REQUIRED_MIN_BUFFER_LEN);
    assert_eq!(3, Leb128Codec::<i16, Strict>::REQUIRED_MIN_BUFFER_LEN);
}

#[test]
fn test_leb128_codec_reads_and_writes_unsigned_values_unchecked() {
    let mut output = [0u8; Leb128Codec::<u16, NonStrict>::REQUIRED_MIN_BUFFER_LEN + 2];
    let len = unsafe { Leb128Codec::<u16, NonStrict>::write_unchecked(&mut output, 1, 300) };

    assert_eq!(2, len);
    assert_eq!([0x00, 0xac, 0x02, 0x00, 0x00], output);

    let decoded =
        unsafe { Leb128Codec::<u16, NonStrict>::read_unchecked(&output, 1) }.expect("valid u16 should decode");
    assert_eq!((300, 2), decoded);

    let mut output = [0u8; Leb128Codec::<u16, NonStrict>::REQUIRED_MIN_BUFFER_LEN];
    let len = unsafe { Leb128Codec::<u16, NonStrict>::write_unchecked(&mut output, 0, u16::MAX) };
    let decoded = unsafe { Leb128Codec::<u16, NonStrict>::read_unchecked(&output, 0) }.expect("u16::MAX should decode");
    assert_eq!((u16::MAX, len), decoded);
}

#[test]
fn test_leb128_codec_reads_and_writes_signed_values_unchecked() {
    let mut output = [0u8; Leb128Codec::<i16, NonStrict>::REQUIRED_MIN_BUFFER_LEN + 2];
    let len = unsafe { Leb128Codec::<i16, NonStrict>::write_unchecked(&mut output, 1, -300) };

    assert_eq!(2, len);
    assert_eq!([0x00, 0xd4, 0x7d, 0x00, 0x00], output);

    let decoded =
        unsafe { Leb128Codec::<i16, NonStrict>::read_unchecked(&output, 1) }.expect("valid i16 should decode");
    assert_eq!((-300, 2), decoded);

    let mut output = [0u8; Leb128Codec::<i16, NonStrict>::REQUIRED_MIN_BUFFER_LEN];
    let len = unsafe { Leb128Codec::<i16, NonStrict>::write_unchecked(&mut output, 0, 300) };
    let decoded =
        unsafe { Leb128Codec::<i16, NonStrict>::read_unchecked(&output, 0) }.expect("positive i16 should decode");
    assert_eq!((300, len), decoded);

    let mut output = [0u8; Leb128Codec::<i128, NonStrict>::REQUIRED_MIN_BUFFER_LEN];
    let len = unsafe { Leb128Codec::<i128, NonStrict>::write_unchecked(&mut output, 0, i128::MIN) };
    let decoded =
        unsafe { Leb128Codec::<i128, NonStrict>::read_unchecked(&output, 0) }.expect("i128::MIN should decode");
    assert_eq!((i128::MIN, len), decoded);

    let values: [i16; 8] = [0, -1, 63, 64, -64, -65, i16::MIN, i16::MAX];
    let mut output = [0u8; Leb128Codec::<i16, NonStrict>::REQUIRED_MIN_BUFFER_LEN];
    for value in values {
        output.fill(0);
        let len = unsafe { Leb128Codec::<i16, NonStrict>::write_unchecked(&mut output, 0, value) };
        let decoded = unsafe { Leb128Codec::<i16, NonStrict>::read_unchecked(&output, 0) }
            .expect("signed boundary value should decode");
        assert_eq!((value, len), decoded);
    }
}

#[test]
fn test_leb128_codec_roundtrips_all_strict_and_non_strict_instantiations() {
    macro_rules! roundtrip_unsigned {
        ($ty:ty, $value:expr) => {{
            let value = $value as $ty;

            let mut output = [0u8; Leb128Codec::<$ty, NonStrict>::REQUIRED_MIN_BUFFER_LEN];
            let len = unsafe { Leb128Codec::<$ty, NonStrict>::write_unchecked(&mut output, 0, value) };
            let decoded = unsafe { Leb128Codec::<$ty, NonStrict>::read_unchecked(&output, 0) }
                .expect("non-strict unsigned value should decode");
            assert_eq!((value, len), decoded);

            let mut output = [0u8; Leb128Codec::<$ty, Strict>::REQUIRED_MIN_BUFFER_LEN];
            let len = unsafe { Leb128Codec::<$ty, Strict>::write_unchecked(&mut output, 0, value) };
            let decoded = unsafe { Leb128Codec::<$ty, Strict>::read_unchecked(&output, 0) }
                .expect("strict unsigned value should decode");
            assert_eq!((value, len), decoded);
        }};
    }

    macro_rules! roundtrip_signed {
        ($ty:ty, $value:expr) => {{
            let value = $value as $ty;

            let mut output = [0u8; Leb128Codec::<$ty, NonStrict>::REQUIRED_MIN_BUFFER_LEN];
            let len = unsafe { Leb128Codec::<$ty, NonStrict>::write_unchecked(&mut output, 0, value) };
            let decoded = unsafe { Leb128Codec::<$ty, NonStrict>::read_unchecked(&output, 0) }
                .expect("non-strict signed value should decode");
            assert_eq!((value, len), decoded);

            let mut output = [0u8; Leb128Codec::<$ty, Strict>::REQUIRED_MIN_BUFFER_LEN];
            let len = unsafe { Leb128Codec::<$ty, Strict>::write_unchecked(&mut output, 0, value) };
            let decoded = unsafe { Leb128Codec::<$ty, Strict>::read_unchecked(&output, 0) }
                .expect("strict signed value should decode");
            assert_eq!((value, len), decoded);
        }};
    }

    roundtrip_unsigned!(u8, u8::MAX);
    roundtrip_unsigned!(u16, u16::MAX);
    roundtrip_unsigned!(u32, u32::MAX);
    roundtrip_unsigned!(u64, u64::MAX);
    roundtrip_unsigned!(u128, u128::MAX);
    roundtrip_unsigned!(usize, usize::MAX);

    roundtrip_signed!(i8, i8::MIN);
    roundtrip_signed!(i16, i16::MIN);
    roundtrip_signed!(i32, i32::MIN);
    roundtrip_signed!(i64, i64::MIN);
    roundtrip_signed!(i128, i128::MIN);
    roundtrip_signed!(isize, isize::MIN);
}

#[test]
fn test_leb128_codec_rejects_all_instantiated_error_paths() {
    macro_rules! reject_unsigned {
        ($ty:ty) => {{
            let unterminated = [0x80u8; Leb128Codec::<$ty, NonStrict>::REQUIRED_MIN_BUFFER_LEN];
            let error = unsafe { Leb128Codec::<$ty, NonStrict>::read_unchecked(&unterminated, 0) }
                .expect_err("unterminated unsigned value should fail");
            assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());

            let error = unsafe { Leb128Codec::<$ty, Strict>::read_unchecked(&unterminated, 0) }
                .expect_err("unterminated strict unsigned value should fail");
            assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());

            let max_bytes = Leb128Codec::<$ty, NonStrict>::REQUIRED_MIN_BUFFER_LEN;
            let bits = <$ty>::BITS as usize;
            let used_bits = bits - (max_bytes - 1) * 7;
            let mut malformed = [0x80u8; Leb128Codec::<$ty, NonStrict>::REQUIRED_MIN_BUFFER_LEN];
            malformed[max_bytes - 1] = 1u8 << used_bits;
            let error = unsafe { Leb128Codec::<$ty, NonStrict>::read_unchecked(&malformed, 0) }
                .expect_err("too-wide unsigned payload should fail");
            assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());

            let mut noncanonical = [0u8; Leb128Codec::<$ty, Strict>::REQUIRED_MIN_BUFFER_LEN];
            noncanonical[0] = 0x80;
            let error = unsafe { Leb128Codec::<$ty, Strict>::read_unchecked(&noncanonical, 0) }
                .expect_err("non-canonical unsigned value should fail");
            assert_eq!(Leb128DecodeErrorKind::NonCanonical, error.kind());
        }};
    }

    macro_rules! reject_signed {
        ($ty:ty) => {{
            let unterminated = [0x80u8; Leb128Codec::<$ty, NonStrict>::REQUIRED_MIN_BUFFER_LEN];
            let error = unsafe { Leb128Codec::<$ty, NonStrict>::read_unchecked(&unterminated, 0) }
                .expect_err("unterminated signed value should fail");
            assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());

            let error = unsafe { Leb128Codec::<$ty, Strict>::read_unchecked(&unterminated, 0) }
                .expect_err("unterminated strict signed value should fail");
            assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());

            let max_bytes = Leb128Codec::<$ty, NonStrict>::REQUIRED_MIN_BUFFER_LEN;
            let bits = <$ty>::BITS as usize;
            let used_bits = bits - (max_bytes - 1) * 7;

            let mut malformed = [0x80u8; Leb128Codec::<$ty, NonStrict>::REQUIRED_MIN_BUFFER_LEN];
            malformed[max_bytes - 1] = 1u8 << used_bits;
            let error = unsafe { Leb128Codec::<$ty, NonStrict>::read_unchecked(&malformed, 0) }
                .expect_err("too-wide positive signed payload should fail");
            assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());

            let mut malformed = [0x80u8; Leb128Codec::<$ty, NonStrict>::REQUIRED_MIN_BUFFER_LEN];
            malformed[max_bytes - 1] = 1u8 << (used_bits - 1);
            let error = unsafe { Leb128Codec::<$ty, NonStrict>::read_unchecked(&malformed, 0) }
                .expect_err("too-narrow negative signed payload should fail");
            assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());

            let mut noncanonical = [0u8; Leb128Codec::<$ty, Strict>::REQUIRED_MIN_BUFFER_LEN];
            noncanonical[0] = 0xff;
            noncanonical[1] = 0x7f;
            let error = unsafe { Leb128Codec::<$ty, Strict>::read_unchecked(&noncanonical, 0) }
                .expect_err("non-canonical signed value should fail");
            assert_eq!(Leb128DecodeErrorKind::NonCanonical, error.kind());
        }};
    }

    reject_unsigned!(u8);
    reject_unsigned!(u16);
    reject_unsigned!(u32);
    reject_unsigned!(u64);
    reject_unsigned!(u128);
    reject_unsigned!(usize);

    reject_signed!(i8);
    reject_signed!(i16);
    reject_signed!(i32);
    reject_signed!(i64);
    reject_signed!(i128);
    reject_signed!(isize);
}

#[test]
fn test_leb128_codec_rejects_malformed_values() {
    let error = unsafe { Leb128Codec::<u16, NonStrict>::read_unchecked(&[0x80, 0x80, 0x04], 0) }
        .expect_err("too-wide unsigned payload should fail");
    assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());
    assert_eq!(2, error.index());

    let error = unsafe { Leb128Codec::<u16, NonStrict>::read_unchecked(&[0x80, 0x80, 0x80], 0) }
        .expect_err("unterminated unsigned payload should fail");
    assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());
    assert_eq!(2, error.index());

    let error = unsafe { Leb128Codec::<i16, NonStrict>::read_unchecked(&[0x80, 0x80, 0x04], 0) }
        .expect_err("too-wide signed payload should fail");
    assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());
    assert_eq!(2, error.index());

    let error = unsafe { Leb128Codec::<i16, NonStrict>::read_unchecked(&[0x80, 0x80, 0x02], 0) }
        .expect_err("too-narrow negative signed payload should fail");
    assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());
    assert_eq!(2, error.index());

    let error = unsafe { Leb128Codec::<i16, NonStrict>::read_unchecked(&[0x80, 0x80, 0x80], 0) }
        .expect_err("unterminated signed payload should fail");
    assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());
    assert_eq!(2, error.index());
}

#[test]
fn test_leb128_codec_rejects_noncanonical_strict_values() {
    let decoded = unsafe { Leb128Codec::<u16, Strict>::read_unchecked(&[0xac, 0x02, 0x00], 0) }
        .expect("canonical unsigned value should decode");
    assert_eq!((300, 2), decoded);

    let decoded = unsafe { Leb128Codec::<i16, Strict>::read_unchecked(&[0xd4, 0x7d, 0x00], 0) }
        .expect("canonical signed value should decode");
    assert_eq!((-300, 2), decoded);

    let decoded = unsafe { Leb128Codec::<i16, Strict>::read_unchecked(&[0xac, 0x02, 0x00], 0) }
        .expect("canonical positive signed value should decode");
    assert_eq!((300, 2), decoded);

    let error = unsafe { Leb128Codec::<u16, Strict>::read_unchecked(&[0x80, 0x00, 0x00], 0) }
        .expect_err("non-canonical unsigned value should fail");
    assert_eq!(Leb128DecodeErrorKind::NonCanonical, error.kind());
    assert_eq!(0, error.index());

    let error = unsafe { Leb128Codec::<i16, Strict>::read_unchecked(&[0xff, 0x7f, 0x00], 0) }
        .expect_err("non-canonical signed value should fail");
    assert_eq!(Leb128DecodeErrorKind::NonCanonical, error.kind());
    assert_eq!(0, error.index());
}
