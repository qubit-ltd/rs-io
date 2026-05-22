use qubit_io::{
    BinaryCodec,
    ByteOrder,
};
use std::hint::black_box;

#[test]
fn test_new_stores_byte_order() {
    let mut codec = BinaryCodec::new(ByteOrder::BigEndian);

    assert_eq!(ByteOrder::BigEndian, codec.byte_order());

    codec.set_byte_order(ByteOrder::LittleEndian);

    assert_eq!(ByteOrder::LittleEndian, codec.byte_order());
}

#[test]
fn test_read_from_array_decodes_fixed_width_values() {
    assert_eq!(
        0x1234,
        BinaryCodec::new(ByteOrder::BigEndian).read_u16_from_array([0x12, 0x34]),
    );
    assert_eq!(
        0x1234,
        BinaryCodec::new(ByteOrder::LittleEndian).read_u16_from_array([0x34, 0x12]),
    );
    assert_eq!(
        0x0001f600,
        BinaryCodec::new(ByteOrder::BigEndian).read_u32_from_array([0x00, 0x01, 0xf6, 0x00]),
    );
    assert_eq!(
        0x0001f600,
        BinaryCodec::new(ByteOrder::LittleEndian).read_u32_from_array([0x00, 0xf6, 0x01, 0x00]),
    );
    assert_eq!(
        0x0102_0304_0506_0708,
        BinaryCodec::new(ByteOrder::BigEndian).read_u64_from_array([1, 2, 3, 4, 5, 6, 7, 8]),
    );
    assert_eq!(
        0x0102_0304_0506_0708,
        BinaryCodec::new(ByteOrder::LittleEndian).read_u64_from_array([8, 7, 6, 5, 4, 3, 2, 1]),
    );
    assert_eq!(
        -0x1234,
        BinaryCodec::new(ByteOrder::BigEndian).read_i16_from_array([0xed, 0xcc]),
    );
    assert_eq!(
        -0x1234,
        BinaryCodec::new(ByteOrder::LittleEndian).read_i16_from_array([0xcc, 0xed]),
    );
    assert_eq!(
        -0x0001f600,
        BinaryCodec::new(ByteOrder::LittleEndian).read_i32_from_array([0x00, 0x0a, 0xfe, 0xff]),
    );
    assert_eq!(
        -0x0102_0304_0506_0708,
        BinaryCodec::new(ByteOrder::BigEndian)
            .read_i64_from_array([0xfe, 0xfd, 0xfc, 0xfb, 0xfa, 0xf9, 0xf8, 0xf8]),
    );
    assert_eq!(
        -0x0102_0304_0506_0708,
        BinaryCodec::new(ByteOrder::LittleEndian)
            .read_i64_from_array([0xf8, 0xf8, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd, 0xfe]),
    );
    assert_eq!(
        0x0102_0304_0506_0708_1112_1314_1516_1718,
        BinaryCodec::new(ByteOrder::BigEndian).read_u128_from_array([
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16,
            0x17, 0x18,
        ]),
    );
    assert_eq!(
        0x0102_0304_0506_0708_1112_1314_1516_1718,
        BinaryCodec::new(ByteOrder::LittleEndian).read_u128_from_array([
            0x18, 0x17, 0x16, 0x15, 0x14, 0x13, 0x12, 0x11, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03,
            0x02, 0x01,
        ]),
    );
    assert_eq!(
        -0x0102_0304_0506_0708_1112_1314_1516_1718,
        BinaryCodec::new(ByteOrder::LittleEndian).read_i128_from_array([
            0xe8, 0xe8, 0xe9, 0xea, 0xeb, 0xec, 0xed, 0xee, 0xf7, 0xf8, 0xf9, 0xfa, 0xfb, 0xfc,
            0xfd, 0xfe,
        ]),
    );
}

