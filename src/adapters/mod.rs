// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Adapters between Qubit I/O traits and boxed, borrowed, futures-io, or Tokio
//! values.

mod box_async_input;
mod box_async_output;
mod box_input;
mod box_output;
#[cfg(all(feature = "futures-io", not(miri)))]
mod futures_async_read;
#[cfg(all(feature = "futures-io", not(miri)))]
mod futures_async_write;
#[cfg(all(feature = "futures-io", not(miri)))]
mod futures_input;
#[cfg(all(feature = "futures-io", not(miri)))]
mod futures_output;
mod input_ref;
mod output_ref;
#[cfg(all(feature = "tokio", not(miri)))]
mod tokio_async_read;
#[cfg(all(feature = "tokio", not(miri)))]
mod tokio_async_write;
#[cfg(all(feature = "tokio", not(miri)))]
mod tokio_input;
#[cfg(all(feature = "tokio", not(miri)))]
mod tokio_output;

pub use box_async_input::BoxAsyncInput;
pub use box_async_output::BoxAsyncOutput;
pub use box_input::BoxInput;
pub use box_output::BoxOutput;
#[cfg(all(feature = "futures-io", not(miri)))]
pub use futures_async_read::FuturesAsyncRead;
#[cfg(all(feature = "futures-io", not(miri)))]
pub use futures_async_write::FuturesAsyncWrite;
#[cfg(all(feature = "futures-io", not(miri)))]
pub use futures_input::FuturesInput;
#[cfg(all(feature = "futures-io", not(miri)))]
pub use futures_output::FuturesOutput;
pub use input_ref::InputRef;
pub use output_ref::OutputRef;
#[cfg(all(feature = "tokio", not(miri)))]
pub use tokio_async_read::TokioAsyncRead;
#[cfg(all(feature = "tokio", not(miri)))]
pub use tokio_async_write::TokioAsyncWrite;
#[cfg(all(feature = "tokio", not(miri)))]
pub use tokio_input::TokioInput;
#[cfg(all(feature = "tokio", not(miri)))]
pub use tokio_output::TokioOutput;
