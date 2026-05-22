/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
/// Reports why a [`crate::Coder`] stopped converting input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoderStatus {
    /// All currently supplied input was consumed.
    Complete,

    /// More input is needed to complete the next output value.
    ///
    /// - `input_index`: Absolute input index where input ended while decoding.
    /// - `required`: Number of additional input units required to continue.
    /// - `available`: Number of input units currently available from the current
    ///   input position.
    NeedInput {
        /// Absolute input index where input ended.
        input_index: usize,
        /// Number of additional input units required to continue.
        required: usize,
        /// Number of input units currently available.
        available: usize,
    },

    /// More output capacity is needed before conversion can continue.
    ///
    /// - `output_index`: Absolute output index where output ended while decoding.
    /// - `required`: Number of additional output units required to continue.
    /// - `available`: Number of output units currently available from the current
    ///   output position.
    NeedOutput {
        /// Absolute output index where output ended.
        output_index: usize,
        /// Number of additional output units required to continue.
        required: usize,
        /// Number of output units currently available.
        available: usize,
    },
}