#[test]
fn test_read_at_returns_none_when_range_is_not_available() {
    let codec = BinaryCodec::new(ByteOrder::BigEndian);
    let bytes = [0xaa, 0x12, 0x34, 0x00, 0x01, 0xf6, 0x00];
    let bytes64 = [0xaa, 1, 2, 3, 4, 5, 6, 7, 8];

    assert_eq!(Some(0x1234), codec.read_u16_at(&bytes, 1));
    assert_eq!(Some(-0x1234), codec.read_i16_at(&[0xaa, 0xed, 0xcc], 1));
    assert_eq!(Some(0x0001f600), codec.read_u32_at(&bytes, 3));
    assert_eq!(
        Some(-0x0001f600),
        codec.read_i32_at(&[0xaa, 0xff, 0xfe, 0x0a, 0x00], 1),
    );
    assert_eq!(Some(0x0102_0304_0506_0708), codec.read_u64_at(&bytes64, 1),);
    assert_eq!(
        Some(-0x0102_0304_0506_0708),
        codec.read_i64_at(&[0xaa, 0xfe, 0xfd, 0xfc, 0xfb, 0xfa, 0xf9, 0xf8, 0xf8], 1),
    );
    assert_eq!(
        Some(0x0102_0304_0506_0708_1112_1314_1516_1718),
        codec.read_u128_at(
            &[
                0xaa, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x11, 0x12, 0x13, 0x14, 0x15,
                0x16, 0x17, 0x18,
            ],
            1,
        ),
    );
    assert_eq!(
        Some(-0x0102_0304_0506_0708_1112_1314_1516_1718),
        codec.read_i128_at(
            &[
                0xaa, 0xfe, 0xfd, 0xfc, 0xfb, 0xfa, 0xf9, 0xf8, 0xf7, 0xee, 0xed, 0xec, 0xeb, 0xea,
                0xe9, 0xe8, 0xe8,
            ],
            1,
        ),
    );
    assert_eq!(None, codec.read_u16_at(&bytes, 6));
    assert_eq!(None, codec.read_i16_at(&bytes, 6));
    assert_eq!(None, codec.read_u32_at(&bytes, 4));
    assert_eq!(None, codec.read_i32_at(&bytes, 4));
    assert_eq!(None, codec.read_u64_at(&bytes64, 2));
    assert_eq!(None, codec.read_i64_at(&bytes64, 2));
    assert_eq!(None, codec.read_u128_at(&bytes64, 0));
    assert_eq!(None, codec.read_i128_at(&bytes64, 0));
    assert_eq!(None, codec.read_u16_at(&bytes, usize::MAX));
    assert_eq!(None, codec.read_u32_at(&bytes, usize::MAX));
    assert_eq!(None, codec.read_u64_at(&bytes64, usize::MAX));
    assert_eq!(None, codec.read_u128_at(&bytes64, usize::MAX));
}

#[test]
fn test_read_at_unchecked_reads_after_caller_validates_bounds() {
    let bytes = [0xaa, 0x12, 0x34, 0x00, 0x01, 0xf6, 0x00, 0xbb];
    let bytes64 = [0xaa, 8, 7, 6, 5, 4, 3, 2, 1];

    assert!(2 < bytes.len());
    assert!(3 + 4 <= bytes.len());
    assert!(8 < bytes64.len());
    let bytes128 = [
        0xaa, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16,
        0x17, 0x18,
    ];
    assert!(16 < bytes128.len());
    // SAFETY: The assertions above prove the requested byte ranges are in bounds.
    unsafe {
        assert_eq!(
            0x1234,
            BinaryCodec::new(ByteOrder::BigEndian).read_u16_at_unchecked(&bytes, 1)
        );
        assert_eq!(
            0x0001f600,
            BinaryCodec::new(ByteOrder::BigEndian).read_u32_at_unchecked(&bytes, 3),
        );
        assert_eq!(
            0x0102_0304_0506_0708,
            BinaryCodec::new(ByteOrder::LittleEndian).read_u64_at_unchecked(&bytes64, 1),
        );
        assert_eq!(
            -0x1234,
            BinaryCodec::new(ByteOrder::BigEndian).read_i16_at_unchecked(&[0xaa, 0xed, 0xcc], 1),
        );
        assert_eq!(
            -0x0001f600,
            BinaryCodec::new(ByteOrder::BigEndian)
                .read_i32_at_unchecked(&[0xaa, 0xff, 0xfe, 0x0a, 0x00], 1),
        );
        assert_eq!(
            -0x0102_0304_0506_0708,
            BinaryCodec::new(ByteOrder::BigEndian)
                .read_i64_at_unchecked(&[0xaa, 0xfe, 0xfd, 0xfc, 0xfb, 0xfa, 0xf9, 0xf8, 0xf8], 1,),
        );
        assert_eq!(
            0x0102_0304_0506_0708_1112_1314_1516_1718,
            BinaryCodec::new(ByteOrder::BigEndian).read_u128_at_unchecked(&bytes128, 1),
        );
    }
}

