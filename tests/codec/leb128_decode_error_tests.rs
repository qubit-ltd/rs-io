use qubit_io::{
    Leb128DecodeError,
    Leb128DecodeErrorKind,
};

#[test]
fn test_new_stores_kind_and_index() {
    let error = Leb128DecodeError::new(Leb128DecodeErrorKind::Malformed, 3);

    assert_eq!(Leb128DecodeErrorKind::Malformed, error.kind());
    assert_eq!(3, error.index());
    assert_eq!("malformed LEB128 integer", error.to_string());
}
