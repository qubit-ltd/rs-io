use qubit_io::Leb128DecodeErrorKind;

#[test]
fn test_display_formats_decode_error_kinds() {
    assert_eq!("malformed LEB128 integer", Leb128DecodeErrorKind::Malformed.to_string());
    assert_eq!(
        "non-canonical LEB128 integer",
        Leb128DecodeErrorKind::NonCanonical.to_string()
    );
}