#[test]
fn test_to_bytes_encodes_fixed_width_values() {
    assert_eq!(
        [0x12, 0x34],
        BinaryCodec::new(ByteOrder::BigEndian).u16_bytes(0x1234),
    );
    assert_eq!(
        [0x34, 0x12],
        BinaryCodec::new(ByteOrder::LittleEndian).u16_bytes(0x1234),
    );
    assert_eq!(
        [0x00, 0x01, 0xf6, 0x00],
        BinaryCodec::new(ByteOrder::BigEndian).u32_bytes(0x0001f600),
    );
    assert_eq!(
        [0x00, 0xf6, 0x01, 0x00],
        BinaryCodec::new(ByteOrder::LittleEndian).u32_bytes(0x0001f600),
    );
    assert_eq!(
        [1, 2, 3, 4, 5, 6, 7, 8],
        BinaryCodec::new(ByteOrder::BigEndian).u64_bytes(0x0102_0304_0506_0708),
    );
    assert_eq!(
        [8, 7, 6, 5, 4, 3, 2, 1],
        BinaryCodec::new(ByteOrder::LittleEndian).u64_bytes(0x0102_0304_0506_0708),
    );
    assert_eq!(
        [0xed, 0xcc],
        BinaryCodec::new(ByteOrder::BigEndian).i16_bytes(-0x1234),
    );
    assert_eq!(
        [0xcc, 0xed],
        BinaryCodec::new(ByteOrder::LittleEndian).i16_bytes(-0x1234),
    );
    assert_eq!(
        [0x00, 0x0a, 0xfe, 0xff],
        BinaryCodec::new(ByteOrder::LittleEndian).i32_bytes(-0x0001f600),
    );
    assert_eq!(
        [0xfe, 0xfd, 0xfc, 0xfb, 0xfa, 0xf9, 0xf8, 0xf8],
        BinaryCodec::new(ByteOrder::BigEndian).i64_bytes(-0x0102_0304_0506_0708),
    );
    assert_eq!(
        [0xf8, 0xf8, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd, 0xfe],
        BinaryCodec::new(ByteOrder::LittleEndian).i64_bytes(-0x0102_0304_0506_0708),
    );
    assert_eq!(
        [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16,
            0x17, 0x18,
        ],
        BinaryCodec::new(ByteOrder::BigEndian)
            .u128_bytes(0x0102_0304_0506_0708_1112_1314_1516_1718),
    );
    assert_eq!(
        [
            0x18, 0x17, 0x16, 0x15, 0x14, 0x13, 0x12, 0x11, 0x08, 0x07, 0x06, 0x05, 0x04, 0x03,
            0x02, 0x01,
        ],
        BinaryCodec::new(ByteOrder::LittleEndian)
            .u128_bytes(0x0102_0304_0506_0708_1112_1314_1516_1718),
    );
    assert_eq!(
        [
            0xfe, 0xfd, 0xfc, 0xfb, 0xfa, 0xf9, 0xf8, 0xf7, 0xee, 0xed, 0xec, 0xeb, 0xea, 0xe9,
            0xe8, 0xe8,
        ],
        BinaryCodec::new(ByteOrder::BigEndian)
            .i128_bytes(-0x0102_0304_0506_0708_1112_1314_1516_1718),
    );
    assert_eq!(
        [
            0xe8, 0xe8, 0xe9, 0xea, 0xeb, 0xec, 0xed, 0xee, 0xf7, 0xf8, 0xf9, 0xfa, 0xfb, 0xfc,
            0xfd, 0xfe,
        ],
        BinaryCodec::new(ByteOrder::LittleEndian)
            .i128_bytes(-0x0102_0304_0506_0708_1112_1314_1516_1718),
    );
}

