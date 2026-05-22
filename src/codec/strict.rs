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

/// Marker type selecting strict decoding.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Strict;

impl DecodePolicy for Strict {
    /// Whether this policy rejects non-canonical encodings.
    const STRICT: bool = true;
}
