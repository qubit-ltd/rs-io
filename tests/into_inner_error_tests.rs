// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::error::Error as StdError;
use std::io::{Error, ErrorKind};

use qubit_io::IntoInnerError;

#[test]
fn test_accessors_preserve_error_and_buffered_object() {
    let mut error = IntoInnerError::new(Error::other("write failed"), vec![1_u8]);

    assert_eq!("write failed", error.to_string());
    assert_eq!(ErrorKind::Other, error.error().kind());
    assert_eq!(&[1], error.inner().as_slice());
    assert_eq!(&[1], error.writer().as_slice());

    error.inner_mut().push(2);
    error.writer_mut().push(3);
    let (io_error, buffered) = error.into_parts();
    assert_eq!(ErrorKind::Other, io_error.kind());
    assert_eq!(vec![1, 2, 3], buffered);
}

#[test]
fn test_into_writer_and_into_error_return_owned_values() {
    let inner =
        IntoInnerError::new(Error::other("flush failed"), String::from("pending")).into_inner();
    assert_eq!("pending", inner);

    let writer =
        IntoInnerError::new(Error::other("flush failed"), String::from("pending")).into_writer();
    assert_eq!("pending", writer);

    let error = IntoInnerError::new(Error::other("flush failed"), ()).into_error();
    assert_eq!("flush failed", error.to_string());
}

#[test]
fn test_source_returns_retained_io_error() {
    let error = IntoInnerError::new(Error::other("write failed"), vec![1_u8]);

    let source = StdError::source(&error).expect("I/O error should be source");

    assert_eq!("write failed", source.to_string());
}
