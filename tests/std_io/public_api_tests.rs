// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io::Cursor;

use qubit_io::std_io::ReadSeek;
use qubit_io::std_io::ReadWrite;
use qubit_io::std_io::ext::ReadExt;
use qubit_io::std_io::ext::WriteExt;

/// Verifies that standard I/O integrations are available only through the
/// dedicated public module.
#[test]
fn test_std_io_exposes_standard_traits_and_extensions() {
    /// Requires the standard read-and-seek composite trait.
    fn require_read_seek<T: ReadSeek>(_: &mut T) {}

    /// Requires the standard read-and-write composite trait.
    fn require_read_write<T: ReadWrite>(_: &mut T) {}

    /// Requires the standard reader extension trait.
    fn require_read_ext<T: ReadExt>(_: &mut T) {}

    /// Requires the standard writer extension trait.
    fn require_write_ext<T: WriteExt>(_: &mut T) {}

    let mut cursor = Cursor::new(vec![1_u8, 2, 3]);
    require_read_seek(&mut cursor);
    require_read_write(&mut cursor);
    require_read_ext(&mut cursor);
    require_write_ext(&mut cursor);
}
