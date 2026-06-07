// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

/// Default capacity used by [`BufferedInput`] and [`BufferedOutput`].
///
/// The value is intentionally shared by input and output buffering so callers
/// can rely on the same default behavior when they construct buffered byte
/// wrappers with `new`.
///
/// [`BufferedInput`]: crate::buffered::BufferedInput
/// [`BufferedOutput`]: crate::buffered::BufferedOutput
pub const DEFAULT_BUFFER_CAPACITY: usize = 8 * 1024;
