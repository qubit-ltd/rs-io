// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

#[cfg(feature = "futures-io")]
mod futures_async_read;
#[cfg(feature = "futures-io")]
mod futures_async_write;
#[cfg(feature = "futures-io")]
mod futures_input;
#[cfg(feature = "futures-io")]
mod futures_output;
#[cfg(feature = "tokio")]
mod tokio_async_read;
#[cfg(feature = "tokio")]
mod tokio_async_write;
#[cfg(feature = "tokio")]
mod tokio_input;
#[cfg(feature = "tokio")]
mod tokio_output;

#[cfg(feature = "futures-io")]
pub use futures_async_read::FuturesAsyncRead;
#[cfg(feature = "futures-io")]
pub use futures_async_write::FuturesAsyncWrite;
#[cfg(feature = "futures-io")]
pub use futures_input::FuturesInput;
#[cfg(feature = "futures-io")]
pub use futures_output::FuturesOutput;
#[cfg(feature = "tokio")]
pub use tokio_async_read::TokioAsyncRead;
#[cfg(feature = "tokio")]
pub use tokio_async_write::TokioAsyncWrite;
#[cfg(feature = "tokio")]
pub use tokio_input::TokioInput;
#[cfg(feature = "tokio")]
pub use tokio_output::TokioOutput;
