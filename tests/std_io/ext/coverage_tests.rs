// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#[cfg(coverage)]
mod coverage_tests {
    use std::collections::VecDeque;
    use std::io::{
        Cursor,
        ErrorKind,
    };

    use qubit_io::std_io::ext::ReadExt;
    use qubit_io::{
        Input,
        Output,
        Streams,
        coverage_add_item_count_overflow,
        coverage_fail_next_add_item_count,
        coverage_reset_add_item_count_hooks,
    };
    use qubit_utils::{
        coverage_fail_next_reserve,
        coverage_fail_next_string_reserve,
        coverage_fail_reserve_above,
        coverage_fail_reserve_after,
        coverage_reset_reserve_hooks,
    };

    fn reset_coverage_hooks() {
        coverage_reset_add_item_count_hooks();
        coverage_reset_reserve_hooks();
    }

    struct ChunkInput {
        chunks: VecDeque<Vec<u16>>,
    }

    impl ChunkInput {
        fn new(chunks: Vec<Vec<u16>>) -> Self {
            Self {
                chunks: VecDeque::from(chunks),
            }
        }
    }

    impl Input for ChunkInput {
        type Item = u16;

        unsafe fn read_unchecked(
            &mut self,
            output: &mut [u16],
            index: usize,
            count: usize,
        ) -> std::io::Result<usize> {
            let Some(chunk) = self.chunks.pop_front() else {
                return Ok(0);
            };
            let read = count.min(chunk.len());
            output[index..index + read].copy_from_slice(&chunk[..read]);
            if read < chunk.len() {
                self.chunks.push_front(chunk[read..].to_vec());
            }
            Ok(read)
        }
    }

    #[derive(Default)]
    struct CollectOutput {
        values: Vec<u16>,
    }

    impl Output for CollectOutput {
        type Item = u16;

