# Qubit IO User Guide

## 1. What this crate does

`qubit-io` is a runtime-neutral, item-oriented transfer layer. It lets a
codec, buffer, or wrapper operate on a stream without choosing `std::io`,
Tokio, or `futures-io` as its public abstraction.

The crate deliberately models transfer only. It does not represent file paths,
file identity, metadata, publication, commit, abort, or persistence. Use
`qubit-fs` when those lifecycle semantics are required, `qubit-io-binary` for
typed binary values, and `qubit-io-text` for text and character encodings.

| Need | Synchronous API | Asynchronous API |
| --- | --- | --- |
| Move items | `Input`, `Output` | `AsyncInput`, `AsyncOutput` |
| Flush or close | `Output::flush` | `AsyncOutput::flush_async`, `AsyncClose::close_async` |
| Add a buffer | `BufferedInput`, `BufferedOutput` | `AsyncBufferedInput`, `AsyncBufferedOutput` |
| Constrain or observe transfer | limit, counting, checksum, tee wrappers | async limit, counting, checksum wrappers |
| Interoperate with another ecosystem | blanket `std::io` implementations | optional Tokio and `futures-io` newtypes |

The public types are re-exported from the crate root. Internal modules are not
part of the compatibility boundary.

## 2. Add the dependency and select features

The default feature set contains the runtime-neutral core only:

```toml
[dependencies]
qubit-io = "0.14"
```

Enable an adapter only when the application uses that ecosystem:

```toml
[dependencies]
qubit-io = { version = "0.14", features = ["tokio"] }
```

`tokio` enables `TokioInput`, `TokioOutput`, `TokioAsyncRead`, and
`TokioAsyncWrite`. `futures-io` enables the corresponding `Futures*` types.
The core traits do not select an executor and do not require either feature.

## 3. Synchronous item transfer

`Input` produces `Item` values and `Output` accepts them. Their safe methods
validate the count returned by the implementation; implementers provide the
unchecked indexed operation only when they can uphold its documented range
contract. Most applications consume the safe methods and do not implement the
traits directly.

All `std::io::Read` values implement `Input<Item = u8>`, and all
`std::io::Write` values implement `Output<Item = u8>`. That is a byte-stream
adapter, not a claim that every input is a file.

```rust
use std::io::{Cursor, Result};
use qubit_io::{Input, Output};

fn main() -> Result<()> {
    let mut input = Cursor::new(b"qubit".to_vec());
    let mut bytes = [0_u8; 5];
    input.read_exactly(&mut bytes)?;

    let mut output = Vec::new();
    output.write_fully(&bytes)?;
    assert_eq!(b"qubit", output.as_slice());
    Ok(())
}
```

`read` and `write` perform one operation and may make partial progress.
`read_fully` stops at EOF and returns the number transferred;
`read_exactly` returns `UnexpectedEof` unless it fills the destination.
`write_fully` retries interrupted writes and returns `WriteZero` when the
output reports zero progress before all items are accepted. `flush` asks an
`Output` to deliver its internally buffered items; it is not a close operation.

The item type is generic. Wrapper families such as limit, counting, and tee
can therefore operate on inexpensive scalar items other than bytes. Checksum
wrappers are intentionally byte-only because `std::hash::Hasher` consumes
bytes.

### Seeking and composite traits

`Seekable` is the item-oriented counterpart of `std::io::Seek`; its positions
are measured in the wrapped stream's unit. Standard `Seek` values use `u8`.
`SeekableInput`, `SeekableOutput`, `ReadSeek`, `WriteSeek`, `ReadWrite`, and
`ReadWriteSeek` express useful trait combinations without introducing new
behavior. `PositionGuard` records a `Seekable` position and restores it on drop
unless the guard is dismissed; call `restore` to observe a restoration error.

## 4. `Buffer<T>` and synchronous buffering

`Buffer<T>` owns initialized `Copy + Default` storage and tracks a readable
window `position..limit`. `readable()` exposes the queued items,
`spare_mut()` exposes free initialized storage, and `available()` and
`spare_capacity()` report the two lengths. Its state-changing low-level methods
are `unsafe`: the caller must prove the requested range fits. It is primarily a
building block for buffered drivers and specialized encoders.

```rust
use qubit_io::Buffer;

fn main() {
    let source = [10_u8, 20, 30];
    let mut buffer = Buffer::with_capacity(4);

    // SAFETY: `source[0..3]` is valid and the new buffer has four spare slots.
    unsafe { buffer.copy_from(&source, 0, source.len()) };
    assert_eq!(&[10, 20, 30], buffer.readable());

    // SAFETY: three items are readable, so consuming two stays in range.
    unsafe { buffer.consume(2) };
    assert_eq!(&[30], buffer.readable());
}
```

