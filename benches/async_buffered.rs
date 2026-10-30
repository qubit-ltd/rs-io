// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Runtime-neutral benchmarks for asynchronous buffered item I/O with
//! consumption barriers.
//!
//! Run with `cargo bench --bench async_buffered`. The scripted inner streams
//! model full progress, partial progress, and periodic `Pending` without using
//! an executor. Successful sink writes consume the accepted input range through
//! `black_box`, so results include buffering, polling, and consumption
//! overhead.
//! Each configuration validates transferred lengths and contents before timing.
//! Input measurements include a nonempty EOF probe and any terminal `Pending`.
//! Output recording is used only for untimed validation, not measured writes.

use std::hint::black_box;
use std::io;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;
use std::time::Duration;

use criterion::BatchSize;
use criterion::BenchmarkId;
use criterion::Criterion;
use criterion::Throughput;
use criterion::criterion_group;
use criterion::criterion_main;
use qubit_io::AsyncBufferedInput;
use qubit_io::AsyncBufferedOutput;
use qubit_io::AsyncInput;
use qubit_io::AsyncOutput;

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

const PROGRESS_MODES: [ProgressMode; 3] = [ProgressMode::ReadyFull, ProgressMode::Partial, ProgressMode::Pending];

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
        output[index..index + copied].copy_from_slice(&this.data[this.position..this.position + copied]);
        this.position += copied;
        Poll::Ready(Ok(copied))
    }
}

/// Captures successful prefixes during untimed benchmark validation only.
struct RecordingOutput {
    inner: ScriptedOutput,
    bytes: Vec<u8>,
}

impl AsyncOutput for RecordingOutput {
    type Item = u8;

    unsafe fn poll_write_unchecked(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        let result = Pin::new(&mut this.inner).poll_write(cx, &input[index..index + count]);
        if let Poll::Ready(Ok(written)) = &result {
            this.bytes.extend_from_slice(&input[index..index + *written]);
        }
        result
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().inner).poll_flush(cx)
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
        input: &[u8],
        index: usize,
        count: usize,
    ) -> Poll<io::Result<usize>> {
        let this = self.as_mut().get_mut();
        if this.poll_is_pending() {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        let accepted = count.min(this.progress_limit);
        black_box(&input[index..index + accepted]);
        this.written += accepted;
        Poll::Ready(Ok(accepted))
    }

    /// Completes immediately unless this poll is scheduled to be pending.
    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
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
    (0..DATA_LEN).map(|index| (index as u8).wrapping_mul(31)).collect()
}

