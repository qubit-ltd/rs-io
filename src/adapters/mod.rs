// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

mod box_input;
mod box_output;
#[cfg(feature = "futures-io")]
mod futures_async_read;
#[cfg(feature = "futures-io")]
mod futures_async_write;
#[cfg(feature = "futures-io")]
mod futures_input;
#[cfg(feature = "futures-io")]
mod futures_output;
mod input_ref;
mod output_ref;
#[cfg(feature = "tokio")]
mod tokio_async_read;
#[cfg(feature = "tokio")]
mod tokio_async_write;
#[cfg(feature = "tokio")]
mod tokio_input;
#[cfg(feature = "tokio")]
mod tokio_output;

pub use box_input::BoxInput;
pub use box_output::BoxOutput;
#[cfg(feature = "futures-io")]
pub use futures_async_read::FuturesAsyncRead;
#[cfg(feature = "futures-io")]
pub use futures_async_write::FuturesAsyncWrite;
#[cfg(feature = "futures-io")]
pub use futures_input::FuturesInput;
#[cfg(feature = "futures-io")]
pub use futures_output::FuturesOutput;
pub use input_ref::InputRef;
pub use output_ref::OutputRef;
#[cfg(feature = "tokio")]
pub use tokio_async_read::TokioAsyncRead;
#[cfg(feature = "tokio")]
pub use tokio_async_write::TokioAsyncWrite;
#[cfg(feature = "tokio")]
pub use tokio_input::TokioInput;
#[cfg(feature = "tokio")]
pub use tokio_output::TokioOutput;
