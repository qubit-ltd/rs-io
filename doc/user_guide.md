# Qubit IO User Guide

## Contents

1. [Start with the boundary](#1-start-with-the-boundary)
2. [Add the dependency and select features](#2-add-the-dependency-and-select-features)
3. [Case study: a bounded length-prefixed frame](#3-case-study-a-bounded-length-prefixed-frame)
4. [Keep the async driver runtime-neutral](#4-keep-the-async-driver-runtime-neutral)
5. [Compose transfer policies around the decoder](#5-compose-transfer-policies-around-the-decoder)
6. [Recover ownership after partial progress](#6-recover-ownership-after-partial-progress)
7. [Case study: Map/Reduce records without byte plumbing](#7-case-study-mapreduce-records-without-byte-plumbing)
8. [Standard I/O, seek, and advanced tools](#8-standard-io-seek-and-advanced-tools)
9. [Choose the narrowest boundary](#9-choose-the-narrowest-boundary)

## 1. Start with the boundary

Use Qubit IO when your library owns a transfer algorithm but must not own its caller's runtime, transport, or item type. A codec can depend on `Input<Item = u8>` or `AsyncInput<Item = u8>`; the application decides whether the bytes come from a standard reader, Tokio, or `futures-io`.

This crate deliberately stops at transfer. It does not model paths, file identity, metadata, publication, commit, abort, or persistence. Use [qubit-fs](https://docs.rs/qubit-fs) for those lifecycle semantics, [qubit-io-binary](https://docs.rs/qubit-io-binary) for typed byte values, and [qubit-io-text](https://docs.rs/qubit-io-text) for text encoding.

The two case studies in this guide answer different questions:

- A length-prefixed frame shows why an async library boundary should not select Tokio or `futures-io`.
- A Map/Reduce mapper shows why an item stream is more than a renamed byte stream.

## 2. Add the dependency and select features

The default feature set contains the runtime-neutral core:

```toml
[dependencies]
qubit-io = "0.16"
```

Enable an adapter only in an application that uses that ecosystem:

```toml
[dependencies]
qubit-io = { version = "0.16", features = ["tokio"] }
```

`tokio` enables `TokioInput`, `TokioOutput`, `TokioAsyncRead`, and `TokioAsyncWrite`. `futures-io` enables the matching `Futures*` types. The core traits neither select an executor nor require either feature.

The adapter names describe the destination interface:

| Existing value | Exposed as | Tokio adapter | futures-io adapter |
| --- | --- | --- | --- |
| ecosystem `AsyncRead` | Qubit `AsyncInput<Item = u8>` | `TokioInput` | `FuturesInput` |
| ecosystem `AsyncWrite` | Qubit `AsyncOutput<Item = u8>` | `TokioOutput` | `FuturesOutput` |
| Qubit `AsyncInput<Item = u8>` | ecosystem `AsyncRead` | `TokioAsyncRead` | `FuturesAsyncRead` |
| Qubit `AsyncOutput<Item = u8>` | ecosystem `AsyncWrite` | `TokioAsyncWrite` | `FuturesAsyncWrite` |

## 3. Case study: a bounded length-prefixed frame

The protocol below has a four-byte big-endian length header followed by that many bytes of payload. It accepts an empty payload, rejects a declared payload over 64 KiB before allocating, and treats truncated input as an error. The complete checked program is [examples/bounded_frame.rs](../examples/bounded_frame.rs).

```rust
use std::io::{self, Error, ErrorKind};
use qubit_io::Input;

const MAX_FRAME_LEN: usize = 64 * 1024;

fn read_frame<I>(input: &mut I) -> io::Result<Vec<u8>>
where
    I: Input<Item = u8>,
{
    let mut header = [0_u8; 4];
    input.read_exactly(&mut header)?;
    let length = u32::from_be_bytes(header) as usize;
    if length > MAX_FRAME_LEN {
        return Err(Error::new(ErrorKind::InvalidData, "frame is too large"));
    }

    let mut payload = vec![0_u8; length];
    input.read_exactly(&mut payload)?;
    Ok(payload)
}
```

`Input::read_exactly` converts an incomplete header or payload into `UnexpectedEof`. The explicit length check is still required even when an outer wrapper limits a connection: a transport quota and a protocol rule protect different boundaries.

All `std::io::Read` values implement `Input<Item = u8>`, so a command-line program can call `read_frame` with a `Cursor` during tests or a `File` in production. The decoder has no file-specific behavior.

## 4. Keep the async driver runtime-neutral

The asynchronous entry point has the same protocol rule but uses `AsyncInput`:

```rust
use std::io::{self, Error, ErrorKind};
use qubit_io::AsyncInput;

const MAX_FRAME_LEN: usize = 64 * 1024;

async fn read_frame_async<I>(input: &mut I) -> io::Result<Vec<u8>>
where
    I: AsyncInput<Item = u8> + Unpin,
{
    let mut header = [0_u8; 4];
    input.read_exactly_async(&mut header).await?;
    let length = u32::from_be_bytes(header) as usize;
    if length > MAX_FRAME_LEN {
        return Err(Error::new(ErrorKind::InvalidData, "frame is too large"));
    }

    let mut payload = vec![0_u8; length];
    input.read_exactly_async(&mut payload).await?;
    Ok(payload)
}
```

A Tokio caller wraps its reader in `TokioInput`; a `futures-io` caller wraps its reader in `FuturesInput`. Both call the same `read_frame_async` function. `TokioAsyncRead` and `FuturesAsyncRead` provide the reverse direction when a Qubit byte stream must be presented to that ecosystem.

Synchronous and asynchronous drivers remain separate functions. The benefit is that the async public API does not duplicate one implementation for Tokio and another for `futures-io`.

## 5. Compose transfer policies around the decoder

A caller can place transport policy outside the protocol:

```text
transport adapter -> limit -> buffer -> counting -> frame decoder
```

For example, a Tokio service can construct:

```rust,ignore
use qubit_io::{
    AsyncBufferedInput, AsyncCountingInput, AsyncLimitInput, TokioInput,
};

let mut input = AsyncCountingInput::new(AsyncBufferedInput::with_capacity(
    AsyncLimitInput::new(TokioInput::new(stream), 1_048_576),
    8 * 1024,
));
let frame = read_frame_async(&mut input).await?;
let consumed = input.bytes_read();
```

The 1 MiB limit bounds a connection. The 64 KiB check in the decoder bounds one frame. Neither policy has to know the other's implementation.

Wrapper order changes meaning. Compare these two synchronous flows:

```text
decoder -> CountingInput -> BufferedInput -> LimitInput -> source
decoder -> BufferedInput -> CountingInput -> LimitInput -> source
```

In the first flow, counting is outside the buffer and reports items delivered to the decoder. In the second, counting is inside the buffer and reports items pulled from the source, including unread prefetched items. Apply the same rule when deciding whether a checksum describes bytes consumed by a codec or bytes fetched from a transport.

`LimitInput` and `LimitOutput` expose at most a remaining item count. `CountingInput` and `CountingOutput` saturating-count successful items. `ChecksumInput` and `ChecksumOutput` hash successful `u8` prefixes only. `TeeInput` and `TeeOutput` mirror a source or primary path, but they are ordered and non-transactional: a branch failure never rolls back work already performed by the source or primary path.

`inner_mut()` and `branch_mut()` intentionally bypass wrapper bookkeeping. Reads and writes through them are not limited, counted, hashed, or mirrored.

## 6. Recover ownership after partial progress

`read` and `write` perform one operation and may make partial progress. `read_fully` stops at EOF and returns the transferred count; `read_exactly` returns `UnexpectedEof` unless it fills the destination. `write_fully` retries interrupted writes and returns `WriteZero` if an output stops making progress.

Buffers make ownership explicit:

- `BufferedInput::into_parts` returns the inner input and unread buffer without discarding prefetched items. Consume the unread window before reading the inner input again.
- `BufferedOutput::into_parts` performs no I/O and returns the inner output plus pending items. Flush first for normal completion; after a flush failure, retain the wrapper to retry or inspect it.
- Dropping a synchronous `BufferedOutput` makes only a best-effort flush. Asynchronous destructors cannot perform I/O, so call `flush_async` explicitly.
- `AsyncClose::close_async` represents real transport shutdown; flushing is not closing.

Named async futures retain their multi-poll state. Before dropping a pending `ReadFullyFuture`, `ReadExactFuture`, or `WriteFullyFuture`, inspect `items_read()` or `items_written()` if recovery needs the exact progress count. A single underlying poll that returns `Pending` or an error reports no new successful items, but an aggregate future can already contain progress from earlier polls. Implementations must not expose `WouldBlock` or `Interrupted` through the async trait boundary.

## 7. Case study: Map/Reduce records without byte plumbing

An item stream can carry typed business records. The mapper below does not know whether records originated from an in-memory partition, a file-backed engine, or a network deserializer. The complete checked program, including its in-memory adapters and assertions, is [examples/typed_records.rs](../examples/typed_records.rs).

```rust
use std::io;
use qubit_io::{Input, Output};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Sale {
    store_id: u32,
    category_id: u16,
    amount_cents: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct CategoryRevenue {
    category_id: u16,
    amount_cents: u64,
}

fn map_partition<I, O>(input: &mut I, output: &mut O) -> io::Result<()>
where
    I: Input<Item = Sale>,
    O: Output<Item = CategoryRevenue>,
{
    let mut sales = [Sale::default(); 2];
    loop {
        let count = input.read(&mut sales)?;
        if count == 0 {
            return Ok(());
        }

        let mut revenues = [CategoryRevenue::default(); 2];
        for (sale, revenue) in sales[..count].iter().zip(&mut revenues) {
            *revenue = CategoryRevenue {
                category_id: sale.category_id,
                amount_cents: sale.amount_cents,
            };
        }
        output.write_fully(&revenues[..count])?;
    }
}
```

The execution engine implements the I/O boundary once. These small in-memory adapters make the example runnable; application code normally receives its adapters from the engine.

```rust
use std::io;
use qubit_io::{Input, Output};

struct SliceRecordInput<'a, T> {
    items: &'a [T],
    position: usize,
}

impl<T: Copy> Input for SliceRecordInput<'_, T> {
    type Item = T;

    unsafe fn read_unchecked(
        &mut self,
        output: &mut [T],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        let read = count.min(self.items.len() - self.position);
        output[index..index + read]
            .copy_from_slice(&self.items[self.position..self.position + read]);
        self.position += read;
        Ok(read)
    }
}

#[derive(Default)]
struct VecRecordOutput<T> {
    items: Vec<T>,
}

impl<T: Copy> Output for VecRecordOutput<T> {
    type Item = T;

    unsafe fn write_unchecked(
        &mut self,
        input: &[T],
        index: usize,
        count: usize,
    ) -> io::Result<usize> {
        self.items.extend_from_slice(&input[index..index + count]);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
```

The `unsafe` methods are confined to the adapter implementation. Their callers have already proved the indexed ranges are valid; the implementation copies exactly `count` items and advances the source by the same amount. The mapper itself uses only safe operations.

Now compose a record pipeline:

```rust
use std::io;
use qubit_io::{
    BufferedInput, CountingInput, CountingOutput, LimitInput, TeeOutput,
};

fn run_mapper() -> io::Result<()> {
    let source = SliceRecordInput {
        items: &[
            Sale { store_id: 1, category_id: 7, amount_cents: 300 },
            Sale { store_id: 2, category_id: 7, amount_cents: 500 },
            Sale { store_id: 3, category_id: 9, amount_cents: 900 },
        ],
        position: 0,
    };
    let limited = LimitInput::new(source, 2);
    let buffered = BufferedInput::with_capacity(limited, 2);
    let mut input = CountingInput::new(buffered);

    let output = TeeOutput::new(
        VecRecordOutput::default(),
        VecRecordOutput::default(),
    );
    let mut output = CountingOutput::new(output);
    map_partition(&mut input, &mut output)?;

    assert_eq!(2, input.items_read());
    assert_eq!(2, output.items_written());
    let (shuffle, audit) = output.into_inner().into_parts();
    assert_eq!(shuffle.items, audit.items);
    assert_eq!(2, shuffle.items.len());
    Ok(())
}
```

Here the limit, buffer, and counters operate in records, not bytes. `TeeOutput` sends the typed mapping result to both a shuffle sink and an audit sink without changing the mapper.

The boundary is intentional:

- `Input`/`Output`, limit, counting, and tee can move non-`u8` items.
- Checksum wrappers are byte-only because `std::hash::Hasher` consumes bytes.
- Qubit `Buffer<T>` and buffered wrappers require `Copy + Default`. The records above meet that condition.
- Records containing `String` or another non-`Copy` field can still use core streams and the wrappers that do not require copying, but not the current generic buffer.
- Network and disk boundaries still need encoding. The gain is avoiding repeated encode/decode work in every business operator.

## 8. Standard I/O, seek, and advanced tools

### Standard I/O extensions

Standard-library integrations live under `qubit_io::std_io` and extension traits under `qubit_io::std_io::ext`. They add bounded reads, bounded strings and delimiter reads, copy helpers, discard helpers, and position-preserving operations for byte streams. For data controlled by another party, prefer a `*_limited` method over unbounded `read_to_end`, `read_to_string`, or delimiter reads; the limit is resource policy.

### Seek and transfer utilities

`Seekable` measures positions in the wrapped stream's item unit. `SeekableInput` and `SeekableOutput` express useful combinations without adding behavior. `PositionGuard` restores a recorded position on drop unless dismissed; call `restore` when the restoration error must be observed.

`Streams` is a non-constructible namespace. Its `copy_input_to_output*` methods work with generic Qubit items, while its `copy*` and comparison methods work with `std::io` byte streams.

### Buffers and pinned asynchronous values

`Buffer<T>` owns initialized `Copy + Default` storage and exposes a readable window plus spare slots. Its low-level state-changing methods are `unsafe` because callers must prove the requested range fits. Use `BufferedInput` or `BufferedOutput` for ordinary buffering; use `Buffer<T>` directly only when implementing a specialized driver or encoder.

`BufferedInput::ensure` and `BufferedOutput::ensure` avoid another Qubit buffer only when `is_buffered()` says the value is buffered. They cannot detect `std::io::BufReader` or `BufWriter` that entered through blanket `Read` and `Write` implementations.

Pinned `!Unpin` async values and trait objects use `PinnedAsyncInputExt` and `PinnedAsyncOutputExt` instead of the `Unpin` convenience methods.

## 9. Choose the narrowest boundary

1. Use `Input`/`Output` for blocking transfer. Use async traits only when the caller already has an async execution path.
2. Put a limit closest to untrusted input or an output quota.
3. Add a buffer at the layer that benefits from batching. Do not use `ensure` to wrap a standard `BufReader` or `BufWriter` again.
4. Place counters and checksums according to the bytes or items that must be observed.
5. Flush or close explicitly. Retain wrappers after failure when recovery needs their unread or pending data.
6. Use native I/O directly when no runtime-neutral boundary, item-generic transfer, or composable policy is needed.

[The API documentation](https://docs.rs/qubit-io) documents each API's exact error, panic, ownership, and pinning constraints. This guide explains how those APIs fit into a library design.