`BufferedInput<I>` prefetches input items. Use `fill_more`, `fill_until`, or
`ensure_available` before reading its unread window manually; then call
`consume` exactly once for the items consumed. `BufferedOutput<O>` accumulates
small writes and flushes when required. Both default to
`DEFAULT_BUFFER_CAPACITY`; `with_capacity` clamps a zero capacity to one, while
`try_with_capacity` and `try_reserve_capacity` report allocation failure.

```rust
use std::io::{Cursor, Result};
use qubit_io::BufferedInput;

fn main() -> Result<()> {
    let mut input = BufferedInput::with_capacity(
        Cursor::new(b"abcdef".to_vec()),
        4,
    );
    assert!(input.fill_until(3)?);
    assert_eq!(b"abcd", input.unread());

    // SAFETY: `fill_until(3)` succeeded and four items are buffered.
    unsafe { input.consume(2) };
    let (_inner, unread) = input.into_parts();
    assert_eq!(b"cd", unread.readable());
    Ok(())
}
```

`into_parts()` makes ownership decisions explicit. For input, it returns both
the inner input and its unread `Buffer`; consume that readable window before
reading from the inner input again. For output, it performs no I/O and returns
the inner output with its pending `Buffer`. Call `flush` first for normal
completion; if flushing fails, the wrapper remains owned and can be retried.

Dropping a synchronous `BufferedOutput` makes a best-effort flush. Do not use
drop as a delivery guarantee; explicitly `flush`, then use `into_parts` when
ownership of the output is needed.

```rust
use std::io::Result;
use qubit_io::BufferedOutput;

fn main() -> Result<()> {
    let mut output = BufferedOutput::with_capacity(Vec::<u8>::new(), 4);
    output.write_fully(b"abc")?;
    output.flush()?;

    let (inner, pending) = output.into_parts();
    assert_eq!(b"abc", inner.as_slice());
    assert!(pending.is_empty());
    Ok(())
}
```

`BufferedInput::ensure` and `BufferedOutput::ensure` avoid another Qubit
buffer only when `is_buffered()` says the value is buffered. They cannot detect
`std::io::BufReader` or `BufWriter`, because those enter through the blanket
`Read` and `Write` implementations. Do not pass an already standard-buffered
stream to `ensure`.

## 5. Wrappers and composition

Wrappers forward the underlying contract while adding one focused policy:

| Wrapper family | Meaning | Important boundary |
| --- | --- | --- |
| `LimitInput` / `LimitOutput` | exposes at most a remaining item count | reaching zero behaves as EOF for input or accepts no more output |
| `CountingInput` / `CountingOutput` | saturating count of successful items | `bytes_*` is available for `u8`; `items_*` works for every item type |
| `ChecksumInput` / `ChecksumOutput` | hashes successful byte prefixes | only `u8`; `Pending` and errors do not update it |
| `TeeInput` / `TeeOutput` | mirrors source or primary transfer to a branch | ordered and non-transactional |
| `SyncSeekTeeInput` | mirrors reads and synchronizes seeks | source is changed before a branch failure can be returned |

`inner_mut()` and `branch_mut()` intentionally bypass wrapper bookkeeping.
Reads and writes through such accessors are not counted, limited, hashed, or
mirrored. Use them only when that is the desired escape hatch.

```rust
use std::io::Result;
use qubit_io::{CountingOutput, Output, TeeOutput};

fn main() -> Result<()> {
    let tee = TeeOutput::new(Vec::<u8>::new(), Vec::<u8>::new());
    let mut output = CountingOutput::new(tee);
    output.write_fully(b"copy")?;
    assert_eq!(4, output.bytes_written());

    let (primary, branch) = output.into_inner().into_parts();
    assert_eq!(b"copy", primary.as_slice());
    assert_eq!(b"copy", branch.as_slice());
    Ok(())
}
```

Tee writes update the primary output before the branch. Tee reads advance the
source before writing the branch. Flushes and synchronized seeks likewise act
on the primary or source first. A later error never rolls back earlier work;
add a transaction or recovery layer above Qubit IO when atomic replication is
required.

## 6. Standard I/O extensions and `Streams`

The extension traits operate on standard byte streams and add explicit resource
limits. `ReadExt` offers exact-or-EOF reads, bounded vectors and strings,
bounded copy operations, and discard helpers. `BufReadExt` adds bounded line
and delimiter reads. `ReadSeekExt`, `SeekExt`, and `WriteSeekExt` offer
position-preserving operations. The unchecked extension methods have the same
range obligations as their names indicate.

`Streams` is a non-constructible namespace. Its `copy_input_to_output*`
methods work with generic Qubit items, while `copy*` and comparison methods
work with `std::io` byte streams.

```rust
use std::io::{Cursor, Result};
use qubit_io::{BufReadExt, Streams};

fn main() -> Result<()> {
    let mut input = Cursor::new(b"abcdef".to_vec());
    let mut output = Vec::new();
    let copied = Streams::copy_input_to_output_at_most(
        &mut input,
        &mut output,
        3,
    )?;
    assert_eq!(3, copied);
    assert_eq!(b"abc", output.as_slice());

    let mut line_input = Cursor::new(b"hello\nrest".to_vec());
    assert_eq!("hello\n", line_input.read_line_limited(6)?);
    Ok(())
}
```

