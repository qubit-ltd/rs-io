/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use super::coder_progress::CoderProgress;

/// Converts one sequence of code units into another sequence of code units.
///
/// `convert` is the main streaming API. It transforms a provided input segment and
/// writes as much output as available buffer space allows, without automatically
/// flushing internal pending state.
///
/// The method is suitable for:
/// - pull-style consumers that call conversion repeatedly as buffers arrive;
/// - bounded output sinks that need `NeedOutput` progress when capacity is hit;
/// - stateless and stateful codecs that all return progress-oriented stopping
///   reasons.
///
/// `Coder` is intentionally independent from any charset semantics:
///
/// - Use `Coder` directly for custom, policy-free unit transforms.
/// - Use `Coder` when you want to own malformed/unmappable decisions at the call site.
///
/// # Example: raw unit transform
///
/// # Example
///
/// ```rust
/// use qubit_io::{Coder, CoderProgress, CoderStatus};
///
/// #[derive(Default)]
/// struct ByteCopy;
///
/// impl Coder<u8, u8> for ByteCopy {
///     type Error = core::convert::Infallible;
///
///     fn max_output_len(&self, input_len: usize) -> Option<usize> {
///         Some(input_len)
///     }
///
///     fn convert(
///         &mut self,
///         input: &[u8],
///         input_index: usize,
///         output: &mut [u8],
///         output_index: usize,
///     ) -> Result<CoderProgress, Self::Error> {
///         let mut read = 0;
///         let mut written = 0;
///         while input_index + read < input.len() && output_index + written < output.len() {
///             output[output_index + written] = input[input_index + read];
///             read += 1;
///             written += 1;
///         }
///         if input_index + read == input.len() {
///             Ok(CoderProgress::complete(read, written))
///         } else {
///             let status = CoderStatus::NeedOutput {
///                 output_index: output_index + written,
///                 required: 1,
///                 available: output.len().saturating_sub(output_index + written),
///             };
///             Ok(CoderProgress::new(
///                 status,
///                 read,
///                 written,
///             ))
///         }
///     }
/// }
///
/// let mut coder = ByteCopy;
/// let mut output = [0_u8; 2];
/// let progress = coder
///     .convert(&[1, 2, 3], 0, &mut output, 0)
///     .expect("convert works on partial output");
/// assert!(matches!(progress.status(), CoderStatus::NeedOutput { .. }));
/// assert_eq!(2, progress.read());
/// assert_eq!(2, progress.written());
/// assert_eq!([1, 2], output);
/// ```
///
/// The trait is intentionally independent from charset concepts. Implementors
/// use `input_index` and `output_index` as absolute positions in the supplied
/// slices. Returned progress counters are relative counts from those positions.
/// For raw codecs this gives a compact API; higher-level workflows can wrap this
/// trait with their own semantic policies.
///
/// Use `Coder` directly when you need a byte/word-buffer converter without any
/// charset policy coupling:
///
/// ```rust
/// use qubit_io::{Coder, CoderProgress, CoderStatus};
///
/// #[derive(Default)]
/// struct ByteCopy;
///
/// impl Coder<u8, u8> for ByteCopy {
///     type Error = core::convert::Infallible;
///
///     fn max_output_len(&self, input_len: usize) -> Option<usize> {
///         Some(input_len)
///     }
///
///     fn convert(
///         &mut self,
///         input: &[u8],
///         input_index: usize,
///         output: &mut [u8],
///         output_index: usize,
///     ) -> Result<CoderProgress, Self::Error> {
///         let mut read = 0;
///         let mut written = 0;
///         while input_index + read < input.len() && output_index + written < output.len() {
///             output[output_index + written] = input[input_index + read];
///             read += 1;
///             written += 1;
///         }
///         if input_index + read == input.len() {
///             Ok(CoderProgress::complete(read, written))
///         } else {
///             let status = CoderStatus::NeedOutput {
///                 output_index: output_index + written,
///                 required: 1,
///                 available: output.len().saturating_sub(output_index + written),
///             };
///             Ok(CoderProgress::new(
///                 status,
///                 read,
///                 written,
///             ))
///         }
///     }
/// }
///
/// let mut coder = ByteCopy;
/// let mut output = [0_u8; 1];
/// let progress = coder
///     .convert(&[1], 0, &mut output, 0)
///     .expect("convert works on partial output");
/// assert!(matches!(progress.status(), CoderStatus::Complete));
/// ```
///
/// # Type Parameters
///
/// - `Input`: Input unit type accepted by this coder.
/// - `Output`: Output unit type produced by this coder.
pub trait Coder<Input, Output> {
    /// Error reported for semantic conversion failures.
    type Error;

