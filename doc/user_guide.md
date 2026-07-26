# Qubit IO User Guide

## 1. Boundary and purpose

`qubit-io` is a small item-transfer abstraction. It exists so higher layers can
compose buffering and codecs without committing their public APIs to
`std::io`, Tokio, or `futures-io`.

The abstraction intentionally stops at transfer:

- `Input` and `AsyncInput` produce items.
- `Output` and `AsyncOutput` accept items and can flush transport buffers.
- `std::io::Error` remains the transport error type.
- File paths, metadata, publication, commit, and abort do not belong here.

## 2. Synchronous traits

`Input` and `Output` use an associated `Item` type. Their unchecked indexed
methods are the implementation boundary; safe methods validate ranges and
reported counts.

```rust
use qubit_io::{Input, Output};

fn copy_once<I, O>(input: &mut I, output: &mut O) -> std::io::Result<usize>
where
    I: Input<Item = u8>,
    O: Output<Item = u8>,
{
    let mut buffer = [0_u8; 1024];
    let read = input.read(&mut buffer)?;
    output.write_fully(&buffer[..read])?;
    Ok(read)
}
```

Blanket implementations adapt standard `Read` and `Write` byte streams. This
does not mean every `Input<u8>` is a file; it only means it is a byte source.
`Input::read_fully` returns the number of items available before EOF, while
`Input::read_exactly` either fills the complete destination or reports
`UnexpectedEof`.

## 3. Asynchronous traits

The asynchronous core is executor-independent:

```text
AsyncInput::poll_read_unchecked
AsyncOutput::poll_write_unchecked
AsyncOutput::poll_flush
AsyncClose::poll_close
```

Core poll methods support `!Unpin` implementations. Convenience extension
methods require `Unpin` and return named futures:

- `read_async`: one input operation;
- `read_fully_async`: fill a destination or stop at EOF;
- `read_exactly_async`: fill a destination or report `UnexpectedEof`;
- `write_async`: one output operation;
- `write_fully_async`: accept the complete source or report `WriteZero`;
- `flush_async`: flush the output.
- `close_async`: close an `AsyncClose` output.

Already pinned `!Unpin` implementations and trait objects use
`PinnedAsyncInputExt` and `PinnedAsyncOutputExt` for the same operations.
`ReadFullyFuture`, `ReadExactFuture`, and `WriteFullyFuture` expose completed
item counts so callers can account for progress after cancellation.

The poll contract is strict: zero-length transfers complete immediately
without polling the inner stream; `Pending` and errors transfer no items;
`Pending` registers the current waker; and `WouldBlock` or `Interrupted` never
cross the async trait boundary. A zero result for a non-empty read is EOF. A
zero result during a full write becomes `WriteZero`. Polling a completed named
future is a caller error and panics.

## 4. Buffering

`Buffer<T>` owns initialized scalar storage and tracks a readable
`position..limit` window. It is shared conceptually by all buffered drivers.

Synchronous buffering:

- `BufferedInput<I>` retains prefetched items and exposes `unread()`.
- `BufferedOutput<O>` accumulates small writes and flushes them to `O`.
- `EnsuredBufferedInput` and `EnsuredBufferedOutput` avoid redundant wrapping.

Asynchronous buffering:

- `AsyncBufferedInput<I>` retains prefetched items across `Pending`.
- `AsyncBufferedOutput<O>` retains accepted items and partial-flush progress.

Consuming synchronous buffers requires an explicit choice:

- `BufferedInput::into_inner()` discards unread prefetched items.
  `try_into_inner()` reports them, while `into_parts()` recovers them.
- `BufferedOutput::into_inner()` flushes and returns `IntoInnerError<Self>` on
  failure, retaining pending items. `try_into_inner()` is a compatibility alias.
  `into_parts()` performs no I/O.

Dropping `BufferedOutput` makes a best-effort flush. Calling
`IntoInnerError::into_error()` drops its retained output and can therefore
trigger that attempt; recover the output with `into_inner()`, `into_writer()`,
or `into_parts()` when pending data must stay under caller control.

`AsyncBufferedInput` exposes `into_parts()` rather than an `into_inner()` that
could silently discard unread prefetched items. When its inner output supports
`AsyncClose`, `AsyncBufferedOutput` drains its own buffer before closing it.

An asynchronous `Drop` cannot await. `AsyncBufferedOutput` therefore never
pretends drop-time delivery succeeded. Complete `flush_async()` before drop, or
use `into_parts()` to recover the inner output and pending `Buffer`.

## 5. Limit, counting, and checksum wrappers

The asynchronous wrappers implement the poll traits directly and can contain a
`!Unpin` inner stream:

- `AsyncLimitInput` / `AsyncLimitOutput` expose at most a configured item count.
- `AsyncCountingInput` / `AsyncCountingOutput` count successful ready results.
- `AsyncChecksumInput` / `AsyncChecksumOutput` hash successful byte transfers.

Counts and hashes do not change on `Pending` or errors. Checksum wrappers hash
only the prefix actually reported by the inner stream.

The synchronous wrappers are item-oriented: `LimitInput` / `LimitOutput`,
`CountingInput` / `CountingOutput`, and tee wrappers work with any item type.
`ChecksumInput` and `ChecksumOutput` remain byte-only because `Hasher`
consumes bytes. Standard `Read` and `Write` values can be used as byte inputs
and outputs through the blanket implementations.

## 6. Tokio and futures-io bridges

Async bridges are explicit newtypes in both directions:

| External ecosystem to Qubit | Qubit to external ecosystem |
| --- | --- |
| `TokioInput`, `TokioOutput` | `TokioAsyncRead`, `TokioAsyncWrite` |
| `FuturesInput`, `FuturesOutput` | `FuturesAsyncRead`, `FuturesAsyncWrite` |

Explicit wrappers avoid overlapping blanket implementations. The runtime-
neutral core has no optional dependency enabled by default.

Closing is never emulated by flushing. `TokioOutput` delegates to
`poll_shutdown`, `FuturesOutput` delegates to `poll_close`, and reverse write
adapters require `AsyncClose<Item = u8>`.

## 7. Layering guidance

- Use `qubit-io` for transport and generic buffering.
- Use `qubit-io-binary` for typed binary values.
- Use `qubit-io-text` for Unicode text and charset conversion.
- Use `qubit-fs` when a byte stream must also carry file identity and a
  commit/abort lifecycle.

Keep codecs independent from the synchronous or asynchronous driver. The same
codec state should be driven by `Input`/`Output` or
`AsyncInput`/`AsyncOutput`, not implemented twice.

## 8. Migrating from 0.13

Version 0.14 renamed the synchronous `Reader`/`Writer` wrappers to
`Input`/`Output` wrappers and changed their implemented traits accordingly.
See the [0.14 migration guide](migration-0.14.md) for the type mapping, method
changes, and tee failure semantics.
