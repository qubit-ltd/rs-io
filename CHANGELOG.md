# Changelog

## Unreleased

### Added

- Added `SyncSeekTeeReader` and `TeeReader::with_sync_branch_seek` for tee
  readers whose branch writer should seek to the source reader's resulting
  position.

### Fixed

- Fixed `BufferedInput::fill_more` on a completely full unread buffer. It now
  returns `InvalidInput` instead of panicking in debug builds or being reported
  as EOF in release builds.
- Fixed `BufferedInput` relative seeks with large offsets outside the retained
  buffer window on narrow pointer-width targets.
