use qubit_io::ByteOrder;

#[test]
fn test_byte_order_variants_are_distinct_and_copyable() {
    let big = ByteOrder::BigEndian;
    let little = ByteOrder::LittleEndian;

    assert_eq!(ByteOrder::BigEndian, big);
    assert_eq!(ByteOrder::LittleEndian, little);
    assert_ne!(big, little);
}
