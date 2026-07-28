# Migrating from Qubit IO 0.13 to 0.14

Qubit IO 0.14 unifies synchronous wrappers around generic item-stream traits
and moves standard-library-specific APIs into an explicit namespace. This is a
breaking release for code using the old wrapper names or crate-root standard
I/O traits.

## Release notes

- Synchronous reader and writer wrappers were renamed to input and output
  wrappers, and now operate on generic Qubit item streams.
- Standard-library composite and extension traits are imported from
  `qubit_io::std_io` and `qubit_io::std_io::ext`.
- `Seek` forwarding is represented by `Seekable`; legacy `BufRead` forwarding
  and `consume`-based counting and limiting were removed.
- Asynchronous wrapper names and optional Tokio and `futures-io` adapters are
  compatible.

## Rename synchronous wrappers

| 0.13 type | 0.14 type |
| --- | --- |
| `CountingReader` | `CountingInput` |
| `CountingWriter` | `CountingOutput` |
| `LimitReader` | `LimitInput` |
| `LimitWriter` | `LimitOutput` |
| `ChecksumReader` | `ChecksumInput` |
| `ChecksumWriter` | `ChecksumOutput` |
| `TeeReader` | `TeeInput` |
| `TeeWriter` | `TeeOutput` |
| `SyncSeekTeeReader` | `SyncSeekTeeInput` |

The renamed wrappers implement `Input` or `Output`, not `std::io::Read` or
`std::io::Write`. Standard byte streams remain directly usable because every
`Read` implements `Input<Item = u8>` and every `Write` implements
`Output<Item = u8>`.

```rust
use std::io::Cursor;
use qubit_io::{CountingInput, Input};

let mut input = CountingInput::new(Cursor::new(b"abc".to_vec()));
let mut bytes = [0_u8; 3];
Input::read_exactly(&mut input, &mut bytes)?;
assert_eq!(3, input.bytes_read());
# Ok::<(), std::io::Error>(())
```

Use `Input` and `Output` operations (`read`, `read_fully`, `read_exactly`,
`write`, and `write_fully`) rather than the former wrapper's standard-library
trait implementation. Fully qualified syntax avoids method-name ambiguity.

## Update standard I/O imports

The crate root keeps runtime-neutral APIs such as `Input`, `Output`, and
`Seekable`. Import standard-library APIs from these namespaces:

| API kind | 0.14 import |
| --- | --- |
| Composite traits | `qubit_io::std_io::{BufReadSeek, ReadSeek, ReadWrite, ReadWriteSeek, WriteSeek}` |
| Extension traits | `qubit_io::std_io::ext::{BufReadExt, ReadExt, ReadSeekExt, SeekExt, WriteExt, WriteSeekExt}` |

For example, replace `use qubit_io::{ReadSeek, ReadSeekExt};` with:

```rust
use qubit_io::std_io::ReadSeek;
use qubit_io::std_io::ext::ReadSeekExt;
```

## Understand item, seek, and tee behavior

`CountingInput`, `CountingOutput`, `LimitInput`, `LimitOutput`, `TeeInput`,
and `TeeOutput` support matching item types other than bytes. Count values
saturate at `u64::MAX`; checksum wrappers remain byte-only.

`Seek` forwarding has been replaced by `Seekable`. Positions use the wrapped
stream's `Seekable::Unit`; standard-library `Seek` values use `u8`.

Tee operations are ordered and non-transactional. `TeeInput` and
`SyncSeekTeeInput` advance the source before writing the branch; `TeeOutput`
writes the primary before the branch. A later failure does not roll back the
earlier operation. Add transaction or recovery behavior at a higher layer when
replication must be atomic.

Mutable wrapper accessors bypass bookkeeping. `inner_mut()` reads are not
counted, limited, hashed, or mirrored; direct branch operations can make a tee
diverge. Checksum and tee wrappers return both components through `into_parts()`;
counting and limit wrappers retain `into_inner()`.
