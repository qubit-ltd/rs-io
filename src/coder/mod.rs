/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
//! Progress-oriented conversion traits and status types.

#![allow(clippy::module_inception)]

mod coder;
mod coder_progress;
mod coder_status;

pub use coder::Coder;
pub use coder_progress::CoderProgress;
pub use coder_status::CoderStatus;
