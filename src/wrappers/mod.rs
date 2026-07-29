// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Synchronous and asynchronous item-stream wrappers.
//!
//! These adapters add counting, checksumming, transfer limits, tee behavior,
//! and position restoration while preserving the item-oriented traits of the
//! wrapped streams.

mod async_checksum_input;
mod async_checksum_output;
mod async_counting_input;
mod async_counting_output;
mod async_limit_input;
mod async_limit_output;
mod checksum_input;
mod checksum_output;
mod counting_input;
mod counting_output;
mod limit_input;
mod limit_output;
mod position_guard;
mod sync_seek_tee_input;
mod tee_input;
mod tee_output;

pub use async_checksum_input::AsyncChecksumInput;
pub use async_checksum_output::AsyncChecksumOutput;
pub use async_counting_input::AsyncCountingInput;
pub use async_counting_output::AsyncCountingOutput;
pub use async_limit_input::AsyncLimitInput;
pub use async_limit_output::AsyncLimitOutput;
pub use checksum_input::ChecksumInput;
pub use checksum_output::ChecksumOutput;
pub use counting_input::CountingInput;
pub use counting_output::CountingOutput;
pub use limit_input::LimitInput;
pub use limit_output::LimitOutput;
pub use position_guard::PositionGuard;
pub use sync_seek_tee_input::SyncSeekTeeInput;
pub use tee_input::TeeInput;
pub use tee_output::TeeOutput;
