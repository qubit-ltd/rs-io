// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Compile-time `NonZeroUsize` construction helpers.

use core::num::NonZeroUsize;

/// Returns a [`NonZeroUsize`] from a known non-zero compile-time constant.
///
/// This helper panics during const evaluation when `value` is zero, surfacing
/// the violation at build time. At runtime it remains a single conditional
/// branch that the compiler folds away for constant inputs, eliminating the
/// `unsafe { NonZeroUsize::new_unchecked(...) }` ceremony every concrete codec
/// otherwise repeats.
///
/// # Parameters
///
/// - `value`: Non-zero item count.
///
/// # Returns
///
/// Returns a [`NonZeroUsize`] equal to `value`.
///
/// # Panics
///
/// Panics when `value` is zero.
#[must_use]
#[inline(always)]
pub const fn nz(value: usize) -> NonZeroUsize {
    match NonZeroUsize::new(value) {
        Some(v) => v,
        None => panic!("qubit_io::nz!(): value must be non-zero"),
    }
}

/// Const-friendly wrapper around [`nz`] for use in expression position.
///
/// Prefer this macro over `unsafe { NonZeroUsize::new_unchecked(...) }` in
/// `Codec::min_units_per_value` / `max_units_per_value` and similar sites
/// where the item count is a compile-time constant: the panic branch is
/// folded away and the call expands to a `NonZeroUsize` value the compiler
/// can prove is non-zero.
///
/// # Examples
///
/// ```ignore
/// use qubit_io::nz;
/// const MAX: core::num::NonZeroUsize = qubit_io::nz!(4);
/// ```
#[macro_export]
macro_rules! nz {
    ($value:expr) => {{ $crate::nz_const($value) }};
}

/// Re-export of [`nz`] under the name expected by `nz!`.
///
/// The macro qualifies its expansion with `$crate::nz_const` so that callers
/// can `use qubit_io::nz;` without also importing the function itself.
///
/// # Parameters
///
/// - `value`: Non-zero item count.
///
/// # Returns
///
/// Returns a [`NonZeroUsize`] equal to `value`.
///
/// # Panics
///
/// Panics when `value` is zero.
#[must_use]
#[inline(always)]
pub const fn nz_const(value: usize) -> NonZeroUsize {
    nz(value)
}
