# Changelog

All notable changes to this project are documented in this file.

## 0.14.0

### Changed

- Replaced the synchronous `std::io::Read`/`Write` wrapper family with generic
  `Input`/`Output` wrappers.
- Renamed synchronous wrappers from `*Reader`/`*Writer` to `*Input`/`*Output`.
- Made limit, counting, and tee wrappers generic over item types; checksum
  wrappers remain byte-oriented because `std::hash::Hasher` consumes bytes.
- Replaced wrapper `Seek` forwarding with item-oriented `Seekable` forwarding.
- Removed the synchronous wrappers' `BufRead` forwarding behavior.
- Documented ordered, non-transactional error behavior for tee writes, flushes,
  and synchronized seeks.

The renamed wrappers and their behavior are documented in the user guide.
