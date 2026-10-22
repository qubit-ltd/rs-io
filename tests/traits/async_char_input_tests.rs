// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::task::Waker;

use qubit_io::AsyncCharInput;
use qubit_io::AsyncInput;

use super::specialized_stream_support::ItemInput;

fn read<I: AsyncCharInput + ?Sized>(
    input: Pin<&mut I>,
    cx: &mut Context<'_>,
    output: &mut [char],
) -> Poll<io::Result<usize>> {
    input.poll_read(cx, output)
}

#[test]
fn test_parameter_and_opaque_return_preserve_pinned_pending_reads() {
    fn input() -> impl AsyncCharInput {
        ItemInput::new(&['你', '🦀'])
    }

    let mut input = Box::pin(input());
    let mut cx = Context::from_waker(Waker::noop());
    let mut items = ['\0'; 2];
    assert!(read(input.as_mut(), &mut cx, &mut items).is_pending());
    assert_eq!(items, ['\0'; 2]);
    assert!(matches!(read(input.as_mut(), &mut cx, &mut items), Poll::Ready(Ok(2))));
    assert_eq!(items, ['你', '🦀']);
    assert!(matches!(read(input.as_mut(), &mut cx, &mut items), Poll::Ready(Ok(0))));
}

#[test]
fn test_unsized_base_trait_automatically_satisfies_specialized_bound() {
    let mut input = Box::pin(ItemInput::new(&['你', '🦀']));
    let input: Pin<&mut dyn AsyncInput<Item = char>> = input.as_mut();
    let mut cx = Context::from_waker(Waker::noop());
    let mut items = ['\0'; 2];
    let mut input = input;
    assert!(read(input.as_mut(), &mut cx, &mut items).is_pending());
    assert!(matches!(read(input.as_mut(), &mut cx, &mut items), Poll::Ready(Ok(2))));
    assert_eq!(items, ['你', '🦀']);
}

#[test]
fn test_specialized_trait_object_supports_pinned_reads() {
    let mut input: Pin<Box<dyn AsyncCharInput>> = Box::pin(ItemInput::new(&['你', '🦀']));
    let mut cx = Context::from_waker(Waker::noop());
    let mut items = ['\0'; 2];
    assert!(read(input.as_mut(), &mut cx, &mut items).is_pending());
    assert!(matches!(read(input.as_mut(), &mut cx, &mut items), Poll::Ready(Ok(2))));
    assert_eq!(items, ['你', '🦀']);
}