#[test]
fn test_write_at_returns_none_when_range_is_not_available() {
    let codec = BinaryCodec::new(ByteOrder::BigEndian);
    let mut bytes = [0_u8; 8];
    let mut bytes64 = [0_u8; 9];

    assert_eq!(Some(()), codec.write_u16_at(&mut bytes, 1, 0x1234));
    assert_eq!(Some(()), codec.write_i16_at(&mut bytes, 1, -0x1234));
    assert_eq!(Some(()), codec.write_u32_at(&mut bytes, 3, 0x0001f600));
    assert_eq!(Some(()), codec.write_i32_at(&mut bytes, 3, -0x0001f600));
    assert_eq!(None, codec.write_u16_at(&mut bytes, 7, 0xabcd));
    assert_eq!(None, codec.write_i16_at(&mut bytes, 7, -1));
    assert_eq!(None, codec.write_u16_at(&mut bytes, usize::MAX, 0xabcd));
    assert_eq!(None, codec.write_u32_at(&mut bytes, 6, 0));
    assert_eq!(None, codec.write_i32_at(&mut bytes, 6, 0));
    assert_eq!(None, codec.write_u32_at(&mut bytes, usize::MAX, 0));
    assert_eq!([0, 0xed, 0xcc, 0xff, 0xfe, 0x0a, 0x00, 0], bytes,);
    assert_eq!(
        Some(()),
        codec.write_u64_at(&mut bytes64, 1, 0x0102_0304_0506_0708),
    );
    assert_eq!(
        Some(()),
        codec.write_i64_at(&mut bytes64, 1, -0x0102_0304_0506_0708),
    );
    assert_eq!(
        None,
        codec.write_u64_at(&mut bytes64, 2, 0x0102_0304_0506_0708),
    );
    assert_eq!(None, codec.write_i64_at(&mut bytes64, 2, -1));
    assert_eq!(None, codec.write_u64_at(&mut bytes64, usize::MAX, 0));
    assert_eq!([0, 0xfe, 0xfd, 0xfc, 0xfb, 0xfa, 0xf9, 0xf8, 0xf8], bytes64);

    let mut bytes128 = [0_u8; 17];
    assert_eq!(
        Some(()),
        codec.write_u128_at(&mut bytes128, 1, 0x0102_0304_0506_0708_1112_1314_1516_1718,),
    );
    assert_eq!(
        None,
        codec.write_i128_at(&mut bytes128, 2, -0x0102_0304_0506_0708_1112_1314_1516_1718,),
    );
    assert_eq!(
        [
            0, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16,
            0x17, 0x18,
        ],
        bytes128,
    );
    assert_eq!(
        Some(()),
        codec.write_i128_at(&mut bytes128, 1, -0x0102_0304_0506_0708_1112_1314_1516_1718,),
    );
    assert_eq!(
        [
            0, 0xfe, 0xfd, 0xfc, 0xfb, 0xfa, 0xf9, 0xf8, 0xf7, 0xee, 0xed, 0xec, 0xeb, 0xea, 0xe9,
            0xe8, 0xe8,
        ],
        bytes128,
    );
}

#[test]
fn test_write_at_unchecked_writes_after_caller_validates_bounds() {
    let mut bytes = [0_u8; 8];
    let mut bytes64 = [0_u8; 8];

    assert!(8 <= bytes.len());
    assert!(8 <= bytes64.len());
    // SAFETY: The assertions above prove the requested byte ranges are in bounds.
    unsafe {
        BinaryCodec::new(ByteOrder::BigEndian).write_u16_at_unchecked(&mut bytes, 0, 0x1234);
        BinaryCodec::new(ByteOrder::BigEndian).write_u32_at_unchecked(&mut bytes, 2, 0x0001f600);
        BinaryCodec::new(ByteOrder::LittleEndian).write_u64_at_unchecked(
            &mut bytes64,
            0,
            0x0102_0304_0506_0708,
        );
        BinaryCodec::new(ByteOrder::BigEndian).write_i16_at_unchecked(&mut bytes, 0, -0x1234);
        BinaryCodec::new(ByteOrder::BigEndian).write_i32_at_unchecked(&mut bytes, 2, -0x0001f600);
    }

    assert_eq!([0xed, 0xcc, 0xff, 0xfe, 0x0a, 0x00, 0, 0], bytes);
    assert_eq!([8, 7, 6, 5, 4, 3, 2, 1], bytes64);

    let mut bytes128 = [0_u8; 16];
    assert!(16 <= bytes128.len());
    // SAFETY: The assertion above proves the requested byte range is in bounds.
    unsafe {
        BinaryCodec::new(ByteOrder::BigEndian).write_u128_at_unchecked(
            &mut bytes128,
            0,
            0x0102_0304_0506_0708_1112_1314_1516_1718,
        );
    }
    assert_eq!(
        [
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16,
            0x17, 0x18,
        ],
        bytes128,
    );

    let mut signed128 = [0_u8; 16];
    assert!(16 <= signed128.len());
    // SAFETY: The assertion above proves the requested byte range is in bounds.
    unsafe {
        BinaryCodec::new(ByteOrder::BigEndian).write_i128_at_unchecked(
            &mut signed128,
            0,
            -0x0102_0304_0506_0708_1112_1314_1516_1718,
        );
        assert_eq!(
            -0x0102_0304_0506_0708_1112_1314_1516_1718,
            BinaryCodec::new(ByteOrder::BigEndian).read_i128_at_unchecked(&signed128, 0),
        );
        assert_eq!(
            -0x1234,
            BinaryCodec::new(ByteOrder::BigEndian).read_i16_at_unchecked(&[0xed, 0xcc], 0),
        );
        BinaryCodec::new(ByteOrder::BigEndian).write_i64_at_unchecked(
            &mut bytes64,
            0,
            -0x0102_0304_0506_0708,
        );
    }
    assert_eq!(
        [
            0xfe, 0xfd, 0xfc, 0xfb, 0xfa, 0xf9, 0xf8, 0xf7, 0xee, 0xed, 0xec, 0xeb, 0xea, 0xe9,
            0xe8, 0xe8,
        ],
        signed128,
    );
}

