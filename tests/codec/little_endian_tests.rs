use qubit_io::LittleEndian;

#[test]
fn test_little_endian_is_copyable_default_marker() {
    let marker = LittleEndian;

    assert_eq!(marker, LittleEndian);
}
