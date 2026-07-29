// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

//! Future implementations backing the asynchronous I/O convenience methods.

mod close_future;
mod flush_future;
mod read_exact_future;
mod read_fully_future;
mod read_future;
mod write_fully_future;
mod write_future;

pub use close_future::CloseFuture;
pub use flush_future::FlushFuture;
pub use read_exact_future::ReadExactFuture;
pub use read_fully_future::ReadFullyFuture;
pub use read_future::ReadFuture;
pub use write_fully_future::WriteFullyFuture;
pub use write_future::WriteFuture;

pub(crate) use read_fully_future::MAX_READY_OPERATIONS_PER_POLL;