For data whose size is controlled by another party, use a `*_limited` method
instead of unbounded `read_to_end`, `read_to_string`, or delimiter reads. The
limit is part of the caller's resource policy, not merely a convenience value.

## 7. Asynchronous contract

`AsyncInput` and `AsyncOutput` use `Pin`, `Context`, and `Poll`, but do not
depend on a runtime. Implementers provide `poll_read_unchecked` or
`poll_write_unchecked`; consumers usually call `read_async`,
`read_fully_async`, `read_exactly_async`, `write_async`, `write_fully_async`,
and `flush_async`.

The contract is strict:

- A zero-length transfer completes without polling the inner stream.
- `Poll::Pending` and errors transfer no items; `Pending` must register the
  current waker.
- `WouldBlock` and `Interrupted` must not cross this boundary.
- A non-empty read returning zero is EOF; a full write returning zero becomes
  `WriteZero`.
- Polling a named operation future after it completed is a caller error and
  panics.

`ReadFuture`, `ReadFullyFuture`, `ReadExactFuture`, `WriteFuture`,
`WriteFullyFuture`, `FlushFuture`, and `CloseFuture` keep multi-poll state in
the future itself. Before dropping a pending `ReadFullyFuture`,
`ReadExactFuture`, or `WriteFullyFuture`, inspect `items_read()` or
`items_written()` if recovery needs an exact progress count. Pinned `!Unpin`
values and trait objects use `PinnedAsyncInputExt` and
`PinnedAsyncOutputExt` instead of the `Unpin` convenience methods.

`AsyncClose` is separate from flush. A close represents a real transport
shutdown and is exposed through `close_async`.

## 8. Asynchronous buffering and adapters

`AsyncBufferedInput` retains prefetched items across `Pending` and exposes
`poll_fill_more`, `poll_fill_until`, and `poll_ensure_available` for manual
window management. `AsyncBufferedOutput` retains every accepted item until the
inner output accepts it; partial flushes update retained progress before
returning `Pending`. Both provide `try_with_capacity` and
`try_reserve_capacity` for allocation-aware construction.

Asynchronous destructors cannot perform I/O. Call `flush_async` to guarantee a
flush, or `into_parts` to recover the inner stream and pending buffer.
`AsyncBufferedOutput` drains its own pending items before delegating
`AsyncClose` to an inner output that supports close.

The Tokio adapters are explicit newtypes to avoid overlapping ecosystem trait
implementations. The following complete program writes through an
`AsyncBufferedOutput`, closes it, and reads through an `AsyncBufferedInput`.

```toml
[dependencies]
qubit-io = { version = "0.14", features = ["tokio"] }
tokio = { version = "1", features = ["macros", "rt", "io-util"] }
```

```rust
use std::io;
use qubit_io::{
    AsyncBufferedInput, AsyncBufferedOutput, AsyncClose, AsyncInput,
    AsyncOutput, TokioInput, TokioOutput,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let (writer, reader) = tokio::io::duplex(64);
    let mut output = AsyncBufferedOutput::with_capacity(TokioOutput::new(writer), 4);
    output.write_fully_async(b"qubit").await?;
    output.close_async().await?;

    let mut input = AsyncBufferedInput::with_capacity(TokioInput::new(reader), 4);
    let mut received = [0_u8; 5];
    input.read_exactly_async(&mut received).await?;
    assert_eq!(b"qubit", &received);
    Ok(())
}
```

`TokioInput` and `TokioOutput` adapt Tokio to Qubit IO; `TokioAsyncRead` and
`TokioAsyncWrite` expose Qubit byte streams to Tokio. `FuturesInput`,
`FuturesOutput`, `FuturesAsyncRead`, and `FuturesAsyncWrite` provide the same
two directions for `futures-io`. Tokio close delegates to `poll_shutdown` and
the futures-io close delegates to `poll_close`; neither is emulated with a
flush.

## 9. Choosing and recovering the right owner

Use this checklist when composing stream layers:

1. Pick `Input`/`Output` for blocking transfer and the async traits only when
   the caller already owns an async execution path.
2. Add a buffer at the outermost layer that benefits from batching. Avoid
   wrapping a standard `BufReader` or `BufWriter` through `ensure`.
3. Put a limit closest to untrusted input or the output quota it protects.
4. Place counters and checksums according to the bytes or items that must be
   observed, not simply according to construction convenience.
5. Flush or close explicitly. When an operation fails, retain the wrapper and
   retry or inspect it; use `into_parts` only when transferring responsibility
   for pending data is intentional.

The API documentation on docs.rs describes every method and its exact error,
panic, ownership, and pinning constraints. This guide explains how those APIs
fit together in an application.
