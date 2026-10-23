# Qubit IO User Guide

[简体中文](user_guide.zh_CN.md) | [Project overview](../README.md) | [API documentation](https://docs.rs/qubit-io/0.19.0/qubit_io/)

This guide is for Rust developers processing streams of bytes, characters, or business objects, and library authors implementing new streams. It describes the current `qubit-io 0.19.0` codebase and requires Rust 1.94 or later, using the Rust 2024 edition.

## Contents

1. [Understand generic input and output streams](#1-understand-generic-input-and-output-streams)
2. [Build an object-processing pipeline](#2-build-an-object-processing-pipeline)
3. [Handle reads, writes, and end of input](#3-handle-reads-writes-and-end-of-input)
4. [Connect standard-library byte streams](#4-connect-standard-library-byte-streams)
5. [Add buffering](#5-add-buffering)
6. [Limit and count transferred items](#6-limit-and-count-transferred-items)
7. [Compute checksums and content digests](#7-compute-checksums-and-content-digests)
8. [Mirror input and output](#8-mirror-input-and-output)
9. [Seek, copy, and compare](#9-seek-copy-and-compare)
10. [Use asynchronous generic streams](#10-use-asynchronous-generic-streams)
11. [Implement your own streams](#11-implement-your-own-streams)
12. [Troubleshooting and capability reference](#12-troubleshooting-and-capability-reference)

For a first integration, read chapters 1–3 and then select the wrappers you need. Start with chapter 4 if you already have standard byte streams. Async callers should continue with chapter 10; implementors should read chapter 11. API documentation provides exact signatures; this guide explains composition, observable results, and failure handling.

## 1. Understand generic input and output streams

A batch job reads sales records, produces category-revenue records, and passes them to another processing stage. If every stage only accepts bytes, business processing repeatedly becomes involved in serialization. Qubit IO gives these stages a common generic stream interface:

```text
Input<Item = Sale> → business processing → Output<Item = CategoryRevenue>
```

`Input` reads items into a caller-provided slice; `Output` accepts items from a slice. The associated type `Item` selects the element type: `u8`, `char`, `String`, and business structs can all be stream items. Counts returned by reads and writes measure items, so reading ten records is different from reading ten bytes.

The core traits impose no universal `Copy`, `Clone`, or `Default` bound. Concrete implementations and wrappers can add requirements: current buffering requires `Clone + Default`, while checksums and digests are byte-only. Generic object streams do not encode objects for disk or networks. An encoding layer must still convert between objects and bytes at those boundaries.

Buffering, limits, counting, and mirroring compose around these interfaces. A processing function depends on its input and output contracts; its caller selects the source, destination, and wrapper order. Standard `Read` and `Write` implementations participate as byte streams automatically. Async traits retain the item model and connect to different ecosystems through adapters.

This library transfers items. Paths, file identity, publication, commit, abort, and persistence belong at a higher layer; see [qubit-fs](https://docs.rs/qubit-fs). For typed values encoded in bytes, see [qubit-io-binary](https://docs.rs/qubit-io-binary); for text encoding, see [qubit-io-text](https://docs.rs/qubit-io-text). Applications needing only native byte I/O can use standard-library or runtime interfaces directly.

## 2. Build an object-processing pipeline

### Prepare a project

Create a Rust 2024 binary project and add this dependency to `Cargo.toml`:

```toml
[dependencies]
qubit-io = "0.19.0"
```

The default feature set is empty; no async runtime is needed. Save the complete program below as `src/main.rs` and run `cargo run`. Its in-memory source and sink are an introductory exercise. Passing assertions and exit code 0 indicate success; the program prints nothing.

### Read sales and produce category-revenue records

This job exposes only two of the three source records. Records contain `String` fields and do not need to be encoded into bytes first. `map_sales` maps each sale to one revenue record; it does not sum categories. Aggregation belongs to the receiver or a downstream stage.

`Records` and `Collected` are application-owned in-memory adapters, not library types. You can read `map_sales` and `main` first; chapter 11 explains the implementation contracts.

<!-- example: objects profile=core mode=run -->
```rust
use std::io;
use qubit_io::{BufferedInput, BufferedOutput, CountingInput, CountingOutput,
               Input, LimitInput, Output};

#[derive(Clone, Debug, Default, PartialEq)]
struct Sale {
    category: String,
    amount_cents: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct CategoryRevenue {
    category: String,
    amount_cents: u64,
}

// Application-owned adapters; not types provided by qubit-io.
struct Records<T> {
    items: Vec<T>,
    position: usize,
}

impl<T: Clone> Input for Records<T> {
    type Item = T;

    unsafe fn read_unchecked(
        &mut self, output: &mut [T], index: usize, count: usize,
    ) -> io::Result<usize> {
        let n = count.min(self.items.len() - self.position);
        output[index..index + n]
            .clone_from_slice(&self.items[self.position..self.position + n]);
        self.position += n;
        Ok(n)
    }
}

struct Collected<T>(Vec<T>);

impl<T: Clone> Output for Collected<T> {
    type Item = T;

    unsafe fn write_unchecked(
        &mut self, input: &[T], index: usize, count: usize,
    ) -> io::Result<usize> {
        self.0.extend_from_slice(&input[index..index + count]);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

fn map_sales<I, O>(input: &mut I, output: &mut O) -> io::Result<()>
where
    I: Input<Item = Sale> + ?Sized,
    O: Output<Item = CategoryRevenue> + ?Sized,
{
    let mut batch = std::array::from_fn::<_, 2, _>(|_| Sale::default());
    loop {
        let n = input.read(&mut batch)?;
        if n == 0 { return Ok(()); }
        for sale in &batch[..n] {
            let revenue = CategoryRevenue {
                category: sale.category.clone(),
                amount_cents: sale.amount_cents,
            };
            output.write_fully(&[revenue])?;
        }
    }
}

fn main() -> io::Result<()> {
    let source = Records {
        items: vec![
            Sale { category: "books".into(), amount_cents: 300 },
            Sale { category: "books".into(), amount_cents: 500 },
            Sale { category: "games".into(), amount_cents: 900 },
        ],
        position: 0,
    };
    let mut input = CountingInput::new(BufferedInput::with_capacity(
        LimitInput::new(source, 2), 4,
    ));
    let mut output = CountingOutput::new(BufferedOutput::with_capacity(
        Collected(Vec::new()), 4,
    ));
    map_sales(&mut input, &mut output)?;
    output.flush()?;
    assert_eq!(input.items_read(), 2);
    assert_eq!(output.items_written(), 2);
    let (sink, recovery) = output.into_inner().into_parts();
    assert!(matches!(recovery, qubit_io::BufferedOutputRecovery::Pending(buffer) if buffer.is_empty()));
    assert_eq!(sink.0, vec![
        CategoryRevenue { category: "books".into(), amount_cents: 300 },
        CategoryRevenue { category: "books".into(), amount_cents: 500 },
    ]);
    Ok(())
}
```

Each layer has a specific responsibility:

| Layer | Responsibility in this example |
| --- | --- |
| `Records<Sale>` | Supplies sales records already constructed by the application |
| `LimitInput` | Exposes at most two records without consuming the third |
| `BufferedInput` | Buffers records with a capacity of four items |
| `CountingInput` | Counts records actually read by the processing function |
| `map_sales` | Maps sales to revenue records |
| `CountingOutput<BufferedOutput<_>>` | Counts accepted revenue records; explicitly flush before checking the sink |
| `Collected<CategoryRevenue>` | Retains the final results for this exercise |

You have now checked object input, a different output item type, wrapper composition, flushing, and result inspection. If `output.flush()?` fails, the success assertions do not run. In an application, retain the output and inspect pending data as described in chapter 5.

In a real project, the application prepares sources and sinks, constructs the pipeline, calls the processor, and decides on retry or compensation. `map_sales` can live in a business module; the two adapters can be replaced by object streams supplied by an execution engine. These memory adapters provide no persistent storage, transactions, or audit-delivery guarantees.

The repository also provides a shorter [typed-record example](../examples/typed_records.rs) using `Copy` records. Run `cargo run --example typed_records` from the repository root. It checks mapping, limits, buffering, counting, and two in-memory outputs. It does not exercise this chapter's `String` records or real storage integration.

## 3. Handle reads, writes, and end of input

`Input` and `Output` operate on slices, but a single operation need not transfer the entire batch.

| Operation | Successful result and stopping condition |
| --- | --- |
| `read` | One read; returns the actual count. Zero for a nonempty request denotes end of input |
| `read_fully` | Repeats until the slice is full or EOF; returns the count and retries `Interrupted` |
| `read_exactly` | Must fill the slice; early EOF returns `UnexpectedEof` |
| `write` | One write; returns the accepted count and may be short |
| `write_fully` | Repeats until all items are accepted; retries `Interrupted`, returns `WriteZero` on no progress |
| `flush` | Requests the output implementation's flushing behavior; check its result |

Zero progress for an empty slice does not establish EOF. Process only `buffer[..n]` after reading; the remaining slots are not valid input. Aggregate operations may already have read or written a prefix before returning an error. Errors do not restore positions or roll back transfers.

This independent `src/main.rs` accepts a short input, then checks the position after an exact read fails:

<!-- example: basic profile=core mode=run -->
```rust
use std::io::{self, Cursor, ErrorKind};
use qubit_io::{Input, Output};

fn main() -> io::Result<()> {
    let mut source = Cursor::new(b"abc");
    let mut batch = [0; 5];
    let n = source.read_fully(&mut batch)?;
    assert_eq!(n, 3);
    let mut destination = Vec::new();
    destination.write_fully(&batch[..n])?;
    assert_eq!(destination, b"abc");

    let mut truncated = Cursor::new(b"abc");
    assert_eq!(truncated.read_exactly(&mut [0; 5]).unwrap_err().kind(),
               ErrorKind::UnexpectedEof);
    assert_eq!(truncated.position(), 3);
    Ok(())
}
```

In the object pipeline, these counts measure records. Use `read_exactly` when the business operation requires a complete batch; use `read` or `read_fully` and inspect the count when a short final batch is valid.

The [bounded-frame example](../examples/bounded_frame.rs) demonstrates byte-protocol handling. Run `cargo run --example bounded_frame` to check normal, empty, oversized, and truncated messages. Its four-byte big-endian length prefix is checked before allocating a payload larger than 64 KiB. An input quota does not replace protocol-length validation.

## 4. Connect standard-library byte streams

Every `std::io::Read` implementation automatically implements `Input<Item = u8>`, and every `Write` implementation automatically implements `Output<Item = u8>`. Pass a `File`, `Cursor`, or another standard stream directly to a byte-oriented Qubit algorithm.

### Simplify byte-stream API bounds

For APIs that only transfer bytes, use `I: ByteInput` in place of
`I: Input<Item = u8>`, and `O: ByteOutput` in place of
`O: Output<Item = u8>`. These are method-free subtraits with blanket
implementations for every matching stream, including `?Sized` streams.
Standard readers and writers therefore satisfy them automatically, as do
byte-oriented Qubit wrappers. Custom stream types still implement `Input` or
`Output`, with `Item = u8`.

This independent program uses the short bounds in parameters and opaque
return types. The output borrows the destination vector. Its trait objects
pass through the same generic function, then an `OutputRef` exposes the
output to a standard-library writer:

<!-- example: byte-traits profile=core mode=run -->
```rust
use std::io::{self, Cursor, Write};
use qubit_io::{ByteInput, ByteOutput, CountingOutput, Output, OutputRef};

fn input() -> impl ByteInput {
    Cursor::new(b"QBIT")
}

fn output(bytes: &mut Vec<u8>) -> impl ByteOutput + '_ {
    CountingOutput::new(bytes)
}

fn copy_magic<I: ByteInput + ?Sized, O: ByteOutput + ?Sized>(
    input: &mut I, output: &mut O,
) -> io::Result<()> {
    let mut magic = [0; 4];
    input.read_exactly(&mut magic)?;
    output.write_fully(&magic)
}

fn main() -> io::Result<()> {
    let mut source = input();
    let mut bytes = Vec::new();
    {
        let mut sink = output(&mut bytes);
        let input: &mut dyn ByteInput = &mut source;
        let output: &mut dyn ByteOutput = &mut sink;
        copy_magic(input, output)?;

        let mut writer = OutputRef::new(output).into_std_write();
        Write::write_all(&mut writer, b"!")?;
        Write::flush(&mut writer)?;
    }
    assert_eq!(bytes, b"QBIT!");
    Ok(())
}
```

The final assertion verifies both the Qubit transfer and the standard write.
`read_exactly` still returns `UnexpectedEof` for a truncated header;
`write_fully` still returns `WriteZero` if the sink stops making progress.
Changing the bound adds no buffering, allocation, or error handling.

`ByteInput` and `ByteOutput` are traits, not concrete type aliases. A return
type must use `impl ByteInput` / `impl ByteOutput`, a concrete stream type,
or a pointer such as `Box<dyn ByteInput>`. Replacing a concrete return type
with `impl ByteInput` hides its type-specific methods. `?Sized` on the
parameters permits borrowed trait objects. For owned trait objects, use
`BoxInput` / `BoxOutput` when an API requires an owned `Input` / `Output`
implementor. For asynchronous bytes, use `AsyncByteInput` / `AsyncByteOutput`.

### Name character and asynchronous stream bounds

All four base interfaces have item-specific subtraits:

| Base interface | `Item = u8` | `Item = char` |
| --- | --- | --- |
| `Input` | `ByteInput` | `CharInput` |
| `Output` | `ByteOutput` | `CharOutput` |
| `AsyncInput` | `AsyncByteInput` | `AsyncCharInput` |
| `AsyncOutput` | `AsyncByteOutput` | `AsyncCharOutput` |

They inherit the base operations and automatically apply to matching streams,
including unsized trait objects. Implement the base trait rather than adding
an explicit implementation of a specialized trait. Generic parameters and
opaque return types follow the byte example above, such as `I: CharInput` or
`-> impl AsyncCharOutput`. These names do not add `Send`, `Sync`, or `Unpin`
requirements.

The following API-definition fragments illustrate synchronous character
operations and asynchronous character/byte polling. The caller supplies the
concrete stream; asynchronous callers also supply its pinned borrow and task
context:

<!-- example: character-poll profile=core mode=skip reason="API contract fragment requires caller supplied stream state" -->
```rust
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use qubit_io::{AsyncByteOutput, AsyncCharInput, CharInput, CharOutput};

fn read_char<I: CharInput + ?Sized>(input: &mut I) -> io::Result<char> {
    let mut chars = ['\0'];
    input.read_exactly(&mut chars)?;
    Ok(chars[0])
}

fn write_chars<O: CharOutput + ?Sized>(output: &mut O, chars: &[char]) -> io::Result<()> {
    output.write_fully(chars)
}

fn poll_chars<I: AsyncCharInput + ?Sized>(
    input: Pin<&mut I>, cx: &mut Context<'_>, chars: &mut [char],
) -> Poll<io::Result<usize>> {
    input.poll_read(cx, chars)
}

fn poll_bytes<O: AsyncByteOutput + ?Sized>(
    output: Pin<&mut O>, cx: &mut Context<'_>, bytes: &[u8],
) -> Poll<io::Result<usize>> {
    output.poll_write(cx, bytes)
}
```

For a stream containing `['你', '🦀']`, a successful full read returns two
items, even though its UTF-8 encoding occupies seven bytes. A `char` is one
Unicode scalar value, not a grapheme cluster; combining sequences can contain
multiple items. These traits perform no text encoding, decoding, normalization,
or segmentation, and character streams cannot directly use `into_std_read()`
or `into_std_write()`. Use an encoding layer when crossing a byte boundary.

Async bounds preserve the base readiness rules: `Pending` transfers no items
and the implementation arranges a wakeup. Pin a `!Unpin` stream before polling;
for future-returning helpers on pinned streams, use `PinnedAsyncInputExt` /
`PinnedAsyncOutputExt`. Tokio and `futures-io` values still require their
existing adapters; implementing a runtime's async interface alone does not
implement `AsyncByteInput` or `AsyncByteOutput`.

### Convert to standard-library interfaces

In the other direction, `input.into_std_read()` and `output.into_std_write()` produce `StdRead` and `StdWrite`. They consume the original value. Wrap a borrow in `InputRef` or `OutputRef` first to retain ownership. This independent program demonstrates both directions, using qualified calls where method names overlap:

<!-- example: std profile=core mode=run -->
```rust
use std::io::{self, Cursor, Read, Write};
use qubit_io::{Input, InputRef, Output};

fn main() -> io::Result<()> {
    let mut source = Cursor::new(b"QBIT");
    let mut magic = [0; 4];
    Input::read_exactly(&mut source, &mut magic)?;
    assert_eq!(&magic, b"QBIT");

    let mut reader = InputRef::new(&mut source).into_std_read();
    let mut rest = Vec::new();
    Read::read_to_end(&mut reader, &mut rest)?;
    assert!(rest.is_empty());
    let _borrow = reader.into_inner();

    let mut writer = Vec::<u8>::new().into_std_write();
    Write::write_all(&mut writer, b"QBIT")?;
    Write::flush(&mut writer)?;
    assert_eq!(writer.into_inner(), b"QBIT");
    Ok(())
}
```

`StdRead` and `StdWrite` provide `get_ref`, `get_mut`, and `into_inner`, and forward standard `Seek` when the inner stream implements byte-oriented `Seekable`. Conversion constructs a wrapper without allocating, transferring, flushing, or closing. Standard vectored operations use the default fallback; they do not promise to retain the inner stream's scatter/gather optimization.

For dynamic dispatch, use `InputRef` around a borrowed `dyn Input<Item = u8>` and `BoxInput` around an owned boxed input trait object. The output equivalents are `OutputRef` and `BoxOutput`. These adapters also support object streams, but conversion to standard `Read` or `Write` remains restricted to `u8`.

A concrete type implementing standard `Read` cannot also directly implement `Input<Item = Sale>` because its blanket implementation already exists. Introduce a newtype for the object semantics. Corresponding coherence boundaries apply to `Write`/`Output` and `Seek`/`Seekable`.

## 5. Add buffering

### Configure capacity in items

`BufferedInput` and `BufferedOutput` require `Item: Clone + Default`. This includes `String` and business records satisfying those bounds. `new` uses the library's default capacity; `with_capacity(inner, n)` selects an item-slot count. Zero capacity is normalized to one slot. Use `try_with_capacity` and `try_reserve_capacity` when allocation failure must be handled.

Object buffering is not a zero-cost move: initialization and transfer may call `Default` and `Clone`. Four `String` slots do not imply a four-byte memory bound. Include owned allocations and deep-clone costs when budgeting memory; limiting object count does not limit individual object sizes.

`BufferedInput::ensure` and `BufferedOutput::ensure` consult `is_buffered()` before adding buffering and return `EnsuredBufferedInput` or `EnsuredBufferedOutput`. `ensure_boxed` serves boxed boundaries. Standard `BufReader` and `BufWriter` entering through blanket implementations cannot be detected this way. Do not automatically wrap existing standard buffers again on the strength of that check.

### Recover prefetched and pending data

Inputs can prefetch. Inspect `unread()` and `unread_len()` for items not yet delivered. `fill_more` and `fill_until` replenish the window; `ensure_available` reports an error if the requested count cannot be obtained. `into_parts()` performs no I/O and returns the inner input plus its `Buffer`. Process that buffer's `readable()` window before continuing with the inner source.

`fill_until(n)` returns `false` at EOF when fewer than `n` items are available and keeps the unread tail. `ensure_available(n)` instead returns `UnexpectedEof` and consumes that insufficient window. The synchronous and asynchronous input buffers follow the same distinction. Use `fill_until` when truncated data must be retained; after `ensure_available` fails, do not expect the window to remain available.

<!-- example: buffer-eof-recovery profile=core mode=run -->
```rust
use std::io;
use std::io::Cursor;
use std::io::ErrorKind;

use qubit_io::BufferedInput;

fn main() -> io::Result<()> {
    let mut retained = BufferedInput::with_capacity(Cursor::new(b"ab"), 3);
    assert!(!retained.fill_until(3)?);
    assert_eq!(b"ab", retained.unread());

    let mut consumed = BufferedInput::with_capacity(Cursor::new(b"ab"), 3);
    let error = consumed.ensure_available(3).expect_err("input is truncated");
    assert_eq!(ErrorKind::UnexpectedEof, error.kind());
    assert!(consumed.unread().is_empty());
    Ok(())
}
```


A successful output `write_fully` can mean acceptance into a buffer. On normal completion, explicitly `flush` and check the result. `into_parts()` does not flush: it returns the inner output and a `BufferedOutputRecovery` state. `Pending` contains items not reported as written by a normally returning inner call; `Indeterminate` means an inner I/O call panicked and some buffered items may already have taken effect, so blind replay is unsafe. This program splits an unflushed output to show how the caller handles the normal `Pending` case:

<!-- example: buffer profile=core mode=run -->
```rust
use std::io::{self, Cursor};
use qubit_io::{BufferedInput, BufferedOutput, Input, Output};

fn main() -> io::Result<()> {
    let mut input = BufferedInput::with_capacity(Cursor::new(b"abcdef"), 4);
    assert!(input.fill_until(4)?);
    let mut first = [0; 1];
    input.read_exactly(&mut first)?;
    assert_eq!(&first, b"a");
    let (mut source, unread) = input.into_parts();
    let mut rest = unread.readable().to_vec();
    let mut tail = [0; 8];
    let n = source.read_fully(&mut tail)?;
    rest.extend_from_slice(&tail[..n]);
    assert_eq!(rest, b"bcdef");

    let mut output = BufferedOutput::with_capacity(Vec::<u8>::new(), 8);
    output.write_fully(b"abc")?;
    let (mut sink, recovery) = output.into_parts();
    assert!(sink.is_empty());
    let pending = match recovery {
        qubit_io::BufferedOutputRecovery::Pending(buffer) => buffer,
        qubit_io::BufferedOutputRecovery::Indeterminate(_) => unreachable!("no inner I/O panic occurred"),
    };
    sink.write_fully(pending.readable())?;
    Output::flush(&mut sink)?;
    assert_eq!(sink, b"abc");
    Ok(())
}
```

Retain the wrapper after flush failure. If draining fails normally, successfully written prefixes are removed from the pending window; do not resend the entire batch. If an inner I/O call panics, the wrapper becomes poisoned and future I/O through it is rejected. `into_parts()` then returns `Indeterminate`, including when the buffer is empty, because completed side effects cannot be inferred. Do not replay that payload without application-level reconciliation. Without a panic, `into_parts()` returns `Pending`; its window may be empty while the inner `flush` failed. Retry flushing only when the underlying failure and its side effects permit retry.

Synchronous `BufferedOutput` drop only attempts to drain its own buffer and ignores errors; it does not replace explicit `flush`. Async buffering performs no I/O on drop. Direct reads through `inner_mut()` may skip unread prefetched items; direct writes may overtake pending output. Transfer normally through the wrapper.

`Buffer<T>` is the backing storage used by these wrappers, exposing consumed, readable, and spare windows. Prefer buffered streams for ordinary use. Specialized drivers and encoders can manage the windows directly, but must satisfy the range contracts of its `unsafe` state-changing methods.

`AsyncBufferedOutput::spare_raw_parts_mut()` returns `(storage, index, count)`, where the writable range is `storage[index..index + count]`. `count` is the number of spare items, not the end index. Initialize exactly that range before calling `advance(count)`.

## 6. Limit and count transferred items

`LimitInput` and `LimitOutput` accept `u64` item quotas. Inspect `remaining()`; successful transfers reduce it. An exhausted input returns zero even if its source contains more items. An exhausted output also returns zero, so an oversized `write_fully` eventually returns `WriteZero` with its accepted prefix retained.

`CountingInput` and `CountingOutput` saturating-count successfully reported transfers without wrapping on `u64` overflow. Use `items_read()` and `items_written()` for objects; byte streams also provide `bytes_read()` and `bytes_written()`. Counters cannot recover inner side effects that occurred before an operation returned an error without reporting successful progress.

This independent program compares observation points and checks exhausted quotas:

<!-- example: order profile=core mode=run -->
```rust
use std::io::{self, Cursor, ErrorKind};
use qubit_io::{BufferedInput, CountingInput, Input, LimitInput,
               LimitOutput, Output};

fn main() -> io::Result<()> {
    let mut consumed = CountingInput::new(
        BufferedInput::with_capacity(Cursor::new(b"abcdef"), 4));
    consumed.read_exactly(&mut [0; 1])?;
    assert_eq!(consumed.items_read(), 1);

    let mut fetched = BufferedInput::with_capacity(
        CountingInput::new(Cursor::new(b"abcdef")), 4);
    fetched.read_exactly(&mut [0; 1])?;
    assert_eq!(fetched.inner().items_read(), 4);

    let mut limited = LimitInput::new(Cursor::new(b"abc"), 2);
    assert_eq!(limited.read_fully(&mut [0; 4])?, 2);
    assert_eq!(limited.remaining(), 0);
    assert_eq!(limited.read(&mut [0; 1])?, 0);
    assert_eq!(limited.into_inner().position(), 2);

    let mut output = LimitOutput::new(Vec::<u8>::new(), 2);
    assert_eq!(output.write_fully(b"abc").unwrap_err().kind(),
               ErrorKind::WriteZero);
    assert_eq!(output.into_inner(), b"ab");
    Ok(())
}
```

```text
processor → CountingInput → BufferedInput → source
            counts consumption
processor → BufferedInput → CountingInput → source
                            counts fetching, including prefetch
```

Chapter 2 puts the input limit inside the buffer, preventing prefetch beyond two records. A limit outside the buffer still restricts delivery to the processor, but the buffer can fetch more records. For output, an outer limit restricts acceptance; an inner limit may only expose quota exhaustion when buffered items are drained.

`remaining() == 0` does not prove that the underlying input ended exactly at the quota. To validate the size of the entire remaining input, use the end-limited methods in chapter 9. Transfers through `inner_mut()` bypass limits and counters.

## 7. Compute checksums and content digests

### Hasher-based checksums

`ChecksumInput` and `ChecksumOutput` accept a caller-supplied `std::hash::Hasher` and operate only on `u8`. They feed successfully transferred prefixes to the hasher; `checksum()` returns the current `u64`. Each successful non-empty transfer prefix is passed to `Hasher::write` in that operation; the resulting checksum can therefore depend on transfer chunking and the hasher implementation. Empty operations and zero progress do not update the hasher. Use the `Digest` wrappers when the requirement is a content digest whose result is independent of transfer chunk boundaries. This example uses `DefaultHasher` to demonstrate the interface, not as a stable cross-version file format or cryptographic digest:

<!-- example: checksum profile=core mode=run -->
```rust
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;
use std::io::{self, Cursor};
use qubit_io::{ChecksumInput, Input};

fn main() -> io::Result<()> {
    let mut input = ChecksumInput::new(Cursor::new(b"abc"), DefaultHasher::new());
    let mut bytes = [0; 3];
    input.read_exactly(&mut bytes)?;
    let mut expected = DefaultHasher::new();
    expected.write(&bytes);
    assert_eq!(input.checksum(), expected.finish());
    Ok(())
}
```

### Content digests

For fixed-output digests such as SHA-256, enable `digest` and add a compatible algorithm:

```toml
[dependencies]
qubit-io = { version = "0.19.0", features = ["digest"] }
sha2 = "0.11"
```

`DigestInput` and `DigestOutput` accept a `digest::Digest` implementation. The current interface uses `digest 0.11`; algorithms built against 0.10 cannot be mixed in directly. This independent program fingerprints the entire remaining input and checks the output digest:

<!-- example: digest profile=digest mode=run -->
```rust
use std::io::{self, Cursor};
use qubit_io::{DigestInput, DigestOutput, Output};
use sha2::{Digest, Sha256};

fn main() -> io::Result<()> {
    let mut input = Cursor::new(b"hello world");
    let fingerprint = DigestInput::<_, Sha256>::digest_all(&mut input)?;
    assert_eq!(fingerprint, Sha256::digest(b"hello world"));

    let mut output = DigestOutput::new(Vec::<u8>::new(), Sha256::new());
    output.write_fully(b"hello ")?;
    assert_eq!(output.digest(), Sha256::digest(b"hello "));
    output.write_fully(b"world")?;
    output.flush()?;
    let (bytes, written_digest) = output.finalize();
    assert_eq!(bytes, b"hello world");
    assert_eq!(written_digest, fingerprint);
    Ok(())
}
```

Choose a method according to the data you want to cover:

| Method | Behavior |
| --- | --- |
| `DigestInput::<_, D>::digest_all(&mut input)` | Creates fresh algorithm state and reads from the current position to EOF; the caller retains input ownership |
| `new(inner, hasher)` | Preserves supplied initial state and accumulates during ordinary transfers |
| `digest()` | Clones and finalizes a snapshot, allowing transfers to continue; requires `D: Clone` |
| `finalize()` | Consumes the wrapper and returns `(inner, digest)` |
| `into_parts()` | Returns `(inner, hasher)` for continued computation |

`digest_all` uses a fixed 16 KiB read buffer rather than retaining the entire input, and does not require another wrapper based on `is_buffered()`. Start at the beginning to fingerprint the full original input. It retries `Interrupted` in place, preserving the hasher state and any successfully read prefix. Other read errors return no partial digest and do not rewind consumed input; a fresh call starts a new digest from the input's current position.

Checksum and digest wrappers track successfully reported byte prefixes. Errors, zero progress, and async `Pending` add nothing for that operation. `inner_mut()` bypasses tracking; `hasher_mut()` directly changes algorithm state.

`finalize()` and `into_parts()` do not read to EOF, flush, or close. An output digest can already cover bytes accepted into an inner buffer but not yet delivered. Check explicit flush or close results before treating delivery as successful; flush failure does not undo the digest.

Wrapper order follows chapter 6: `DigestInput<BufferedInput<_>, _>` tracks consumption, while an outer buffer can cause prefetched bytes to enter the digest early. Synchronous seek is forwarded without resetting checksums or digests. Repeated reads and overwriting writes accumulate in transfer order, so the result is not automatically a fingerprint of final file contents.

## 8. Mirror input and output

To give chapter 2 two revenue sinks, replace `Collected(Vec::new())` with `TeeOutput::new(Collected(Vec::new()), Collected(Vec::new()))`, and import `TeeOutput`. Keep buffering and counting in their existing positions. At result inspection, split the sink using `into_parts()` and check both `Collected` values. The application supplies the actual business sinks.

`TeeInput` reads the source first, then writes the successful prefix to its branch. `TeeOutput` writes its primary first, then fully mirrors the accepted prefix. Both support non-byte items, but execute in order without atomicity.

This independent byte example demonstrates success and a branch with an insufficient quota:

<!-- example: tee profile=core mode=run -->
```rust
use std::io::{self, ErrorKind};
use qubit_io::{LimitOutput, Output, TeeOutput};

fn main() -> io::Result<()> {
    let mut success = TeeOutput::new(Vec::<u8>::new(), Vec::<u8>::new());
    success.write_fully(b"abc")?;
    success.flush()?;
    let (primary, branch) = success.into_parts();
    assert_eq!(primary, branch);

    let mut failed = TeeOutput::new(Vec::<u8>::new(),
                                   LimitOutput::new(Vec::<u8>::new(), 2));
    let error = failed.write_fully(b"abc").unwrap_err();
    assert_eq!(error.kind(), ErrorKind::WriteZero);
    let (primary, branch) = failed.into_parts();
    assert_eq!(primary, b"abc");
    assert_eq!(branch.into_inner(), b"ab");
    // The primary has already accepted abc. Do not resend the entire batch.
    Ok(())
}
```

On branch failure, a `TeeInput` source and the caller's destination slice may already have advanced or changed. A `TeeOutput` primary may contain the entire batch while its branch contains only a prefix. Stop ordinary processing, inspect both paths, and apply application-specific idempotency or compensation. Do not interpret an error as no transfer and blindly resend. An outer counter may not reflect those side effects either.

`TeeOutput::flush` flushes the primary first and only then the branch. `into_parts()` does not flush. A `TeeInput` branch is an output that the caller must flush separately, for example through `branch_mut()`. Direct transfers through `inner_mut()` or `branch_mut()` can make the paths diverge.

Choose seek semantics explicitly: ordinary `TeeInput` seeks only its source. `TeeInput::with_sync_branch_seek` returns `SyncSeekTeeInput`, which seeks the source and then aligns the branch. `TeeOutput` likewise seeks its primary before aligning the branch. A second-path seek failure does not roll back the first. These types currently have no async tee equivalents.

## 9. Seek, copy, and compare

### Seek and restore positions

`Seekable` measures positions using its associated `Unit`. `SeekableInput` and `SeekableOutput` combine stream and matching-unit seek capabilities without implementing storage. Standard `Seek` integrations use `u8`; object streams may use records. Do not assume every `SeekFrom` offset measures bytes.

`PositionGuard` captures the position on creation. Explicit `restore()` lets you observe restoration errors; `dismiss()` disables automatic restoration. Drop-time failure is neither returned nor logged. Buffered input seeking accounts for unread items in its logical position. General buffered output seeks drain pending items first; a `Current(0)` query does not flush.

This independent program reads the beginning temporarily and restores the position, then checks an oversized generic copy:

<!-- example: seek profile=core mode=run -->
```rust
use std::io::{self, Cursor, ErrorKind, SeekFrom};
use qubit_io::{Input, PositionGuard, Seekable, Streams};

fn main() -> io::Result<()> {
    let mut source = Cursor::new(b"abcdef");
    source.seek_to(SeekFrom::Start(2))?;
    {
        let mut guard = PositionGuard::new(&mut source)?;
        guard.inner_mut().seek_to(SeekFrom::Start(0))?;
        let mut magic = [0; 2];
        guard.inner_mut().read_exactly(&mut magic)?;
        assert_eq!(&magic, b"ab");
        guard.restore()?;
    }
    assert_eq!(source.position(), 2);

    let mut input = Cursor::new(b"abc");
    let mut output = Vec::new();
    let error = Streams::copy_input_to_output_end_limited(
        &mut input, &mut output, 2).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidData);
    assert!(output.is_empty());
    assert_eq!(input.position(), 3);
    Ok(())
}
```

### Choose a copy policy

Call `Streams` methods as associated functions; it is a utility namespace. These methods do not automatically close or flush the output. The application still checks `flush` after successful copying.

| Task | Generic item streams | Standard byte streams |
| --- | --- | --- |
| Copy to EOF | `copy_input_to_output` | `copy` |
| Copy at most N items or bytes | `copy_input_to_output_at_most` | `copy_at_most` |
| Require the entire remaining input to fit within N | `copy_input_to_output_end_limited` | `copy_to_end_limited` |

Generic copying requires `Clone + Default`. Both end-limited variants can consume one excess item to detect oversized input, but their output behavior differs. The generic version collects first: oversized input and failures during collection leave the output untouched, at the cost of temporary memory growing to the item limit. After EOF is confirmed, output can still fail partially. The standard byte variant streams into the writer, so oversized input can leave N bytes in the output. Chapter 6's `LimitInput` only truncates the visible range; it does not validate the entire input this way.

`Streams::content_eq` and `compare_content` compare the remaining contents of standard byte streams from their current positions; the latter returns lexicographic ordering. Comparison advances positions and may read past the first difference to a chunk boundary. Use position guards and check restoration when positions must be preserved.

### Standard I/O extensions

Import the required extension traits from `qubit_io::std_io::ext` when working directly with standard byte streams:

| Trait | Common operations |
| --- | --- |
| `ReadExt` | `read_exact_array`, `read_exact_vec_limited`, `read_to_end_limited`, `read_to_string_limited`, copying and discarding |
| `BufReadExt` | `read_until_limited`, `read_line_limited`, `discard_until_limited` |
| `ReadSeekExt` | `peek_exact_or_eof`, `read_exact_or_eof_at` |
| `WriteSeekExt` | `write_all_at_preserving_position` |
| `SeekExt` | `stream_size` |
| `WriteExt` | Low-level writes over ranges whose validity has already been proved |

The bounded `BufReadExt` methods retry interrupted reads without resetting accumulated progress; other I/O errors are returned.

This independent program checks source position and output after quota failure instead of assuming rollback:

<!-- example: limited profile=core mode=run -->
```rust
use std::io::{self, Cursor, ErrorKind};
use qubit_io::std_io::ext::ReadExt;

fn main() -> io::Result<()> {
    let mut source = Cursor::new(b"abc");
    assert_eq!(source.read_to_end_limited(2).unwrap_err().kind(),
               ErrorKind::InvalidData);
    assert_eq!(source.position(), 3);

    let mut source = Cursor::new(b"abc");
    let mut destination = Vec::new();
    assert_eq!(source.copy_to_end_limited(&mut destination, 2)
                     .unwrap_err().kind(), ErrorKind::InvalidData);
    assert_eq!(destination, b"ab");
    assert_eq!(source.position(), 3);
    Ok(())
}
```

Bounded string methods require valid UTF-8. Line and delimiter limits count bytes, including a delimiter returned in the result. Consult each method's contract for collection rollback, consumed input, and position-restoration errors. For example, failure in `read_to_end_limited_into` restores the destination vector's original length but does not restore the reader's position. Decide whether truncation should succeed or oversized input should be rejected before processing untrusted data.

## 10. Use asynchronous generic streams

### The same item model with a separate async driver

`AsyncInput<Item = T>` and `AsyncOutput<Item = T>` retain the generic item model and express readiness through `Pin` and `Poll` without selecting an executor. They do not automatically move synchronous methods into a background worker: blocking and async drivers are separate implementations.

The library currently provides `AsyncBufferedInput`, `AsyncBufferedOutput`, and async limit, counting, checksum, and digest wrappers. Buffering still requires `Clone + Default`; checksums and digests remain byte-only. Do not assume synchronous seek and tee have async equivalents.

### Connect Tokio

Use these dependencies in an independent project's `Cargo.toml`. The application explicitly enables Tokio's runtime and macros; Qubit IO's `tokio` feature supplies I/O adaptation only.

```toml
[dependencies]
qubit-io = { version = "0.19.0", features = ["tokio"] }
tokio = { version = "1", features = ["rt", "macros", "io-util"] }
```

Save this complete program as `src/main.rs` and run `cargo run`. It transfers three bytes to buffered output, explicitly flushes and closes, checks the result, and then demonstrates reverse adaptation:

<!-- example: async profile=tokio mode=run -->
```rust
use std::io;
use qubit_io::{AsyncBufferedOutput, AsyncClose, AsyncCountingInput,
               AsyncInput, AsyncLimitInput, AsyncOutput,
               TokioAsyncRead, TokioInput, TokioOutput};

async fn transfer<I, O>(input: &mut I, output: &mut O) -> io::Result<()>
where
    I: AsyncInput<Item = u8> + Unpin,
    O: AsyncOutput<Item = u8> + Unpin,
{
    let mut batch = [0; 16];
    loop {
        let n = input.read_async(&mut batch).await?;
        if n == 0 { break; }
        output.write_fully_async(&batch[..n]).await?;
    }
    output.flush_async().await
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let source = TokioInput::new(&b"abc"[..]);
    let mut input = AsyncCountingInput::new(AsyncLimitInput::new(source, 3));
    let mut output = AsyncBufferedOutput::with_capacity(
        TokioOutput::new(Vec::<u8>::new()), 8);
    transfer(&mut input, &mut output).await?;
    assert_eq!(input.items_read(), 3);
    output.close_async().await?;
    let (sink, pending) = output.into_parts();
    assert!(pending.is_empty());
    assert_eq!(sink.into_inner(), b"abc");

    let mut reader = TokioAsyncRead::new(TokioInput::new(&b"xyz"[..]));
    let mut bytes = [0; 3];
    tokio::io::AsyncReadExt::read_exact(&mut reader, &mut bytes).await?;
    assert_eq!(&bytes, b"xyz");
    Ok(())
}
```

`transfer` depends only on Qubit traits and can be retained unchanged for another ecosystem. Closing a memory output here demonstrates capability invocation; real socket shutdown behavior belongs to its underlying adapter. `AsyncClose` is a separate optional capability, so `AsyncOutput` alone does not permit closing. A successful flush does not mean transport shutdown, remote business completion, or persistence.

### Connect futures-io

In another independent project, use:

```toml
[dependencies]
qubit-io = { version = "0.19.0", features = ["futures-io"] }
futures = "0.3"
```

This complete program demonstrates both directions using `futures` memory streams and its executor. Run `cargo run`; assertions pass without terminal output. Executor selection belongs to the application; the library does not require `futures::executor`.

<!-- example: futures profile=futures mode=run -->
```rust
use std::io;
use qubit_io::{AsyncInput, AsyncOutput, FuturesAsyncRead,
               FuturesInput, FuturesOutput};

fn main() -> io::Result<()> {
    futures::executor::block_on(async {
        let mut input = FuturesInput::new(futures::io::Cursor::new(b"abc"));
        let mut output = FuturesOutput::new(futures::io::Cursor::new(Vec::new()));
        let mut bytes = [0; 3];
        input.read_exactly_async(&mut bytes).await?;
        output.write_fully_async(&bytes).await?;
        output.flush_async().await?;
        assert_eq!(output.into_inner().into_inner(), b"abc");

        let mut reader = FuturesAsyncRead::new(
            FuturesInput::new(futures::io::Cursor::new(b"xyz")));
        futures::io::AsyncReadExt::read_exact(&mut reader, &mut bytes).await?;
        assert_eq!(&bytes, b"xyz");
        Ok(())
    })
}
```

Adapter directions are listed below. All ecosystem bridges are byte-only:

| Existing interface | Target interface | Tokio | futures-io |
| --- | --- | --- | --- |
| Ecosystem async reader | Qubit `AsyncInput` | `TokioInput` | `FuturesInput` |
| Ecosystem async writer | Qubit `AsyncOutput` | `TokioOutput` | `FuturesOutput` |
| Qubit `AsyncInput` | Ecosystem `AsyncRead` | `TokioAsyncRead` | `FuturesAsyncRead` |
| Qubit `AsyncOutput` | Ecosystem `AsyncWrite` | `TokioAsyncWrite` | `FuturesAsyncWrite` |

Reverse writer adapters additionally require inner `AsyncClose` to satisfy the ecosystem's shutdown/close operation. `AsyncOutput` alone is insufficient.

### Pinned values, cancellation, and progress

Convenience methods such as `read_async` and `write_fully_async` require `Unpin`. Use `PinnedAsyncInputExt` and `PinnedAsyncOutputExt` for pinned `!Unpin` streams or trait objects. This independent program uses the Tokio dependencies above and reads through a pinned trait object without requiring the trait object to implement `Unpin`:

<!-- example: pinned profile=tokio mode=run -->
```rust
use std::io;
use std::pin::Pin;
use qubit_io::{AsyncInput, PinnedAsyncInputExt, TokioInput};

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    let mut input: Pin<Box<dyn AsyncInput<Item = u8>>> =
        Box::pin(TokioInput::new(&b"abc"[..]));
    let mut bytes = [0; 3];
    input.as_mut().read_exactly_async(&mut bytes).await?;
    assert_eq!(&bytes, b"abc");
    Ok(())
}
```

`InputRef`/`OutputRef` serve synchronous borrows. Owned boxed async boundaries can use `BoxAsyncInput` and `BoxAsyncOutput`. Keep pinned values pinned; do not move an inner `!Unpin` value merely to recover ownership.

An underlying poll returning `Pending` or an error must not transfer new items, but aggregate futures such as `ReadFullyFuture`, `ReadExactFuture`, and `WriteFullyFuture` can already contain progress from preceding inner calls. If cancellation and recovery need exact progress, retain a named future and inspect `items_read()` or `items_written()` before dropping it. A count is not protocol state: the application must retain its own message or batch state and cannot assume restarting has no side effects.

Async digests also provide `AsyncDigestInput::<_, D>::digest_all(input.as_mut())`, taking `Pin<&mut I>` and returning `DigestAllFuture`. It retains state across polls and yields after at most 64 successful reads per poll. Cancellation discards the partial digest while retaining the advanced input position. Polling after completion panics. Async digest wrappers support inner `!Unpin` values; snapshots can be obtained through shared references, while consuming `finalize()` requires ownership of the wrapper.

Explicitly call `flush_async` on async output, and use `close_async` according to the application lifecycle when closing is supported. `AsyncBufferedOutput` drains its buffer before closing the inner output. Retain the object and inspect remaining data after failure; drop cannot replace async I/O.

## 11. Implement your own streams

### Start with synchronous adapters

For synchronous streams, `Interrupted` is retried by the aggregate read/write helpers, so it means the current primitive call made no confirmed transfer. If a primitive has already transferred a valid prefix, return `Ok(n)` first and report a later error on a subsequent call. An aggregate operation that later returns an error does not undo successful earlier calls. A domain error with applied or uncertain side effects must not be mapped to retryable `Interrupted` solely to trigger a retry; the adapter or application must choose a policy that accounts for those effects. This synchronous rule does not apply to async traits, where `Interrupted` must be handled internally and must not cross the async boundary.

Chapter 2 contains complete `Records<T>` and `Collected<T>` implementations. Their `T: Clone` bounds reflect their copying strategy, not a universal core-trait requirement. Input implementors provide `type Item` and `read_unchecked`; output implementors provide `type Item`, `write_unchecked`, and `flush`.

The primitive methods are `unsafe` because callers must prove that `index..index + count` fits the slice and addition does not overflow. Implementors must still respect that range, return no more than `count`, and advance their own state correctly. Even an implementation using safe slice operations retains the trait's `unsafe fn` signature. Higher-level algorithms should prefer safe methods such as `read` and `write`.

Check empty requests, short final batches, EOF, short writes, zero progress, `Interrupted`, observable state after failure, and object-cloning semantics. Safe entry points reject impossible reported counts with `InvalidData`; this cannot repair an out-of-bounds memory access already performed inside an implementation.

### Integrate with business modules

Keep business functions expressed in terms of `I: Input<Item = Sale>` and `O: Output<Item = CategoryRevenue>` and assemble them in the application. Chapter 2's `?Sized` allows borrowed trait objects. Use `BoxInput` and `BoxOutput` when ownership crosses a layer boundary. Do not directly implement standard `Read` on an object stream merely to satisfy byte-oriented APIs; use an encoding layer and newtypes to express distinct element semantics.

### Establish readiness before implementing async streams

Async inputs implement `poll_read_unchecked`; outputs implement `poll_write_unchecked` and `poll_flush`. Implement real closing separately through `AsyncClose::poll_close`. In addition to range and count requirements:

- Empty read/write requests return `Ready(Ok(0))` immediately.
- Arrange notification through the current waker before returning `Pending`. Returning `Pending` without a wake-up mechanism leaves the operation stalled.
- A single poll returning `Pending` or an error transfers no items. Report partial success with `Ready(Ok(n))`.
- Convert `WouldBlock` to readiness waiting and handle `Interrupted` internally. These kinds must not cross the async boundary. Safe entry points and relevant wrappers normalize violations to `InvalidData`, retaining the original error as a cause.
- Respect pinning and never move pinned inner values. Supporting `!Unpin` requires correct field projection.

An always-ready memory stream need not manufacture `Pending`. Network streams should use their ecosystem's readiness machinery, preferably through existing adapters. Wrapping blocking I/O in an `AsyncInput` implementation does not make it nonblocking.

## 12. Troubleshooting and capability reference

| Symptom | Check and action |
| --- | --- |
| Object stream cannot use buffering | Check `Clone + Default`; `String` satisfies both. Core traits have no universal bound |
| Ambiguous `read`, `write`, or `flush` | Standard and Qubit traits are both imported; qualify calls with `Input::read`, `Output::write`, or `std::io::Write::flush` |
| Conflicting custom implementation | The type may already have a standard I/O blanket implementation; distinguish byte and object semantics with a newtype |
| Exhausted quota mistaken for original EOF | Inspect `remaining()` and distinguish synthetic EOF from underlying EOF; use an end-limited method to validate the whole input |
| `WriteZero` | Check output quotas and zero progress; accepted prefixes are not rolled back, so do not resend blindly |
| `UnexpectedEof` from exact reading | Check truncation, expected length, and outer quotas; consumed prefixes are not restored |
| Count or digest exceeds business consumption | Check for an inner observation point below buffering or repeated reads; adjust order for the intended boundary |
| Error leaves mirrored outputs different | Tee may have advanced the primary; inspect both paths and use application idempotency or compensation |
| Successful writes but missing final output | Check explicit flush/close; buffered acceptance, transfer completion, and persistence are different stages |
| Async `InvalidData` caused by WouldBlock/Interrupted | The implementation violated the async contract; fix readiness or interruption handling rather than treating it as EOF |
| Async operation never resumes | Check current-waker registration before `Pending` and whether the application keeps driving its executor |
| Incompatible SHA algorithm type | Check that the algorithm uses a compatible `digest 0.11` interface |

| Capability | Items and requirements | Feature / scope |
| --- | --- | --- |
| `Input`, `Output` | Generic items; no universal cloning bound | Default |
| `AsyncInput`, `AsyncOutput` | Generic items; poll interface | Default; application selects executor |
| Sync and async buffering | `Clone + Default` | Default |
| Sync and async limits, counting | Generic items | Default |
| `TeeInput`, `TeeOutput`, `SyncSeekTeeInput` | Generic items; seeking depends on inner capabilities | Currently synchronous |
| Sync and async checksum | `u8` + `Hasher` | Default |
| Sync and async digest | `u8` + `digest::Digest` | `digest` |
| Standard bridges and extensions | Byte streams | Default |
| Bidirectional Tokio bridges | Async byte streams | `tokio` |
| Bidirectional futures-io bridges | Async byte streams | `futures-io` |
| Generic copy utilities | `Clone + Default` | Synchronous `Streams` |

Continue with the [versioned API documentation](https://docs.rs/qubit-io/0.19.0/qubit_io/) or return to the [project overview](../README.md). The Chinese counterpart is the [简体中文用户手册](user_guide.zh_CN.md).
