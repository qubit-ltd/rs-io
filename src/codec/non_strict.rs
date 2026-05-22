/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use super::DecodePolicy;

/// Marker type selecting non-strict decoding.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NonStrict;

impl DecodePolicy for NonStrict {
    /// Whether this policy accepts non-canonical encodings.
    const STRICT: bool = false;
}