/// Polls `input` until EOF and returns the observed caller-side poll counts.
fn read_to_end<I>(input: &mut I, output: &mut [u8], width: usize) -> PollStats
where
    I: AsyncInput<Item = u8> + Unpin,
{
    assert!(width > 0, "benchmark read width must be nonzero");
    let mut context = Context::from_waker(Waker::noop());
    let mut position = 0_usize;
    let mut caller_polls = 0_usize;
    let mut pending_polls = 0_usize;
    let mut eof_probe = [0_u8; 1];
    loop {
        let probing_eof = position == output.len();
        let destination = if probing_eof {
            &mut eof_probe[..]
        } else {
            let end = (position + width).min(output.len());
            &mut output[position..end]
        };
        caller_polls += 1;
        match Pin::new(&mut *input).poll_read(&mut context, destination) {
            Poll::Ready(Ok(0)) => break,
            Poll::Ready(Ok(count)) => {
                assert!(!probing_eof, "benchmark input exceeds the destination capacity");
                position += count;
            }
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
    assert!(width > 0, "benchmark write width must be nonzero");
    let mut context = Context::from_waker(Waker::noop());
    let mut position = 0_usize;
    let mut caller_polls = 0_usize;
    let mut pending_polls = 0_usize;
    while position < input.len() {
        let end = (position + width).min(input.len());
        caller_polls += 1;
        match Pin::new(&mut *output).poll_write(&mut context, &input[position..end]) {
            Poll::Ready(Ok(0)) => panic!("async output benchmark made no write progress"),
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

/// Checks that a full destination does not suppress terminal Pending or EOF.
fn verify_read_terminal_polling() {
    for buffered in [false, true] {
        let source = ScriptedInput {
            data: vec![7],
            position: 0,
            poll_count: 0,
            progress_limit: usize::MAX,
            pending_every: Some(2),
        };
        let mut bytes = [0; 1];
        let (stats, polls) = if buffered {
            let mut input = AsyncBufferedInput::with_capacity(source, BUFFER_CAPACITY);
            let stats = read_to_end(&mut input, &mut bytes, 1);
            let (inner, pending) = input.into_parts();
            assert_eq!(0, pending.available(), "terminal input window must be empty");
            (stats, inner.poll_count())
        } else {
            let mut input = source;
            let stats = read_to_end(&mut input, &mut bytes, 1);
            (stats, input.poll_count())
        };
        assert_eq!([7], bytes);
        assert_eq!(1, stats.bytes);
        assert_eq!(1, stats.pending_polls, "terminal Pending must be observed");
        assert_eq!(3, polls, "data, terminal Pending, and true EOF must reach the source");
    }
    let mut input = ScriptedInput::new(Vec::new(), ProgressMode::ReadyFull);
    let stats = read_to_end(&mut input, &mut [], 1);
    assert_eq!(0, stats.bytes);
    assert_eq!(
        1,
        input.poll_count(),
        "even an empty fixture needs a nonempty EOF probe"
    );
}

/// Validates buffered and unbuffered input results before timing a case.
fn verify_input_case(fixture: &[u8], mode: ProgressMode, width: usize) {
    let mut buffered = AsyncBufferedInput::with_capacity(ScriptedInput::new(fixture.to_vec(), mode), BUFFER_CAPACITY);
    let mut buffered_bytes = vec![0; fixture.len()];
    let stats = read_to_end(&mut buffered, &mut buffered_bytes, width);
    let (source, pending) = buffered.into_parts();
    assert_eq!(fixture.len(), stats.bytes, "buffered input transfer length");
    assert_eq!(fixture, buffered_bytes.as_slice(), "buffered input contents");
    assert_eq!(fixture.len(), source.position, "buffered source consumption");
    assert_eq!(0, pending.available(), "buffered input must have no unread tail");

    let mut source = ScriptedInput::new(fixture.to_vec(), mode);
    let mut bytes = vec![0; fixture.len()];
    let stats = read_to_end(&mut source, &mut bytes, width);
    assert_eq!(fixture.len(), stats.bytes, "unbuffered input transfer length");
    assert_eq!(fixture, bytes.as_slice(), "unbuffered input contents");
    assert_eq!(fixture.len(), source.position, "unbuffered source consumption");
}

/// Validates accepted output contents and final buffering before timing a case.
fn verify_output_case(fixture: &[u8], mode: ProgressMode, width: usize) {
    let recording = RecordingOutput {
        inner: ScriptedOutput::new(mode),
        bytes: Vec::new(),
    };
    let mut buffered = AsyncBufferedOutput::with_capacity(recording, BUFFER_CAPACITY);
    let stats = write_and_flush(&mut buffered, fixture, width);
    let (sink, pending) = buffered.into_parts();
    assert_eq!(fixture.len(), stats.bytes, "buffered output transfer length");
    assert_eq!(fixture.len(), sink.inner.written(), "buffered sink accepted length");
    assert_eq!(fixture, sink.bytes.as_slice(), "buffered output contents");
    assert_eq!(0, pending.available(), "successful flush must drain the output buffer");

    let mut sink = RecordingOutput {
        inner: ScriptedOutput::new(mode),
        bytes: Vec::new(),
    };
    let stats = write_and_flush(&mut sink, fixture, width);
    assert_eq!(fixture.len(), stats.bytes, "unbuffered output transfer length");
    assert_eq!(fixture.len(), sink.inner.written(), "unbuffered sink accepted length");
    assert_eq!(fixture, sink.bytes.as_slice(), "unbuffered output contents");
}

/// Benchmarks asynchronous reads for each scripted progress mode.
fn benchmark_async_input(criterion: &mut Criterion) {
    verify_read_terminal_polling();
    let fixture = benchmark_fixture();
    let mut group = criterion.benchmark_group("async_buffered_input");
    group.measurement_time(Duration::from_secs(2));
    group.sample_size(10);
    group.throughput(Throughput::Bytes(DATA_LEN as u64));

    for mode in PROGRESS_MODES {
        for width in TRANSFER_WIDTHS {
            verify_input_case(&fixture, mode, width);
            let parameter = format!("{}/{}", mode.label(), width);
            group.bench_with_input(BenchmarkId::new("buffered", &parameter), &width, |bencher, &width| {
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
                        let stats = read_to_end(&mut input, &mut output, width);
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
            });
            group.bench_with_input(BenchmarkId::new("unbuffered", &parameter), &width, |bencher, &width| {
                bencher.iter_batched(
                    || (ScriptedInput::new(fixture.clone(), mode), vec![0_u8; DATA_LEN]),
                    |(mut input, mut output)| {
                        let stats = read_to_end(&mut input, &mut output, width);
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
            });
        }
    }
    group.finish();
}

/// Benchmarks asynchronous writes and final flushes for each progress mode.
fn benchmark_async_output(criterion: &mut Criterion) {
    let fixture = benchmark_fixture();
    let mut group = criterion.benchmark_group("async_buffered_output");
    group.measurement_time(Duration::from_secs(2));
    group.sample_size(10);
    group.throughput(Throughput::Bytes(DATA_LEN as u64));

    for mode in PROGRESS_MODES {
        for width in TRANSFER_WIDTHS {
            verify_output_case(&fixture, mode, width);
            let parameter = format!("{}/{}", mode.label(), width);
            group.bench_with_input(BenchmarkId::new("buffered", &parameter), &width, |bencher, &width| {
                bencher.iter_batched(
                    || AsyncBufferedOutput::with_capacity(ScriptedOutput::new(mode), BUFFER_CAPACITY),
                    |mut output| {
                        let stats = write_and_flush(&mut output, &fixture, width);
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
            });
            group.bench_with_input(BenchmarkId::new("unbuffered", &parameter), &width, |bencher, &width| {
                bencher.iter_batched(
                    || ScriptedOutput::new(mode),
                    |mut output| {
                        let stats = write_and_flush(&mut output, &fixture, width);
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
            });
        }
    }
    group.finish();
}

criterion_group!(benches, benchmark_async_input, benchmark_async_output,);
criterion_main!(benches);
