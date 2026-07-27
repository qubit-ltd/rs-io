// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Runtime-neutral benchmarks for asynchronous buffered item I/O.
//!
//! Run with `cargo bench --bench async_buffered`. The scripted inner streams
//! model full progress, partial progress, and periodic `Pending` without using
//! an executor, so the results isolate buffering and polling overhead.

use std::hint::black_box;
use std::io;
use std::pin::Pin;
use std::task::{
    Context,
    Poll,
    Waker,
};

use criterion::{
    BatchSize,
    BenchmarkId,
    Criterion,
    Throughput,
    criterion_group,
    criterion_main,
};
use qubit_io::{
    AsyncBufferedInput,
    AsyncBufferedOutput,
    AsyncInput,
    AsyncOutput,
};

const BUFFER_CAPACITY: usize = 8 * 1024;
const DATA_LEN: usize = 256 * 1024;
const PARTIAL_PROGRESS: usize = 256;
const PENDING_EVERY: usize = 3;
const TRANSFER_WIDTHS: [usize; 3] = [64, BUFFER_CAPACITY, BUFFER_CAPACITY + 1];

/// Describes the scripted progress behavior of a benchmark inner stream.
#[derive(Clone, Copy)]
enum ProgressMode {
    /// Every poll completes the requested transfer when input remains.
    ReadyFull,
    /// Every ready poll completes at most `PARTIAL_PROGRESS` items.
    Partial,
    /// Every `PENDING_EVERY`th poll returns `Poll::Pending`.
    Pending,
}

impl ProgressMode {
    /// Returns the stable label used in Criterion benchmark identifiers.
    fn label(self) -> &'static str {
        match self {
            Self::ReadyFull => "ready_full",
            Self::Partial => "partial",
            Self::Pending => "pending",
        }
    }

    /// Returns the maximum number of items a ready poll accepts or yields.
    fn progress_limit(self) -> usize {
        match self {
            Self::Partial => PARTIAL_PROGRESS,
            Self::ReadyFull | Self::Pending => usize::MAX,
        }
    }

    /// Returns the poll cadence that produces pending results, if any.
    fn pending_every(self) -> Option<usize> {
        match self {
            Self::Pending => Some(PENDING_EVERY),
            Self::ReadyFull | Self::Partial => None,
        }
    }
}

const PROGRESS_MODES: [ProgressMode; 3] = [
    ProgressMode::ReadyFull,
    ProgressMode::Partial,
    ProgressMode::Pending,
];

/// Captures useful polling counters so benchmark work cannot be optimized away.
struct PollStats {
    /// Number of stream items transferred by the driver.
    bytes: usize,
    /// Number of outer trait polls attempted by the driver.
    caller_polls: usize,
    /// Number of outer polls that returned `Poll::Pending`.
    pending_polls: usize,
}

/// Supplies deterministic bytes to an `AsyncInput` implementation.
struct ScriptedInput {
    /// Source bytes returned by ready polls.
    data: Vec<u8>,
    /// Index of the next unread source byte.
    position: usize,
    /// Total inner polls observed by the scripted source.
    poll_count: usize,
    /// Maximum bytes returned by a ready poll.
    progress_limit: usize,
    /// Optional cadence for returning pending polls.
    pending_every: Option<usize>,
}

impl ScriptedInput {
    /// Creates a source that returns `data` according to `mode`.
    fn new(data: Vec<u8>, mode: ProgressMode) -> Self {
        Self {
            data,
            position: 0,
            poll_count: 0,
            progress_limit: mode.progress_limit(),
            pending_every: mode.pending_every(),
        }
    }

    /// Returns the number of polls received by the inner source.
    fn poll_count(&self) -> usize {
        self.poll_count
    }

    /// Records a poll and reports whether it should yield `Poll::Pending`.
    fn poll_is_pending(&mut self) -> bool {
        self.poll_count += 1;
        self.pending_every
            .is_some_and(|every| self.poll_count.is_multiple_of(every))
    }
}

impl AsyncInput for ScriptedInput {
    type Item = u8;