#[test]
fn test_u64_methods_are_callable_through_function_pointers() {
    let read_from_array: fn(BinaryCodec, [u8; 8]) -> u64 = BinaryCodec::read_u64_from_array;
    let read_at: fn(BinaryCodec, &[u8], usize) -> Option<u64> = BinaryCodec::read_u64_at;
    let to_bytes: fn(BinaryCodec, u64) -> [u8; 8] = BinaryCodec::u64_bytes;
    let write_at: fn(BinaryCodec, &mut [u8], usize, u64) -> Option<()> = BinaryCodec::write_u64_at;
    let read_unchecked: unsafe fn(BinaryCodec, &[u8], usize) -> u64 =
        BinaryCodec::read_u64_at_unchecked;
    let write_unchecked: unsafe fn(BinaryCodec, &mut [u8], usize, u64) =
        BinaryCodec::write_u64_at_unchecked;
    let read_i16_unchecked: unsafe fn(BinaryCodec, &[u8], usize) -> i16 =
        BinaryCodec::read_i16_at_unchecked;
    let write_i16_unchecked: unsafe fn(BinaryCodec, &mut [u8], usize, i16) =
        BinaryCodec::write_i16_at_unchecked;
    let bytes = black_box([0xaa, 1, 2, 3, 4, 5, 6, 7, 8]);
    let signed_bytes = black_box([0xaa, 0xed, 0xcc]);
    let mut output = black_box([0_u8; 8]);
    let mut signed_output = black_box([0_u8; 2]);

    assert_eq!(
        0x0102_0304_0506_0708,
        read_from_array(
            BinaryCodec::new(ByteOrder::BigEndian),
            black_box([1, 2, 3, 4, 5, 6, 7, 8]),
        ),
    );
    assert_eq!(
        Some(0x0102_0304_0506_0708),
        read_at(BinaryCodec::new(ByteOrder::BigEndian), &bytes, 1),
    );
    assert_eq!(
        [8, 7, 6, 5, 4, 3, 2, 1],
        to_bytes(
            BinaryCodec::new(ByteOrder::LittleEndian),
            0x0102_0304_0506_0708,
        ),
    );
    assert_eq!(
        Some(()),
        write_at(
            BinaryCodec::new(ByteOrder::LittleEndian),
            &mut output,
            0,
            0x0102_0304_0506_0708,
        ),
    );
    // SAFETY: Both calls use ranges that are fully contained in their buffers.
    unsafe {
        assert_eq!(
            0x0102_0304_0506_0708,
            read_unchecked(BinaryCodec::new(ByteOrder::BigEndian), &bytes, 1),
        );
        write_unchecked(
            BinaryCodec::new(ByteOrder::BigEndian),
            &mut output,
            0,
            0x0102_0304_0506_0708,
        );
        assert_eq!(
            -0x1234,
            read_i16_unchecked(BinaryCodec::new(ByteOrder::BigEndian), &signed_bytes, 1),
        );
        write_i16_unchecked(
            BinaryCodec::new(ByteOrder::BigEndian),
            &mut signed_output,
            0,
            -0x1234,
        );
    }
    assert_eq!([1, 2, 3, 4, 5, 6, 7, 8], output);
    assert_eq!([0xed, 0xcc], signed_output);
}