    /// Returns an upper bound for output units produced from `input_len` units.
    ///
    /// # Parameters
    ///
    /// - `input_len`: Number of input units the caller plans to convert.
    ///
    /// # Returns
    ///
    /// Returns `Some(bound)` when the coder can provide a finite upper bound.
    /// Returns `None` when the bound is not known.
    #[must_use]
    fn max_output_len(&self, input_len: usize) -> Option<usize>;

    /// Resets state retained between conversion calls.
    ///
    /// Stateless coders may keep the default no-op implementation.
    #[inline]
    fn reset(&mut self) {}

    /// Converts input units into output units.
    ///
    /// # Parameters
    ///
    /// - `input`: Complete input unit slice visible to the coder.
    /// - `input_index`: Absolute input unit index where conversion starts.
    /// - `output`: Complete output unit slice visible to the coder.
    /// - `output_index`: Absolute output unit index where writing starts.
    ///
    /// # Returns
    ///
    /// Returns progress describing how many units were consumed and produced and
    /// why conversion stopped.
    ///
    /// # Errors
    ///
    /// Returns `Self::Error` for semantic conversion failures that the coder's
    /// policy does not absorb.
    fn convert(
        &mut self,
        input: &[Input],
        input_index: usize,
        output: &mut [Output],
        output_index: usize,
    ) -> Result<CoderProgress, Self::Error>;

    /// Flushes any buffered output after input conversion is complete.
    ///
    /// `convert` handles input consumption. `finish` is called only after all
    /// source input has been provided and is used to flush buffered state
    /// (for example, a pending decoded character).
    ///
    /// # Example
    ///
    /// ```rust
    /// use qubit_io::{Coder, CoderStatus};
    ///
    /// #[derive(Default)]
    /// struct ByteCopy;
    ///
    /// impl Coder<u8, u8> for ByteCopy {
    ///     type Error = core::convert::Infallible;
    ///
    ///     fn max_output_len(&self, input_len: usize) -> Option<usize> {
    ///         Some(input_len)
    ///     }
    ///
    ///     fn convert(
    ///         &mut self,
    ///         input: &[u8],
    ///         input_index: usize,
    ///         output: &mut [u8],
    ///         output_index: usize,
    ///     ) -> Result<qubit_io::CoderProgress, Self::Error> {
    ///         let mut read = 0;
    ///         let mut written = 0;
    ///         while input_index + read < input.len() && output_index + written < output.len() {
    ///             output[output_index + written] = input[input_index + read];
    ///             read += 1;
    ///             written += 1;
    ///         }
    ///         if input_index + read == input.len() {
    ///             Ok(qubit_io::CoderProgress::complete(read, written))
    ///         } else {
    ///             let status = qubit_io::CoderStatus::NeedOutput {
    ///                 output_index: output_index + written,
    ///                 required: 1,
    ///                 available: output.len().saturating_sub(output_index + written),
    ///             };
    ///             Ok(qubit_io::CoderProgress::new(
    ///                 status,
    ///                 read,
    ///                 written,
    ///             ))
    ///         }
    ///     }
    /// }
    ///
    /// let mut coder = ByteCopy;
    /// let mut output = [1_u8; 1];
    /// let progress = coder
    ///     .convert(&[7], 0, &mut output, 0)
    ///     .expect("writer consumes one unit");
    /// assert_eq!(CoderStatus::Complete, progress.status());
    ///
    /// let finish = coder
    ///     .finish(&mut output, 1)
    ///     .expect("finish does not emit buffered state for no-op coders");
    /// assert_eq!(CoderStatus::Complete, finish.status());
    /// ```
    ///
    /// # Parameters
    ///
    /// - `output`: Complete output unit slice visible to the coder.
    /// - `output_index`: Absolute output unit index where writing starts.
    ///
    /// # Returns
    ///
    /// Returns progress for units written during flushing. Stateless coders
    /// return a completed progress value with zero counters.
    ///
    /// # Errors
    ///
    /// Returns `Self::Error` if pending state cannot be flushed according to the
    /// coder's policy.
    #[inline]
    fn finish(
        &mut self,
        _output: &mut [Output],
        _output_index: usize,
    ) -> Result<CoderProgress, Self::Error> {
        Ok(CoderProgress::complete(0, 0))
    }
}
