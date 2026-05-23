use qubit_io::{
    BigEndian,
    BinaryCodec,
    LittleEndian,
};

#[test]
fn test_binary_codec_exposes_required_min_buffer_len() {
    assert_eq!(1, BinaryCodec::<u8, BigEndian>::REQUIRED_MIN_BUFFER_LEN);
    assert_eq!(1, BinaryCodec::<i8, LittleEndian>::REQUIRED_MIN_BUFFER_LEN);
    assert_eq!(2, BinaryCodec::<u16, BigEndian>::REQUIRED_MIN_BUFFER_LEN);
    assert_eq!(4, BinaryCodec::<u32, LittleEndian>::REQUIRED_MIN_BUFFER_LEN);
    assert_eq!(8, BinaryCodec::<u64, BigEndian>::REQUIRED_MIN_BUFFER_LEN);
    assert_eq!(16, BinaryCodec::<u128, LittleEndian>::REQUIRED_MIN_BUFFER_LEN);
    assert_eq!(4, BinaryCodec::<f32, BigEndian>::REQUIRED_MIN_BUFFER_LEN);
    assert_eq!(8, BinaryCodec::<f64, LittleEndian>::REQUIRED_MIN_BUFFER_LEN);
}

#[test]
fn test_binary_codec_reads_from_explicit_index_unchecked() {
    let input = [0xaa, 0x12, 0x34, 0x56, 0x78, 0xbb];

    let value = unsafe { BinaryCodec::<u32, BigEndian>::read_unchecked(&input, 1) };
    assert_eq!(0x1234_5678, value);

    let value = unsafe { BinaryCodec::<u32, LittleEndian>::read_unchecked(&input, 1) };
    assert_eq!(0x7856_3412, value);
}

#[test]
fn test_binary_codec_writes_to_explicit_index_unchecked() {
    let mut output = [0xaa, 0, 0, 0, 0, 0xbb];

    unsafe {
        BinaryCodec::<u32, BigEndian>::write_unchecked(&mut output, 1, 0x1234_5678);
    }
    assert_eq!([0xaa, 0x12, 0x34, 0x56, 0x78, 0xbb], output);

    unsafe {
        BinaryCodec::<u32, LittleEndian>::write_unchecked(&mut output, 1, 0x1234_5678);
    }
    assert_eq!([0xaa, 0x78, 0x56, 0x34, 0x12, 0xbb], output);
}

#[test]
fn test_binary_codec_handles_byte_signed_and_float_values() {
    let mut output = [0u8; 16];

    unsafe {
        BinaryCodec::<u8, BigEndian>::write_unchecked(&mut output, 0, 0x7f);
        BinaryCodec::<i8, LittleEndian>::write_unchecked(&mut output, 1, -1);
        BinaryCodec::<f32, BigEndian>::write_unchecked(&mut output, 2, 12.5);
        BinaryCodec::<f64, LittleEndian>::write_unchecked(&mut output, 6, -25.25);
    }

    assert_eq!(0x7f, unsafe {
        BinaryCodec::<u8, LittleEndian>::read_unchecked(&output, 0)
    });
    assert_eq!(-1, unsafe { BinaryCodec::<i8, BigEndian>::read_unchecked(&output, 1) });
    assert_eq!(12.5, unsafe {
        BinaryCodec::<f32, BigEndian>::read_unchecked(&output, 2)
    });
    assert_eq!(-25.25, unsafe {
        BinaryCodec::<f64, LittleEndian>::read_unchecked(&output, 6)
    });
}