        unsafe fn write_unchecked(
            &mut self,
            input: &[u16],
            index: usize,
            count: usize,
        ) -> std::io::Result<usize> {
            self.values.extend_from_slice(&input[index..index + count]);
            Ok(count)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_streams_reports_copied_item_count_overflow() {
        let error = coverage_add_item_count_overflow()
            .expect_err("copied item count overflow should fail");

        assert_eq!(ErrorKind::InvalidData, error.kind());
        assert_eq!("copied item count overflows u64", error.to_string());
    }

    #[test]
    fn test_streams_copy_input_to_output_reports_add_item_count_overflow() {
        reset_coverage_hooks();
        let mut input = ChunkInput::new(vec![vec![1, 2, 3]]);
        let mut output = CollectOutput::default();

        coverage_fail_next_add_item_count();
        let error = Streams::copy_input_to_output(&mut input, &mut output)
            .expect_err("copy should propagate copied item count overflow");

        assert_eq!(ErrorKind::InvalidData, error.kind());
    }

    #[test]
    fn test_streams_copy_input_to_output_at_most_reports_add_item_count_overflow()
     {
        reset_coverage_hooks();
        let mut input = ChunkInput::new(vec![vec![1, 2, 3]]);
        let mut output = CollectOutput::default();

        coverage_fail_next_add_item_count();
        let error =
            Streams::copy_input_to_output_at_most(&mut input, &mut output, 3)
                .expect_err(
                    "bounded copy should propagate copied item count overflow",
                );

        assert_eq!(ErrorKind::InvalidData, error.kind());
    }

    #[test]
    fn test_streams_copy_input_to_output_end_limited_reports_add_item_count_overflow()
     {
        reset_coverage_hooks();
        let mut input = ChunkInput::new(vec![vec![1, 2, 3]]);
        let mut output = CollectOutput::default();

        coverage_fail_next_add_item_count();
        let error = Streams::copy_input_to_output_end_limited(
            &mut input,
            &mut output,
            3,
        )
        .expect_err(
            "end-limited copy should propagate copied item count overflow",
        );

        assert_eq!(ErrorKind::InvalidData, error.kind());
    }

    #[test]
    fn test_streams_compare_content_reports_second_buffer_allocation_failure() {
        reset_coverage_hooks();
        let mut left = Cursor::new(b"abc".to_vec());
        let mut right = Cursor::new(b"abc".to_vec());

        coverage_fail_reserve_after(1);
        let error = Streams::compare_content_with_buffer_size(
            &mut left, &mut right, 4,
        )
        .expect_err(
            "compare should propagate second buffer allocation failures",
        );

        assert_eq!(ErrorKind::OutOfMemory, error.kind());
    }

    #[test]
    fn test_input_ext_copy_to_reports_create_vec_allocation_failure() {
        reset_coverage_hooks();
        let mut input = ChunkInput::new(vec![vec![1, 2, 3]]);
        let mut output = CollectOutput::default();

        coverage_fail_next_reserve();
        let error = Streams::copy_input_to_output(&mut input, &mut output)
            .expect_err("copy should propagate buffer allocation failures");

        assert_eq!(ErrorKind::OutOfMemory, error.kind());
        assert!(output.values.is_empty());
    }

    #[test]
    fn test_input_ext_copy_to_at_most_reports_create_vec_allocation_failure() {
        reset_coverage_hooks();
        let mut input = ChunkInput::new(vec![vec![1, 2, 3]]);
        let mut output = CollectOutput::default();

        coverage_fail_next_reserve();
        let error =
            Streams::copy_input_to_output_at_most(&mut input, &mut output, 2)
                .expect_err(
                    "bounded copy should propagate buffer allocation failures",
                );

        assert_eq!(ErrorKind::OutOfMemory, error.kind());
        assert!(output.values.is_empty());
    }

    #[test]
    fn test_copy_input_to_output_at_most_bounds_temporary_buffer() {
        reset_coverage_hooks();
        let mut input = ChunkInput::new(vec![vec![1]]);
        let mut output = CollectOutput::default();

        coverage_fail_reserve_above(1);
        let result =
            Streams::copy_input_to_output_at_most(&mut input, &mut output, 1);
        reset_coverage_hooks();
        let copied =
            result.expect("one-item copy should allocate one temporary item");

        assert_eq!(1, copied);
        assert_eq!(&[1], output.values.as_slice());
    }

    #[test]
    fn test_input_ext_copy_to_end_limited_reports_create_vec_allocation_failure()
     {
        reset_coverage_hooks();
        let mut input = ChunkInput::new(vec![vec![1, 2, 3]]);
        let mut output = CollectOutput::default();

        coverage_fail_next_reserve();
        let error = Streams::copy_input_to_output_end_limited(
            &mut input,
            &mut output,
            3,
        )
        .expect_err(
            "end-limited copy should propagate buffer allocation failures",
        );

        assert_eq!(ErrorKind::OutOfMemory, error.kind());
        assert!(output.values.is_empty());
    }

    #[test]
    fn test_copy_input_to_output_end_limited_bounds_temporary_buffer() {
        reset_coverage_hooks();
        let mut input = ChunkInput::new(Vec::new());
        let mut output = CollectOutput::default();

        coverage_fail_reserve_above(1);
        let result = Streams::copy_input_to_output_end_limited(
            &mut input,
            &mut output,
            0,
        );
        reset_coverage_hooks();
        let copied =
            result.expect("zero-item limit should allocate one probe item");

        assert_eq!(0, copied);
        assert!(output.values.is_empty());
    }

    #[test]
    fn test_input_ext_copy_to_end_limited_reports_collected_reserve_failure() {
        reset_coverage_hooks();
        let mut input = ChunkInput::new(vec![vec![1, 2, 3]]);
        let mut output = CollectOutput::default();

        coverage_fail_reserve_after(1);
        let error = Streams::copy_input_to_output_end_limited(
            &mut input,
            &mut output,
            3,
        )
        .expect_err("end-limited copy should propagate reserve failures");

        assert_eq!(ErrorKind::OutOfMemory, error.kind());
        assert!(output.values.is_empty());
    }

    #[test]
    fn test_read_ext_read_to_string_limited_into_reports_reserve_failure() {
        reset_coverage_hooks();
        let mut reader = Cursor::new(b"hello".to_vec());
        let mut output = String::from("seed-");

        coverage_fail_next_string_reserve();
        let error = reader
            .read_to_string_limited_into(&mut output, 8)
            .expect_err(
                "read_to_string_limited_into should propagate reserve failures",
            );

        assert_eq!(ErrorKind::OutOfMemory, error.kind());
        assert_eq!("seed-", output);
    }

    #[test]
    fn test_read_ext_read_to_end_limited_reports_initial_reserve_failure() {
        reset_coverage_hooks();
        let mut reader = Cursor::new(b"hello".to_vec());

        coverage_fail_next_reserve();
        let error = reader.read_to_end_limited(8).expect_err(
            "read_to_end_limited should propagate reserve failures",
        );

        assert_eq!(ErrorKind::OutOfMemory, error.kind());
        assert_eq!(0, reader.position());
    }

    #[test]
    fn test_read_ext_read_to_end_limited_into_rolls_back_on_reserve_failure() {
        reset_coverage_hooks();
        let mut reader = Cursor::new(b"hello".to_vec());
        let mut output = b"seed-".to_vec();

        coverage_fail_next_reserve();
        let error = reader.read_to_end_limited_into(&mut output, 8).expect_err(
            "read_to_end_limited_into should propagate reserve failures",
        );

        assert_eq!(ErrorKind::OutOfMemory, error.kind());
        assert_eq!(b"seed-", output.as_slice());
        assert_eq!(5, reader.position());
    }
}
