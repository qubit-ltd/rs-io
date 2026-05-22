/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use crate::ByteOrder;

/// Describes a type-level byte order.
pub trait ByteOrderSpec: Copy + Default {
    /// Runtime value represented by this type-level byte order.
    const ORDER: ByteOrder;
}
