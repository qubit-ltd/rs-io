use qubit_io::CoderStatus;

#[test]
fn test_coder_status_variants_are_distinct() {
    assert_ne!(
        CoderStatus::Complete,
        CoderStatus::NeedInput {
            input_index: 0,
            required: 0,
            available: 0
        }
    );
    assert_ne!(
        CoderStatus::NeedInput {
            input_index: 0,
            required: 0,
            available: 0
        },
        CoderStatus::NeedOutput {
            output_index: 0,
            required: 0,
            available: 0,
        }
    );
}
