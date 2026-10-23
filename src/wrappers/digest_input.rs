// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;
use std::io::ErrorKind;
use std::io::SeekFrom;

use digest::Digest;

use crate::ByteInput;
use crate::DEFAULT_COPY_BUFFER_SIZE;
use crate::Input;
use crate::Seekable;
use crate::traits::validate_read_count;

/// Byte input wrapper that computes a digest of successfully returned bytes.
///
/// Available with the `digest` feature.
///
/// Seeking changes only the source position; it does not reset or otherwise
/// modify the accumulated digest. The digest follows transfer order, including
/// repeated reads; it does not reconstruct the original file contents.
///
/// # Examples
///
/// ```
/// use sha2::Digest;
/// use sha2::Sha256;
///
/// use qubit_io::DigestInput;
/// use qubit_io::Input;
///
/// let mut input = DigestInput::new(std::io::Cursor::new(b"abc"), Sha256::new());
/// let mut bytes = [0; 3];
/// assert_eq!(input.read(&mut bytes).expect("read input"), 3);
/// assert_eq!(&bytes, b"abc");
/// assert_eq!(input.digest(), Sha256::digest(b"abc"));
/// ```
///
/// # Type Parameters
///
/// * `I` - Wrapped byte input type.
/// * `D` - Digest state updated with successfully returned bytes.
#[must_use]
#[derive(Debug)]
pub struct DigestInput<I, D> {
    /// Input whose successful bytes are hashed.
    inner: I,
    /// Digest state updated after successful reads.
    hasher: D,
}

impl<I, D> DigestInput<I, D>
where
    D: Digest,
{
    /// Creates a digest input around `inner` and `hasher`.
    ///
    /// # Parameters
    ///
    /// - `inner`: Input that supplies bytes.
    /// - `hasher`: Digest state updated with successfully returned bytes.
    ///
    /// # Returns
    ///
    /// Returns a wrapper preserving the supplied initial digest state.
    #[inline(always)]
    pub const fn new(inner: I, hasher: D) -> Self {
        Self { inner, hasher }
    }

    /// Reads an input to EOF and computes a fresh digest of its remaining
    /// bytes.
    ///
    /// Starts at the current input position with a new algorithm state. Uses a
    /// fixed-size buffer for bulk reads, whether or not the input is buffered.
    /// The input remains accessible to the caller after success or failure.
    ///
    /// # Parameters
    ///
    /// - `input`: Byte input to consume from its current position to EOF.
    ///
    /// # Returns
    ///
    /// Returns the digest of all bytes read by this call. To digest an entire
    /// original stream, position it at the beginning before calling.
    ///
    /// # Errors
    ///
    /// Retries [`ErrorKind::Interrupted`]. Returns any other input error, or
    /// [`ErrorKind::InvalidData`] if the input reports more bytes than the
    /// buffer can hold. Bytes already consumed are not rewound; no partial
    /// digest is returned.
    ///
    /// # Examples
    ///
    /// ```
    /// use qubit_io::DigestInput;
    /// use sha2::{Digest, Sha256};
    ///
    /// let mut input = std::io::Cursor::new(b"abc");
    /// let digest = DigestInput::<_, Sha256>::digest_all(&mut input)?;
    /// assert_eq!(digest, Sha256::digest(b"abc"));
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn digest_all(input: &mut I) -> io::Result<digest::Output<D>>
    where
        I: ByteInput,
    {
        let mut hasher = D::new();
        let mut buffer = [0; DEFAULT_COPY_BUFFER_SIZE];
        loop {
            let read = match input.read(&mut buffer) {
                Ok(read) => read,
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            };
            validate_read_count(read, buffer.len())?;
            if read == 0 {
                return Ok(hasher.finalize());
            }
            hasher.update(&buffer[..read]);
        }
    }

    /// Returns a snapshot of the current digest without changing its state.
    ///
    /// Clones the algorithm state and finalizes the clone. Further transfers
    /// continue accumulating into the original state.
    ///
    /// # Returns
    ///
    /// Returns the algorithm's full, fixed-size digest of bytes tracked so far.
    #[inline]
    #[must_use]
    pub fn digest(&self) -> digest::Output<D>
    where
        D: Clone,
    {
        self.hasher.clone().finalize()
    }

    /// Consumes this wrapper and finalizes the digest without performing I/O.
    ///
    /// Does not read any remaining input. To digest the complete original
    /// input, start at its beginning and read it to EOF before finalizing.
    ///
    /// # Returns
    ///
    /// Returns the wrapped stream and the algorithm's full, fixed-size digest.
    #[inline]
    #[must_use]
    pub fn finalize(self) -> (I, digest::Output<D>) {
        (self.inner, self.hasher.finalize())
    }

    /// Returns a shared reference to the wrapped input.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input without allowing mutation.
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> &I {
        &self.inner
    }

    /// Returns mutable access to the wrapped input.
    ///
    /// Reads made through the returned reference bypass the hasher.
    ///
    /// # Returns
    ///
    /// Returns mutable access to the wrapped input.
    #[inline(always)]
    #[must_use]
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.inner
    }

    /// Returns a shared reference to the wrapped hasher.
    ///
    /// # Returns
    ///
    /// Returns the wrapped hasher without allowing mutation.
    #[inline(always)]
    #[must_use]
    pub const fn hasher(&self) -> &D {
        &self.hasher
    }

    /// Returns mutable access to the wrapped hasher.
    ///
    /// Direct changes become part of the digest state exposed by this
    /// wrapper.
    ///
    /// # Returns
    ///
    /// Returns mutable access to the wrapped hasher.
    #[inline(always)]
    #[must_use]
    pub fn hasher_mut(&mut self) -> &mut D {
        &mut self.hasher
    }

    /// Consumes this wrapper and returns its input and hasher.
    ///
    /// # Returns
    ///
    /// Returns the wrapped input and hasher.
    #[inline(always)]
    #[must_use]
    pub fn into_parts(self) -> (I, D) {
        (self.inner, self.hasher)
    }
}

