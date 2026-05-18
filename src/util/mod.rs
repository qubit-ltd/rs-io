/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
mod compare;
mod copy;
mod file;

pub use compare::{
    compare_content,
    content_eq,
};
pub use copy::{
    copy_at_most,
    copy_to_end_limited,
};
pub use file::{
    atomic_write,
    atomic_write_with,
    create_buffered_writer_with_parent,
    create_file_with_parent,
    open_buffered_reader,
};
