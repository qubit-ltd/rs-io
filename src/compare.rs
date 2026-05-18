/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::cmp::Ordering;
use std::io::{
    Read,
    Result,
};

use crate::read_ext::read_exact_or_eof_from;

/// Buffer size used by stream comparison operations.
const COMPARE_BUFFER_SIZE: usize = 16 * 1024;

/// Tests whether two readable streams have equal remaining contents.
///
/// The comparison starts at each reader's current position and consumes both
/// streams until a difference or EOF is found.
///
/// # Parameters
/// - `left`: First stream.
/// - `right`: Second stream.
///
/// # Returns
/// `true` when both streams produce the same bytes until EOF.
///
/// # Errors
/// Returns the first read error reported by either stream.
pub fn content_eq(left: &mut dyn Read, right: &mut dyn Read) -> Result<bool> {
    Ok(compare_content(left, right)? == Ordering::Equal)
}

/// Lexicographically compares the remaining contents of two readable streams.
///
/// The comparison starts at each reader's current position and consumes both
/// streams until a difference or EOF is found.
///
/// # Parameters
/// - `left`: First stream.
/// - `right`: Second stream.
///
/// # Returns
/// The lexicographic ordering of the remaining bytes.
///
/// # Errors
/// Returns the first read error reported by either stream.
pub fn compare_content(left: &mut dyn Read, right: &mut dyn Read) -> Result<Ordering> {
    let mut left_buffer = [0; COMPARE_BUFFER_SIZE];
    let mut right_buffer = [0; COMPARE_BUFFER_SIZE];
    loop {
        let left_count = read_exact_or_eof_from(left, &mut left_buffer)?;
        let right_count = read_exact_or_eof_from(right, &mut right_buffer)?;
        let common_count = left_count.min(right_count);
        for index in 0..common_count {
            match left_buffer[index].cmp(&right_buffer[index]) {
                Ordering::Equal => {}
                ordering => return Ok(ordering),
            }
        }
        match left_count.cmp(&right_count) {
            Ordering::Equal if left_count == 0 => return Ok(Ordering::Equal),
            Ordering::Equal => {}
            ordering => return Ok(ordering),
        }
    }
}
