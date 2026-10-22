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

use qubit_io::AsyncCharOutput;
use qubit_io::AsyncOutput;

use super::specialized_stream_support::ItemOutput;

fn write<O: AsyncCharOutput + ?Sized>(output: Pin<&mut O>, cx: &mut Context<'_>) -> Poll<io::Result<usize>> {
    output.poll_write(cx, &['你', '🦀'])
}

#[test]
fn test_parameter_and_borrowed_opaque_return_preserve_pinned_pending_writes() {
    fn output(items: &mut Vec<char>) -> impl AsyncCharOutput + '_ {
        ItemOutput::new(items)
    }

    let mut items = Vec::new();
    {
        let mut output = Box::pin(output(&mut items));
        let mut cx = Context::from_waker(Waker::noop());
        assert!(write(output.as_mut(), &mut cx).is_pending());
        assert!(matches!(write(output.as_mut(), &mut cx), Poll::Ready(Ok(2))));
        assert!(matches!(output.as_mut().poll_flush(&mut cx), Poll::Ready(Ok(()))));
    }
    assert_eq!(items, ['你', '🦀']);
}

#[test]
fn test_unsized_base_trait_automatically_satisfies_specialized_bound() {
    let mut items = Vec::new();
    {
        let mut output = Box::pin(ItemOutput::new(&mut items));
        let mut output: Pin<&mut dyn AsyncOutput<Item = char>> = output.as_mut();
        let mut cx = Context::from_waker(Waker::noop());
        assert!(write(output.as_mut(), &mut cx).is_pending());
        assert!(matches!(write(output.as_mut(), &mut cx), Poll::Ready(Ok(2))));
    }
    assert_eq!(items, ['你', '🦀']);
}

#[test]
fn test_specialized_trait_object_supports_pinned_writes() {
    let mut items = Vec::new();
    {
        let mut output: Pin<Box<dyn AsyncCharOutput + '_>> = Box::pin(ItemOutput::new(&mut items));
        let mut cx = Context::from_waker(Waker::noop());
        assert!(write(output.as_mut(), &mut cx).is_pending());
        assert!(matches!(write(output.as_mut(), &mut cx), Poll::Ready(Ok(2))));
    }
    assert_eq!(items, ['你', '🦀']);
}
