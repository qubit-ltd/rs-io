// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Common extension and composition traits for Qubit IO users.
//!
//! Importing this module brings the method-providing extension traits and the
//! object-safe composition traits into scope without importing wrapper types or
//! utility functions.

pub use crate::{
    BufReadExt, BufReadSeek, Input, Output, ReadExt, ReadSeek, ReadSeekExt, ReadWrite,
    ReadWriteSeek, SeekExt, Seekable, SeekableInput, SeekableOutput, WriteExt, WriteSeek,
    WriteSeekExt,
};
