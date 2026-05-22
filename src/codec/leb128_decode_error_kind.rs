/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use thiserror::Error;

/// Classifies failures detected while decoding LEB128 integers.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum Leb128DecodeErrorKind {
    /// The input bytes cannot represent a value of the requested width.
    #[error("malformed LEB128 integer")]
    Malformed,

    /// Strict decoding rejected a value that was not minimally encoded.
    #[error("non-canonical LEB128 integer")]
    NonCanonical,
}
