// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::{Error, ErrorKind, Result, Write};

/// Minimal indexed output interface over units.
///
/// `Output` is intentionally smaller and lower-level than [`Write`]. It only
/// states that an implementor can write up to `count` units from
/// `input[index..index + count]`, plus an explicit flush operation. The caller
/// owns range validation so hot paths can avoid repeated slicing and bounds
/// checks.
///
/// # Method name overlap
///
/// `Output::write`, `Output::write_all`, and `Output::flush` have the same
/// method names as [`Write`]. In generic code where both traits are in scope
/// for the same value, use fully qualified syntax to choose the intended
/// operation:
///
/// ```
/// use std::io::{
///     Result,
///     Write,
/// };
///
/// use qubit_io::Output;
///
/// fn flush_units<T>(output: &mut T) -> Result<()>
/// where
///     T: Output + Write,
/// {
///     <T as Output>::flush(output)
/// }
///
/// fn write_units<T>(output: &mut T, input: &[u8]) -> Result<()>
/// where
///     T: Output<Item = u8> + Write,
/// {
///     unsafe { <T as Output>::write_all(output, input, 0, input.len()) }
/// }
///
/// fn flush_bytes<T>(output: &mut T) -> Result<()>
/// where
///     T: Output + Write,
/// {
///     Write::flush(output)
/// }
/// ```
pub trait Output {
    /// The unit type written to this output.
    type Item;

    /// Writes units from an indexed input range without checking the range.
    ///
    /// # Parameters
    ///
    /// * `input` - Source storage.
    /// * `index` - Start index inside `input`.
    /// * `count` - Maximum number of units to write.
    ///
    /// # Returns
    ///
    /// The number of units accepted from `input[index..index + count]`. The
    /// value must be in `0..=count`.
    ///
    /// # Errors
    ///
    /// Returns the output error reported by the implementation.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + count` is a valid range
    /// inside `input` and that the addition does not overflow.
    unsafe fn write(&mut self, input: &[Self::Item], index: usize, count: usize) -> Result<usize>;

    /// Writes all units from an indexed input range without checking the range.
    ///
    /// This method repeatedly calls [`Output::write`] until all
    /// `count` units are accepted. Interrupted writes are retried. A zero
    /// progress report before the range is complete is converted to
    /// [`ErrorKind::WriteZero`].
    ///
    /// # Parameters
    ///
    /// * `input` - Source storage.
    /// * `index` - Start index inside `input`.
    /// * `count` - Number of units to write.
    ///
    /// # Errors
    ///
    /// Returns the output error reported by the implementation. Returns
    /// [`ErrorKind::WriteZero`] if the implementation accepts zero units before
    /// the requested range is complete. Returns [`ErrorKind::InvalidData`] if
    /// the implementation reports accepting more units than requested.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + count` is a valid range
    /// inside `input` and that the addition does not overflow.
    unsafe fn write_all(&mut self, input: &[Self::Item], index: usize, count: usize) -> Result<()> {
        debug_assert!(
            index
                .checked_add(count)
                .is_some_and(|end| end <= input.len()),
            "unchecked write-all range exceeds input buffer"
        );
        let mut written = 0;
        while written < count {
            let remaining = count - written;
            // SAFETY: The caller guarantees the original source range is
            // valid; `written < count`, so this suffix remains inside it.
            match unsafe { self.write(input, index + written, remaining) } {
                Ok(0) => {
                    return Err(Error::new(
                        ErrorKind::WriteZero,
                        "failed to write whole output range",
                    ));
                }
                Ok(progress) => {
                    if progress > remaining {
                        return Err(Error::new(
                            ErrorKind::InvalidData,
                            format!(
                                "writer reported {progress} units for a {remaining}-unit range"
                            ),
                        ));
                    }
                    written += progress;
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    /// Flushes any internally buffered units.
    ///
    /// # Errors
    ///
    /// Returns the output error reported by the implementation.
    fn flush(&mut self) -> Result<()>;
}

impl<W> Output for W
where
    W: Write + ?Sized,
{
    type Item = u8;

    /// Writes bytes to a standard [`Write`] value from an indexed range.
    #[inline(always)]
    unsafe fn write(&mut self, input: &[u8], index: usize, count: usize) -> Result<usize> {
        debug_assert!(
            index
                .checked_add(count)
                .is_some_and(|end| end <= input.len()),
            "unchecked write range exceeds input buffer"
        );
        // SAFETY: The caller guarantees that the range is valid inside
        // `input`.
        let source = unsafe { core::slice::from_raw_parts(input.as_ptr().add(index), count) };
        Write::write(self, source)
    }

    /// Flushes a standard [`Write`] value.
    #[inline(always)]
    fn flush(&mut self) -> Result<()> {
        Write::flush(self)
    }
}
