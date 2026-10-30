// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Error;
use std::io::ErrorKind;
use std::io::Result;

use super::validate_read_count;
use crate::ByteInput;
use crate::StdRead;
use crate::util::SliceRange;

/// Minimal indexed input interface over items.
///
/// `Input` is intentionally smaller and lower-level than [`std::io::Read`]. It
/// only states that an implementor can read up to `count` items into
/// `output[index..index + count]`. The caller owns range validation so hot
/// paths can avoid repeated slicing and bounds checks.
///
/// # Coherence note
///
/// Every [`std::io::Read`] value automatically implements `Input<Item = u8>`
/// through the standard I/O integration. Because [`Input::Item`] is an
/// associated type rather than a trait parameter, a concrete type that
/// implements [`std::io::Read`] cannot also provide any other direct `Input`
/// implementation for the same type, including one with a different item type.
///
/// Use a wrapper/newtype when a type needs item-oriented input semantics that
/// differ from its byte-oriented [`std::io::Read`] implementation.
///
/// # Examples
///
/// Standard readers implement `Input<Item = u8>` automatically, so callers
/// can use the trait methods directly on them:
///
/// ```
/// use std::io::Cursor;
///
/// use qubit_io::Input;
///
/// let mut input = Cursor::new(b"data");
/// let mut output = [0; 4];
/// assert_eq!(Input::read(&mut input, &mut output).unwrap(), 4);
/// assert_eq!(&output, b"data");
/// ```
pub trait Input {
    /// The item type read from this input.
    type Item;

    /// Converts this byte input into a standard library reader.
    ///
    /// The conversion only wraps `self`; it performs no I/O and does not
    /// allocate. The wrapped input can be recovered with
    /// [`StdRead::into_inner`].
    ///
    /// # Returns
    ///
    /// A standard library reader that owns the converted input.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_io::Input;
    /// use std::io::Read;
    ///
    /// let input = std::io::Cursor::new(b"hello");
    /// let mut reader = input.into_std_read();
    /// let mut bytes = Vec::new();
    /// reader.read_to_end(&mut bytes).unwrap();
    /// assert_eq!(bytes, b"hello");
    /// let input = reader.into_inner();
    /// assert_eq!(input.position(), 5);
    /// ```
    ///
    /// Non-byte inputs cannot be converted:
    ///
    /// ```compile_fail
    /// use qubit_io::Input;
    ///
    /// fn convert<I: Input<Item = char>>(input: I) {
    ///     let _reader = input.into_std_read();
    /// }
    /// ```
    #[inline]
    fn into_std_read(self) -> StdRead<Self>
    where
        Self: Sized + ByteInput,
    {
        StdRead::new(self)
    }

    /// Returns whether this input already buffers items internally.
    ///
    /// # Returns
    ///
    /// `true` when callers should avoid wrapping this input in another generic
    /// item buffer automatically.
    #[inline(always)]
    #[must_use]
    fn is_buffered(&self) -> bool {
        false
    }

    /// Reads items into an indexed output range without checking the range.
    ///
    /// # Parameters
    ///
    /// * `output` - Destination storage.
    /// * `index` - Start index inside `output`.
    /// * `count` - Maximum number of items to read.
    ///
    /// # Returns
    ///
    /// The number of items written into `output[index..index + count]`. The
    /// value must be in `0..=count`. A successful call transfers a prefix of
    /// the requested range and may be short. For a non-empty request, `Ok(0)`
    /// means that no items were available in this call or that the stream
    /// reached its end; it does not promise permanent EOF. Aggregate reads
    /// stop on this zero progress: [`Input::read_fully`] returns the
    /// accumulated count, while [`Input::read_exactly`] reports
    /// [`ErrorKind::UnexpectedEof`] if the destination is not full. For a
    /// zero-length request, a successful call returns zero, which does not
    /// establish EOF. Empty requests are not guaranteed to suppress
    /// implementation errors or avoid an inner call.
    ///
    /// # Errors
    ///
    /// Returns the input error reported by the implementation. An
    /// [`ErrorKind::Interrupted`] result is retried by aggregate read methods
    /// and therefore means this call made no confirmed transfer. If a valid
    /// prefix was transferred, return `Ok(n)` for it and report a later error
    /// on a subsequent call. A later aggregate error does not undo successful
    /// earlier calls. Errors with applied or uncertain side effects must not
    /// be represented as retryable `Interrupted` solely to request a retry;
    /// the adapter or caller must choose a policy for those effects.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + count` is a valid range
    /// inside `output` and that the addition does not overflow.
    unsafe fn read_unchecked(&mut self, output: &mut [Self::Item], index: usize, count: usize) -> Result<usize>;