    /// Copies available source items into the valid caller-provided range.
    unsafe fn poll_read_unchecked(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut [u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        let this = self.as_mut().get_mut();
        if this.poll_is_pending() {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        let available = this.data.len() - this.position;
        let copied = count.min(available).min(this.progress_limit);
        output[index..index + copied]
            .copy_from_slice(&this.data[this.position..this.position + copied]);
        this.position += copied;
        Poll::Ready(Ok(copied))
    }
}

/// Records bytes supplied to an `AsyncOutput` implementation.
struct ScriptedOutput {
    /// Number of accepted bytes.
    written: usize,
    /// Total inner polls observed by the scripted sink.
    poll_count: usize,
    /// Maximum bytes accepted by a ready poll.
    progress_limit: usize,
    /// Optional cadence for returning pending polls.
    pending_every: Option<usize>,
}

impl ScriptedOutput {
    /// Creates a sink that accepts writes according to `mode`.
    fn new(mode: ProgressMode) -> Self {
        Self {
            written: 0,
            poll_count: 0,
            progress_limit: mode.progress_limit(),
            pending_every: mode.pending_every(),
        }
    }

    /// Returns the number of polls received by the inner sink.
    fn poll_count(&self) -> usize {
        self.poll_count
    }

    /// Returns the number of bytes accepted by ready write polls.
    fn written(&self) -> usize {
        self.written
    }

    /// Records a poll and reports whether it should yield `Poll::Pending`.
    fn poll_is_pending(&mut self) -> bool {
        self.poll_count += 1;
        self.pending_every
            .is_some_and(|every| self.poll_count.is_multiple_of(every))
    }
}

impl AsyncOutput for ScriptedOutput {
    type Item = u8;

    /// Accepts a prefix of the valid caller-provided input range.
    unsafe fn poll_write_unchecked(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        _input: &[u8],
        _index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        let this = self.as_mut().get_mut();
        if this.poll_is_pending() {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        let accepted = count.min(this.progress_limit);
        this.written += accepted;
        Poll::Ready(Ok(accepted))
    }

    /// Completes immediately unless this poll is scheduled to be pending.
    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        if self.as_mut().get_mut().poll_is_pending() {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(Ok(()))
        }
    }
}

/// Creates deterministic byte input shared by the asynchronous benchmarks.
fn benchmark_fixture() -> Vec<u8> {
    (0..DATA_LEN)
        .map(|index| (index as u8).wrapping_mul(31))
        .collect()
}

/// Polls `input` until EOF and returns the observed caller-side poll counts.
fn read_to_end<I>(input: &mut I, output: &mut [u8], width: usize) -> PollStats
where
    I: AsyncInput<Item = u8> + Unpin,
{
    let mut context = Context::from_waker(Waker::noop());
    let mut position = 0_usize;
    let mut caller_polls = 0_usize;
    let mut pending_polls = 0_usize;
    loop {
        let end = (position + width).min(output.len());
        caller_polls += 1;
        match Pin::new(&mut *input)
            .poll_read(&mut context, &mut output[position..end])
        {
            Poll::Ready(Ok(0)) => break,
            Poll::Ready(Ok(count)) => position += count,
            Poll::Ready(Err(error)) => {
                panic!("async input benchmark read failed: {error}")
            }
            Poll::Pending => pending_polls += 1,
        }
    }
    PollStats {
        bytes: position,
        caller_polls,
        pending_polls,
    }
}

/// Polls `output` until all input is written and the output is flushed.
fn write_and_flush<O>(output: &mut O, input: &[u8], width: usize) -> PollStats
where
    O: AsyncOutput<Item = u8> + Unpin,
{
    let mut context = Context::from_waker(Waker::noop());
    let mut position = 0_usize;
    let mut caller_polls = 0_usize;
    let mut pending_polls = 0_usize;
    while position < input.len() {
        let end = (position + width).min(input.len());
        caller_polls += 1;
        match Pin::new(&mut *output)
            .poll_write(&mut context, &input[position..end])
        {
            Poll::Ready(Ok(count)) => position += count,
            Poll::Ready(Err(error)) => {
                panic!("async output benchmark write failed: {error}")
            }
            Poll::Pending => pending_polls += 1,
        }
    }
    loop {
        caller_polls += 1;
        match Pin::new(&mut *output).poll_flush(&mut context) {
            Poll::Ready(Ok(())) => break,
            Poll::Ready(Err(error)) => {
                panic!("async output benchmark flush failed: {error}")
            }
            Poll::Pending => pending_polls += 1,
        }
    }
    PollStats {
        bytes: position,
        caller_polls,
        pending_polls,
    }
}

/// Benchmarks asynchronous reads for each scripted progress mode.
fn benchmark_async_input(criterion: &mut Criterion) {
    let fixture = benchmark_fixture();
    let mut group = criterion.benchmark_group("async_buffered_input");
    group.measurement_time(std::time::Duration::from_secs(2));
    group.sample_size(10);
    group.throughput(Throughput::Bytes(DATA_LEN as u64));

    for mode in PROGRESS_MODES {
        for width in TRANSFER_WIDTHS {
            let parameter = format!("{}/{}", mode.label(), width);
            group.bench_with_input(
                BenchmarkId::new("buffered", &parameter),
                &width,
                |bencher, &width| {
                    bencher.iter_batched(
                        || {
                            (
                                AsyncBufferedInput::with_capacity(
                                    ScriptedInput::new(fixture.clone(), mode),
                                    BUFFER_CAPACITY,
                                ),
                                vec![0_u8; DATA_LEN],
                            )
                        },
                        |(mut input, mut output)| {
                            let stats =
                                read_to_end(&mut input, &mut output, width);
                            let (inner, pending) = input.into_parts();
                            black_box((
                                stats.bytes,
                                stats.caller_polls,
                                stats.pending_polls,
                                inner.poll_count(),
                                pending.available(),
                                output,
                            ));
                        },
                        BatchSize::LargeInput,
                    );
                },
            );
            group.bench_with_input(
                BenchmarkId::new("unbuffered", &parameter),
                &width,
                |bencher, &width| {
                    bencher.iter_batched(
                        || {
                            (
                                ScriptedInput::new(fixture.clone(), mode),
                                vec![0_u8; DATA_LEN],
                            )
                        },
                        |(mut input, mut output)| {
                            let stats =
                                read_to_end(&mut input, &mut output, width);
                            black_box((
                                stats.bytes,
                                stats.caller_polls,
                                stats.pending_polls,
                                input.poll_count(),
                                output,
                            ));
                        },
                        BatchSize::LargeInput,
                    );
                },
            );
        }
    }
    group.finish();
}

/// Benchmarks asynchronous writes and final flushes for each progress mode.
fn benchmark_async_output(criterion: &mut Criterion) {
    let fixture = benchmark_fixture();
    let mut group = criterion.benchmark_group("async_buffered_output");
    group.measurement_time(std::time::Duration::from_secs(2));
    group.sample_size(10);
    group.throughput(Throughput::Bytes(DATA_LEN as u64));

    for mode in PROGRESS_MODES {
        for width in TRANSFER_WIDTHS {
            let parameter = format!("{}/{}", mode.label(), width);
            group.bench_with_input(
                BenchmarkId::new("buffered", &parameter),
                &width,
                |bencher, &width| {
                    bencher.iter_batched(
                        || {
                            AsyncBufferedOutput::with_capacity(
                                ScriptedOutput::new(mode),
                                BUFFER_CAPACITY,
                            )
                        },
                        |mut output| {
                            let stats =
                                write_and_flush(&mut output, &fixture, width);
                            let (inner, pending) = output.into_parts();
                            black_box((
                                stats.bytes,
                                stats.caller_polls,
                                stats.pending_polls,
                                inner.poll_count(),
                                inner.written(),
                                pending.available(),
                            ));
                        },
                        BatchSize::SmallInput,
                    );
                },
            );
            group.bench_with_input(
                BenchmarkId::new("unbuffered", &parameter),
                &width,
                |bencher, &width| {
                    bencher.iter_batched(
                        || ScriptedOutput::new(mode),
                        |mut output| {
                            let stats =
                                write_and_flush(&mut output, &fixture, width);
                            black_box((
                                stats.bytes,
                                stats.caller_polls,
                                stats.pending_polls,
                                output.poll_count(),
                                output.written(),
                            ));
                        },
                        BatchSize::SmallInput,
                    );
                },
            );
        }
    }
    group.finish();
}

criterion_group!(benches, benchmark_async_input, benchmark_async_output,);
criterion_main!(benches);
