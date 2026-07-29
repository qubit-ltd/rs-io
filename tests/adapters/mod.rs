// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

mod box_async_input_tests;
mod box_async_output_tests;
mod box_input_tests;
mod box_output_tests;
#[cfg(feature = "futures-io")]
mod futures_async_read_tests;
#[cfg(feature = "futures-io")]
mod futures_async_write_tests;
#[cfg(feature = "futures-io")]
mod futures_input_tests;
#[cfg(feature = "futures-io")]
mod futures_io_tests;
#[cfg(feature = "futures-io")]
mod futures_output_tests;
mod input_ref_tests;
mod output_ref_tests;
#[cfg(feature = "tokio")]
mod tokio_async_read_tests;
#[cfg(feature = "tokio")]
mod tokio_async_write_tests;
#[cfg(feature = "tokio")]
mod tokio_input_tests;
#[cfg(feature = "tokio")]
mod tokio_output_tests;
#[cfg(feature = "tokio")]
mod tokio_tests;
