use qubit_io::Strict;

#[test]
fn test_strict_is_copyable_default_marker() {
    let marker = Strict;

    assert_eq!(marker, Strict);
}
