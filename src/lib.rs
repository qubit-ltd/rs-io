/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! # Qubit IO
//!
//! Small I/O trait utilities for Rust.
//!

use std::io::{Read, Seek, Write};

/// Trait object friendly alias for types that implement [`Read`] and [`Seek`].
pub trait ReadSeek: Read + Seek {}

impl<T> ReadSeek for T where T: Read + Seek {}

/// Trait object friendly alias for types that implement [`Read`] and [`Write`].
pub trait ReadWrite: Read + Write {}

impl<T> ReadWrite for T where T: Read + Write {}

/// Trait object friendly alias for types that implement [`Write`] and [`Seek`].
pub trait WriteSeek: Write + Seek {}

impl<T> WriteSeek for T where T: Write + Seek {}

/// Trait object friendly alias for types that implement [`Read`], [`Write`], and [`Seek`].
pub trait ReadWriteSeek: Read + Write + Seek {}

impl<T> ReadWriteSeek for T where T: Read + Write + Seek {}
