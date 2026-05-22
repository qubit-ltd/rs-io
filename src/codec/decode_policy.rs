/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

/// Describes a type-level decoding policy.
pub trait DecodePolicy: Copy + Default {
    /// Whether this policy rejects non-canonical encodings.
    const STRICT: bool;
}
