# Migrating from Qubit IO 0.13 to 0.14

Version 0.14 replaces the synchronous standard-library stream wrappers with
wrappers over Qubit IO's generic item-stream traits. This is a breaking change.

## Type mapping

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

The asynchronous wrapper names are unchanged.

## Trait changes

The renamed synchronous wrappers implement `Input` or `Output`, not
`std::io::Read` or `std::io::Write`. Standard byte streams remain directly
usable as wrapped values because every `Read` implements `Input<Item = u8>` and
every `Write` implements `Output<Item = u8>`.

Call Qubit operations through `Input::read`, `Input::read_fully`,
`Output::write`, and `Output::write_fully`. If a standard-library type brings
an identically named method into scope, use fully qualified syntax:

```rust
use std::io::Cursor;

use qubit_io::{
    CountingInput,
    Input,
};

let mut input = CountingInput::new(Cursor::new(b"abc".to_vec()));
let mut bytes = [0_u8; 3];
Input::read_exactly(&mut input, &mut bytes)?;
assert_eq!(3, input.bytes_read());
# Ok::<(), std::io::Error>(())
```

`Seek` forwarding has been replaced by `Seekable`. Positions are measured in
the wrapped stream's `Seekable::Unit`; standard `Seek` values automatically use
`u8`. The old `BufRead` forwarding and `consume`-based counting/limiting
behavior have been removed.

## Item and accessor behavior

`CountingInput`, `CountingOutput`, `LimitInput`, `LimitOutput`, `TeeInput`, and
`TeeOutput` can now wrap any matching item type. Count values saturate at
`u64::MAX`. `ChecksumInput` and `ChecksumOutput` remain restricted to `u8`.

Mutable accessors bypass wrapper bookkeeping. Reads through `inner_mut()` are
not counted, limited, hashed, or mirrored; direct branch operations can make a
tee diverge. Checksum and tee wrappers now return both owned components through
`into_parts()`. Counting and limit wrappers retain `into_inner()`.

## Tee failure behavior

Tee operations are ordered and non-transactional:

- `TeeInput` and `SyncSeekTeeInput` read the source before writing the branch.
  A branch failure leaves the source advanced and the destination modified.
- `TeeOutput` writes the primary before the branch. A branch failure leaves the
  primary output advanced.
- Tee flushes and synchronized seeks operate on the primary/source first. A
  later branch failure does not roll back the first operation.

Callers that need atomic replication must add transaction or recovery behavior
at a higher layer.