    /// Reads items into the full output slice.
    ///
    /// # Parameters
    /// - `output`: Destination storage.
    ///
    /// # Returns
    /// The number of items read into `output`. A successful call transfers a
    /// prefix and may be short. For a non-empty request, `Ok(0)` means that no
    /// items were available in this call or that the stream reached its end;
    /// it does not promise permanent EOF. Aggregate reads stop on this zero
    /// progress: [`Input::read_fully`] returns the accumulated count, while
    /// [`Input::read_exactly`] reports [`ErrorKind::UnexpectedEof`] if the
    /// destination is not full. For a zero-length request, a successful call
    /// returns zero, which does not establish EOF. Empty requests are not
    /// guaranteed to suppress implementation errors or avoid an inner call.
    ///
    /// # Errors
    ///
    /// Returns the input error reported by the implementation. Returns
    /// [`ErrorKind::InvalidData`] if the implementation reports reading more
    /// items than requested.
    #[inline(always)]
    fn read(&mut self, output: &mut [Self::Item]) -> Result<usize> {
        // SAFETY: The caller ensured the destination slice is valid.
        let read = unsafe { self.read_unchecked(output, 0, output.len()) }?;
        validate_read_count(read, output.len())?;
        Ok(read)
    }

    /// Reads items into an indexed output range until it is full or EOF is
    /// reached.
    ///
    /// This method retries interrupted reads and treats EOF as a successful
    /// partial result.
    ///
    /// # Parameters
    ///
    /// * `output` - Destination storage.
    /// * `index` - Start index inside `output`.
    /// * `count` - Maximum number of items to read.
    ///
    /// # Returns
    ///
    /// The number of items written into `output[index..index + count]`.
    ///
    /// # Errors
    ///
    /// Returns the first non-[`ErrorKind::Interrupted`] input error. Returns
    /// [`ErrorKind::InvalidData`] if the implementation reports more items than
    /// requested.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the requested output range does not fit.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index..index + count` is a valid range
    /// inside `output` and that the addition does not overflow.
    unsafe fn read_fully_unchecked(&mut self, output: &mut [Self::Item], index: usize, count: usize) -> Result<usize> {
        debug_assert!(
            SliceRange::range_fits(output.len(), index, count),
            "unchecked read-fully range exceeds output buffer"
        );
        let mut total = 0;
        while total < count {
            let remaining = count - total;
            // SAFETY: The caller guarantees the original destination range is
            // valid; `total < count`, so this suffix remains inside it.
            match unsafe { self.read_unchecked(output, index + total, remaining) } {
                Ok(0) => break,
                Ok(read) => {
                    validate_read_count(read, remaining)?;
                    total += read;
                }
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
        Ok(total)
    }

    /// Reads items into the full output slice until it is full or EOF is
    /// reached.
    ///
    /// # Parameters
    /// - `output`: Destination storage to fill as far as possible.
    ///
    /// # Returns
    /// The number of items written into `output`.
    ///
    /// # Errors
    /// Returns the first non-interrupted input error, or
    /// [`ErrorKind::InvalidData`] if the implementation reports an impossible
    /// item count.
    #[inline(always)]
    fn read_fully(&mut self, output: &mut [Self::Item]) -> Result<usize> {
        // SAFETY: The full output slice is a valid destination range.
        unsafe { self.read_fully_unchecked(output, 0, output.len()) }
    }

    /// Reads items until the full output slice is filled.
    ///
    /// # Parameters
    ///
    /// * `output` - Destination storage that must be filled completely.
    ///
    /// # Returns
    ///
    /// `Ok(())` after the destination has been filled completely.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::UnexpectedEof`] if the input ends before filling
    /// `output`. Returns the first non-interrupted input error, or
    /// [`ErrorKind::InvalidData`] if the implementation reports an impossible
    /// item count.
    #[inline]
    fn read_exactly(&mut self, output: &mut [Self::Item]) -> Result<()> {
        if self.read_fully(output)? == output.len() {
            Ok(())
        } else {
            Err(Error::new(
                ErrorKind::UnexpectedEof,
                "failed to fill whole input buffer",
            ))
        }
    }

    /// Reads items into multiple buffers in list order.
    ///
    /// The buffers form one logical sequence in list order. `Ok(n)` means that
    /// the first `n` items of that sequence were read, and `n` must not exceed
    /// the total requested length. A read may make short progress and need not
    /// fill every buffer; `n` may cross a buffer boundary. For a non-empty
    /// request, `Ok(0)` means that no data was available in this call or that
    /// the stream reached its end; it does not promise permanent EOF.
    ///
    /// The default implementation skips empty buffers and calls [`Input::read`]
    /// once on the first non-empty buffer. It does not continue to later
    /// buffers after a short read. This is only the default strategy, not a
    /// limit on overrides. Empty lists and lists whose buffers are all empty
    /// return `Ok(0)` without performing I/O. Interrupted errors are not
    /// retried. An error does not undo progress from earlier independent calls.
    /// An override must uphold the sequence and count contract itself; this
    /// trait does not validate counts returned by arbitrary overrides. Use
    /// UFCS when a standard I/O method with the same name is also in scope.
    ///
    /// # Parameters
    ///
    /// - `buffers`: Mutable destination slices, in the order to read them.
    ///
    /// # Returns
    ///
    /// The number of items read from the start of the logical sequence.
    ///
    /// # Errors
    ///
    /// Returns an error from the input operation. The default implementation
    /// returns [`ErrorKind::InvalidData`] if the selected buffer reports more
    /// items than it can hold. The standard I/O blanket implementation also
    /// returns [`ErrorKind::InvalidInput`] if the total length overflows,
    /// [`ErrorKind::OutOfMemory`] if preparing descriptors fails, or
    /// [`ErrorKind::InvalidData`] if the standard reader reports more than the
    /// total capacity. A custom override is responsible for its own checks.
    ///
    /// # Examples
    ///
    /// This example uses a non-byte stream through a trait object. The default
    /// method reads only the first non-empty slice; an override may use more.
    /// For vectored overrides, a count can cross a slice boundary: if two
    /// destination slices each have length 2, `Ok(3)` means the first slice
    /// and the first item of the second slice were filled.
    ///
    /// ```
    /// use qubit_io::Input;
    ///
    /// struct Characters;
    /// impl Input for Characters {
    ///     type Item = char;
    ///     unsafe fn read_unchecked(
    ///         &mut self,
    ///         output: &mut [char],
    ///         index: usize,
    ///         count: usize,
    ///     ) -> std::io::Result<usize> {
    ///         if count == 0 {
    ///             return Ok(0);
    ///         }
    ///         output[index] = '甲';
    ///         Ok(1)
    ///     }
    /// }
    /// let mut input = Characters;
    /// let input: &mut dyn Input<Item = char> = &mut input;
    /// let mut first = ['!'; 2];
    /// let mut second = ['?'; 2];
    /// assert_eq!(Input::read_vectored(input, &mut [&mut first, &mut second])?, 1);
    /// assert_eq!(first, ['甲', '!']);
    /// assert_eq!(second, ['?', '?']);
    /// # Ok::<(), std::io::Error>(())
    /// ```
    fn read_vectored(&mut self, buffers: &mut [&mut [Self::Item]]) -> Result<usize> {
        let Some(buffer) = buffers.iter_mut().find(|buffer| !buffer.is_empty()) else {
            return Ok(0);
        };
        let count = self.read(buffer)?;
        validate_read_count(count, buffer.len())?;
        Ok(count)
    }
}
