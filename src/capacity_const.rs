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
/// can rely on the same default behavior when they construct buffered item
/// wrappers with `new`.
///
/// [`BufferedInput`]: crate::buffered::BufferedInput
/// [`BufferedOutput`]: crate::buffered::BufferedOutput
pub const DEFAULT_BUFFER_CAPACITY: usize = 8 * 1024;

/// Default buffer size used by stream copy operations.
///
/// Used by [`Streams::copy_at_most`] and related bounded copy helpers.
///
/// [`Streams::copy_at_most`]: crate::Streams::copy_at_most
/// [`Streams::copy_at_most_with_buffer_size`]: crate::Streams::copy_at_most_with_buffer_size
pub const DEFAULT_COPY_BUFFER_SIZE: usize = 16 * 1024;

/// Default buffer size used by stream comparison operations.
///
/// Used by [`Streams::compare_content`].
///
/// [`Streams::compare_content`]: crate::Streams::compare_content
pub const DEFAULT_COMPARE_BUFFER_SIZE: usize = 16 * 1024;
