// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;
use std::io::SeekFrom;

use digest::Digest;

use crate::ByteOutput;
use crate::Output;
use crate::Seekable;
use crate::traits::validate_write_count;

/// Byte output wrapper that computes a digest of successfully accepted bytes.
///
/// Available with the `digest` feature.
///
/// Seeking changes only the output position; it does not reset or otherwise
/// modify the accumulated digest. The digest follows transfer order, including
/// overwriting writes; it does not reconstruct the final file contents.
///
/// # Type Parameters
///
/// * `O` - Wrapped byte output type.
/// * `D` - Digest state updated with successfully accepted bytes.
///
/// # Examples
///
/// ```
/// use sha2::Digest;
/// use sha2::Sha256;
///
/// use qubit_io::{DigestOutput, Output};
///
/// let mut output = DigestOutput::new(Vec::<u8>::new(), Sha256::new());
/// output.write_fully(b"abc").unwrap();
/// output.flush().unwrap();
/// let (sink, digest) = output.finalize();
/// assert_eq!(sink, b"abc");
/// assert_eq!(digest, Sha256::digest(b"abc"));
/// ```
#[must_use]
#[derive(Debug)]
pub struct DigestOutput<O, D> {
    /// Output whose successful bytes are hashed.
    inner: O,
    /// Digest state updated after successful writes.
    hasher: D,
}

impl<O, D> DigestOutput<O, D>
where
    D: Digest,
{
    /// Creates a digest output around `inner` and `hasher`.
    ///
    /// # Parameters
    ///
    /// - `inner`: Output that accepts bytes.
    /// - `hasher`: Digest state updated with successfully accepted bytes.
    ///
    /// # Returns
    ///
    /// Returns a wrapper preserving the supplied initial digest state.
    #[inline(always)]
    pub const fn new(inner: O, hasher: D) -> Self {
        Self { inner, hasher }
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
    /// Does not flush buffered output or close the underlying stream. Flush
    /// or close explicitly and check the result before finalizing when
    /// successful delivery must be confirmed. The digest covers accepted bytes,
    /// including bytes still buffered by the underlying output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped stream and the algorithm's full, fixed-size digest.
    #[inline]
    #[must_use]
    pub fn finalize(self) -> (O, digest::Output<D>) {
        (self.inner, self.hasher.finalize())
    }

    /// Returns a shared reference to the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output without allowing mutation.
    #[inline(always)]
    #[must_use]
    pub const fn inner(&self) -> &O {
        &self.inner
    }

    /// Returns mutable access to the wrapped output.
    ///
    /// Writes made through the returned reference bypass the hasher.
    ///
    /// # Returns
    ///
    /// Returns mutable access to the wrapped output.
    #[inline(always)]
    #[must_use]
    pub fn inner_mut(&mut self) -> &mut O {
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

    /// Consumes this wrapper without flushing the wrapped output.
    ///
    /// This method performs no I/O. Any buffering owned by the returned output
    /// remains pending and unchanged.
    ///
    /// # Returns
    ///
    /// Returns the wrapped output and its hasher state.
    #[inline(always)]
    #[must_use]
    pub fn into_parts(self) -> (O, D) {
        (self.inner, self.hasher)
    }
}

impl<O, D> Output for DigestOutput<O, D>
where
    O: ByteOutput,
    D: Digest,
{
    /// Byte item accepted by this output.
    type Item = u8;

    /// Returns the wrapped output's buffering declaration.
    ///
    /// # Returns
    ///
    /// Returns whether the wrapped output reports itself as buffered.
    #[inline(always)]
    fn is_buffered(&self) -> bool {
        self.inner.is_buffered()
    }

    /// Writes bytes and hashes only the successful prefix.
    ///
    /// # Parameters
    ///
    /// - `input`: Source byte slice.
    /// - `index`: Starting source index.
    /// - `count`: Maximum number of bytes to write.
    ///
    /// # Returns
    ///
    /// Returns the number of bytes written and added to the hasher.
    ///
    /// # Errors
    ///
    /// Returns an error from the wrapped output, including
    /// [`io::ErrorKind::InvalidData`] when it reports an impossible count. No
    /// bytes are added to the hasher when an error is returned.
    ///
    /// An invalid count returns [`io::ErrorKind::InvalidData`] without updating
    /// the hasher.
    ///
    /// # Safety
    ///
    /// `index..index + count` must be valid in `input`.
    #[inline]
    unsafe fn write_unchecked(&mut self, input: &[u8], index: usize, count: usize) -> io::Result<usize> {
        let written = self.inner.write(&input[index..index + count])?;
        validate_write_count(written, count)?;
        if written != 0 {
            self.hasher.update(&input[index..index + written]);
        }
        Ok(written)
    }

    /// Flushes the wrapped output.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` after the wrapped output is flushed.
    ///
    /// # Errors
    ///
    /// Returns an error reported by the wrapped output.
    #[inline(always)]
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<O, D> Seekable for DigestOutput<O, D>
where
    O: Seekable,
    D: Digest,
{
    /// Unit used by the wrapped output for stream positions.
    type Unit = O::Unit;

    /// Seeks the wrapped output without changing the hasher.
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
    /// Returns an error reported by the wrapped output.
    #[inline(always)]
    fn seek_to(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.inner.seek_to(position)
    }
}