impl<I, D> Input for DigestInput<I, D>
where
    I: ByteInput,
    D: Digest,
{
    /// Byte item returned by this input.
    type Item = u8;

    /// Returns the wrapped input's buffering declaration.
    ///
    /// # Returns
    ///
    /// Returns whether the wrapped input reports itself as buffered.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Reads bytes and hashes only the successful prefix.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination byte slice.
    /// - `index`: Starting destination index.
    /// - `count`: Maximum number of bytes to read.
    ///
    /// # Returns
    ///
    /// Returns the number of bytes read and added to the hasher.
    ///
    /// # Errors
    ///
    /// Returns an error from the wrapped input, including
    /// [`io::ErrorKind::InvalidData`] when it reports an impossible count. No
    /// bytes are added to the hasher when an error is returned.
    ///
    /// An invalid count returns [`io::ErrorKind::InvalidData`] without updating
    /// the hasher.
    ///
    /// # Safety
    ///
    /// `index..index + count` must be valid in `output`.
    #[inline]
    unsafe fn read_unchecked(&mut self, output: &mut [u8], index: usize, count: usize) -> io::Result<usize> {
        let read = self.inner.read(&mut output[index..index + count])?;
        validate_read_count(read, count)?;
        if read != 0 {
            self.hasher.update(&output[index..index + read]);
        }
        Ok(read)
    }
}

impl<I, D> Seekable for DigestInput<I, D>
where
    I: Seekable,
    D: Digest,
{
    /// Unit used by the wrapped input for stream positions.
    type Unit = I::Unit;

    /// Seeks the wrapped input without changing the hasher.
    ///
    /// # Parameters
    ///
    /// - `position`: Target stream position.
    ///
    /// # Returns
    ///
    /// Returns the resulting absolute stream position.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the wrapped input.
    #[inline(always)]
    fn seek_to(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek_to(position)
    }
}
