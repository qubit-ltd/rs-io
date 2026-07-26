// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
// =============================================================================
//! Tests for [`qubit_io::SyncSeekTeeInput`].

use std::io::{
    Cursor,
    SeekFrom,
};

use qubit_io::{
    Input,
    Seekable,
    SyncSeekTeeInput,
};

#[test]
fn test_sync_seek_tee_input_mirrors_reads_and_aligns_branch_position() {
    let source = Cursor::new(vec![10_u8, 20, 30]);
    let branch = Cursor::new(vec![0_u8; 3]);
    let mut input = SyncSeekTeeInput::new(source, branch);
    let mut output = [0_u8; 2];

    assert_eq!(2, input.read(&mut output).expect("read should succeed"));
    assert_eq!([10, 20], output);
    assert_eq!(2, input.branch().position());
    assert_eq!([10, 20, 0], input.branch().get_ref().as_slice());

    assert_eq!(
        1,
        input
            .seek_to(SeekFrom::Start(1))
            .expect("seek should succeed")
    );
    assert_eq!(1, input.branch().position());

    assert_eq!(
        1,
        input.read(&mut output[..1]).expect("read should succeed")
    );
    assert_eq!([20], output[..1]);
    assert_eq!(2, input.branch().position());
    assert_eq!([10, 20, 0], input.branch().get_ref().as_slice());
}
