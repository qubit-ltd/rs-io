use qubit_io::BigEndian;

#[test]
fn test_big_endian_is_copyable_default_marker() {
    let marker = BigEndian;

    assert_eq!(marker, BigEndian);
}
