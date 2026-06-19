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

    use qubit_io::{
        Input,
        InputExt,
        Output,
        ReadExt,
        coverage_fail_next_add_copied,
        coverage_fail_next_reserve,
        coverage_fail_next_string_reserve,
        coverage_fail_reserve_after,
        coverage_natural_add_copied_overflow,
        coverage_reset_add_copied_hooks,
        coverage_reset_reserve_hooks,
    };

    fn reset_coverage_hooks() {
        coverage_reset_reserve_hooks();
        coverage_reset_add_copied_hooks();
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
    fn test_input_ext_copy_to_reports_create_vec_allocation_failure() {
        reset_coverage_hooks();
        let mut input = ChunkInput::new(vec![vec![1, 2, 3]]);
        let mut output = CollectOutput::default();

        coverage_fail_next_reserve();
        let error = input
            .copy_to(&mut output)
            .expect_err("copy_to should propagate buffer allocation failures");

        assert_eq!(ErrorKind::Other, error.kind());
        assert!(output.values.is_empty());
    }

    #[test]
    fn test_input_ext_copy_to_at_most_reports_create_vec_allocation_failure() {
        reset_coverage_hooks();
        let mut input = ChunkInput::new(vec![vec![1, 2, 3]]);
        let mut output = CollectOutput::default();

        coverage_fail_next_reserve();
        let error = input.copy_to_at_most(&mut output, 2).expect_err(
            "copy_to_at_most should propagate buffer allocation failures",
        );

        assert_eq!(ErrorKind::Other, error.kind());
        assert!(output.values.is_empty());
    }

    #[test]
    fn test_input_ext_copy_to_end_limited_reports_create_vec_allocation_failure()
     {
        reset_coverage_hooks();
        let mut input = ChunkInput::new(vec![vec![1, 2, 3]]);
        let mut output = CollectOutput::default();

        coverage_fail_next_reserve();
        let error = input.copy_to_end_limited(&mut output, 3).expect_err(
            "copy_to_end_limited should propagate buffer allocation failures",
        );

        assert_eq!(ErrorKind::Other, error.kind());
        assert!(output.values.is_empty());
    }

    #[test]
    fn test_input_ext_copy_to_reports_add_copied_overflow() {
        reset_coverage_hooks();
        let mut input = ChunkInput::new(vec![vec![1, 2, 3]]);
        let mut output = CollectOutput::default();

        coverage_fail_next_add_copied();
        let error = input
            .copy_to(&mut output)
            .expect_err("copy_to should propagate copied item count overflow");

        assert_eq!(ErrorKind::InvalidData, error.kind());
    }

    #[test]
    fn test_input_ext_copy_to_end_limited_reports_add_copied_overflow() {
        reset_coverage_hooks();
        let mut input = ChunkInput::new(vec![vec![1, 2, 3]]);
        let mut output = CollectOutput::default();

        coverage_fail_next_add_copied();
        let error = input.copy_to_end_limited(&mut output, 3).expect_err(
            "copy_to_end_limited should propagate copied item count overflow",
        );

        assert_eq!(ErrorKind::InvalidData, error.kind());
    }

    #[test]
    fn test_input_ext_copy_to_end_limited_reports_collected_reserve_failure() {
        reset_coverage_hooks();
        let mut input = ChunkInput::new(vec![vec![1, 2, 3]]);
        let mut output = CollectOutput::default();

        coverage_fail_reserve_after(1);
        let error = input.copy_to_end_limited(&mut output, 3).expect_err(
            "copy_to_end_limited should propagate collected reserve failures",
        );

        assert_eq!(ErrorKind::Other, error.kind());
        assert!(output.values.is_empty());
    }

    #[test]
    fn test_coverage_natural_add_copied_overflow() {
        let error = coverage_natural_add_copied_overflow()
            .expect_err("natural u64 overflow should be reported");

        assert_eq!(ErrorKind::InvalidData, error.kind());
        assert_eq!("copied item count overflows u64", error.to_string());
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

        assert_eq!(ErrorKind::Other, error.kind());
        assert_eq!("seed-", output);
    }
}
