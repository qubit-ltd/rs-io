use qubit_io::{
    DecodePolicy,
    NonStrict,
    Strict,
};

fn is_strict<P: DecodePolicy>() -> bool {
    P::STRICT
}

#[test]
fn test_decode_policy_exposes_strict_flag() {
    assert!(is_strict::<Strict>());
    assert!(!is_strict::<NonStrict>());
}
