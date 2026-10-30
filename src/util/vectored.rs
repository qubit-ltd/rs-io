// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

// qubit-style: allow coverage-cfg
use std::io;
use std::io::IoSlice;
use std::io::IoSliceMut;

use super::allocation_error;
use super::try_reserve_vec;

/// Sums requested slice lengths, returning `InvalidInput` on `usize` overflow.
/// Bridges use this check before preparing descriptors or invoking I/O.
///
/// # Parameters
///
/// - `lengths`: The lengths to sum, in transfer order.
///
/// # Returns
///
/// The total length when it fits in `usize`.
///
/// # Errors
///
/// Returns `InvalidInput` if the sum exceeds `usize::MAX`.
pub(crate) fn requested_items(mut lengths: impl Iterator<Item = usize>) -> io::Result<usize> {
    lengths.try_fold(0_usize, |total, len| {
        total
            .checked_add(len)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "vectored item count exceeds usize"))
    })
}

/// Converts borrowed byte slices to standard write descriptors.
///
/// The returned descriptors borrow the original payloads for `'a`; payload
/// bytes are not copied. The descriptor vector may allocate and returns
/// `OutOfMemory` if that allocation fails.
///
/// # Parameters
///
/// - `buffers`: The payload slices to describe, in transfer order.
///
/// # Returns
///
/// Descriptors borrowing each input payload in order.
///
/// # Errors
///
/// Returns `OutOfMemory` if reserving descriptor storage fails.
pub(crate) fn io_slices<'a>(buffers: &[&'a [u8]]) -> io::Result<Vec<IoSlice<'a>>> {
    let mut result = Vec::new();
    try_reserve_vec(&mut result, buffers.len()).map_err(allocation_error)?;
    result.extend(buffers.iter().map(|buffer| IoSlice::new(buffer)));
    Ok(result)
}

/// Converts borrowed mutable byte slices to standard read descriptors.
///
/// The returned descriptors exclusively borrow the original payloads for
/// `'a`; payload bytes are not copied. The descriptor vector may allocate and
/// returns `OutOfMemory` if that allocation fails.
///
/// # Parameters
///
/// - `buffers`: The mutable payload slices to describe, in read order.
///
/// # Returns
///
/// Descriptors exclusively borrowing each input payload in order.
///
/// # Errors
///
/// Returns `OutOfMemory` if reserving descriptor storage fails.
pub(crate) fn io_slices_mut<'a>(buffers: &'a mut [&mut [u8]]) -> io::Result<Vec<IoSliceMut<'a>>> {
    let mut result = Vec::new();
    try_reserve_vec(&mut result, buffers.len()).map_err(allocation_error)?;
    result.extend(buffers.iter_mut().map(|buffer| IoSliceMut::new(buffer)));
    Ok(result)
}

/// Extracts byte-slice borrows from standard write descriptors.
///
/// The result borrows `buffers` for `'a`, which keeps each descriptor alive
/// while its payload slice is used. It does not copy payload bytes. The result
/// vector may allocate and returns `OutOfMemory` if that allocation fails.
///
/// # Parameters
///
/// - `buffers`: The write descriptors whose payload slices are extracted.
///
/// # Returns
///
/// Payload slices borrowing the descriptors in their original order.
///
/// # Errors
///
/// Returns `OutOfMemory` if reserving the result vector fails.
pub(crate) fn byte_slices<'a>(buffers: &'a [IoSlice<'_>]) -> io::Result<Vec<&'a [u8]>> {
    let mut result = Vec::new();
    try_reserve_vec(&mut result, buffers.len()).map_err(allocation_error)?;
    result.extend(buffers.iter().map(|buffer| &buffer[..]));
    Ok(result)
}

/// Extracts exclusive byte-slice borrows from standard read descriptors.
///
/// The result borrows `buffers` for `'a`, keeping each descriptor alive while
/// its payload slice is used. It does not copy payload bytes. The result vector
/// may allocate and returns `OutOfMemory` if that allocation fails.
///
/// # Parameters
///
/// - `buffers`: The read descriptors whose mutable payload slices are
///   extracted.
///
/// # Returns
///
/// Mutable payload slices borrowing the descriptors in their original order.
///
/// # Errors
///
/// Returns `OutOfMemory` if reserving the result vector fails.
pub(crate) fn byte_slices_mut<'a>(buffers: &'a mut [IoSliceMut<'_>]) -> io::Result<Vec<&'a mut [u8]>> {
    let mut result = Vec::new();
    try_reserve_vec(&mut result, buffers.len()).map_err(allocation_error)?;
    result.extend(buffers.iter_mut().map(|buffer| &mut buffer[..]));
    Ok(result)
}
