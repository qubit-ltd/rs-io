/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use super::{
    ByteOrder,
    ByteOrderSpec,
};

/// Type-level marker for little-endian byte order.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LittleEndian;

impl ByteOrderSpec for LittleEndian {
    /// The little-endian byte order.
    const ORDER: ByteOrder = ByteOrder::LittleEndian;
}
