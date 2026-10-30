// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::cell::RefCell;
use std::io::Error;
use std::io::ErrorKind;
use std::panic::AssertUnwindSafe;
use std::panic::catch_unwind;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use qubit_io::BufferedOutput;
use qubit_io::BufferedOutputRecovery;
use qubit_io::Output;
use qubit_io::Seekable;

#[derive(Clone, Default)]
struct PanicState {
    bytes: Vec<u8>,
    calls: usize,
    panic_next_write: bool,
    panic_next_flush: bool,
}

#[derive(Clone, Default)]
struct PanicOnceSink(Arc<Mutex<PanicState>>);

impl Output for PanicOnceSink {
    type Item = u8;

    unsafe fn write_unchecked(&mut self, input: &[u8], index: usize, count: usize) -> std::io::Result<usize> {
        let should_panic = {
            let mut state = self.0.lock().expect("panic sink mutex should be available");
            state.calls += 1;
            if state.panic_next_write {
                state.panic_next_write = false;
                if count > 0 {
                    state.bytes.push(input[index]);
                }
                true
            } else {
                state.bytes.extend_from_slice(&input[index..index + count]);
                false
            }
        };
        if should_panic {
            panic!("scripted write panic");
        }
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let mut state = self.0.lock().expect("panic sink mutex should be available");
        state.calls += 1;
        let should_panic = state.panic_next_flush;
        state.panic_next_flush = false;
        drop(state);
        if should_panic {
            panic!("scripted flush panic");
        }
        Ok(())
    }
}

#[test]
fn test_panic_marks_recovery_indeterminate_and_blocks_io() {
    let sink = PanicOnceSink::default();
    let state = Arc::clone(&sink.0);
    let mut output = BufferedOutput::with_capacity(sink, 8);
    output.write_fully(b"abc").expect("buffer should accept input");
    state.lock().expect("state mutex should be available").panic_next_write = true;

    let result = catch_unwind(AssertUnwindSafe(|| output.flush()));
    assert!(result.is_err());
    let calls_after_panic = state.lock().expect("state mutex should be available").calls;

    assert_eq!(
        ErrorKind::Other,
        output.write(b"z").expect_err("poisoned write must fail").kind()
    );
    assert_eq!(
        ErrorKind::Other,
        output.flush().expect_err("poisoned flush must fail").kind()
    );
    assert_eq!(
        calls_after_panic,
        state.lock().expect("state mutex should be available").calls
    );

    let (sink, recovery) = output.into_parts();
    assert_eq!(
        b"a",
        sink.0.lock().expect("state mutex should be available").bytes.as_slice()
    );
    match recovery {
        BufferedOutputRecovery::Indeterminate(buffer) => assert_eq!(b"abc", buffer.readable()),
        BufferedOutputRecovery::Pending(_) => panic!("a panicking write must be indeterminate"),
    }
}

#[derive(Default)]
struct ErrorAfterPrefixSink {
    calls: usize,
    bytes: Vec<u8>,
}

impl Output for ErrorAfterPrefixSink {
    type Item = u8;

    unsafe fn write_unchecked(&mut self, input: &[u8], index: usize, count: usize) -> std::io::Result<usize> {
        self.calls += 1;
        if self.calls == 1 {
            let written = count.min(1);
            self.bytes.extend_from_slice(&input[index..index + written]);
            Ok(written)
        } else {
            Err(Error::other("scripted write error"))
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_regular_write_error_recovers_pending_suffix() {
    let mut output = BufferedOutput::with_capacity(ErrorAfterPrefixSink::default(), 4);
    output.write_fully(b"abc").expect("buffer should accept input");
    let error = output.flush().expect_err("second write should fail");

    let (sink, recovery) = output.into_parts();
    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!(b"a", sink.bytes.as_slice());
    match recovery {
        BufferedOutputRecovery::Pending(buffer) => assert_eq!(b"bc", buffer.readable()),
        BufferedOutputRecovery::Indeterminate(_) => panic!("ordinary errors must remain retryable"),
    }
}

#[test]
fn test_direct_write_and_inner_flush_panics_poison_all_write_paths() {
    let sink = PanicOnceSink::default();
    let state = Arc::clone(&sink.0);
    let mut output = BufferedOutput::with_capacity(sink, 4);
    state.lock().expect("state mutex should be available").panic_next_write = true;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| output.write(b"large")));
    assert!(result.is_err());

    assert_eq!(
        ErrorKind::Other,
        output.write(b"").expect_err("empty writes must check poison").kind()
    );
    // SAFETY: The empty range is valid for the empty source slice.
    assert_eq!(
        ErrorKind::Other,
        unsafe { output.write_unchecked(&[], 0, 0) }
            .expect_err("unchecked writes must check poison")
            .kind()
    );
    // SAFETY: The empty range is valid for the empty source slice.
    assert_eq!(
        ErrorKind::Other,
        unsafe { Output::write_unchecked(&mut output, &[], 0, 0) }
            .expect_err("trait writes must check poison")
            .kind()
    );
    // SAFETY: The empty range is valid for the empty source slice.
    assert_eq!(
        ErrorKind::Other,
        unsafe { Output::write_fully_unchecked(&mut output, &[], 0, 0) }
            .expect_err("trait write_fully_unchecked must check poison")
            .kind()
    );
    assert_eq!(
        ErrorKind::Other,
        output
            .write_fully(b"")
            .expect_err("write_fully must check poison")
            .kind()
    );
    assert_eq!(
        ErrorKind::Other,
        output.flush().expect_err("flush must check poison").kind()
    );
    assert_eq!(
        ErrorKind::Other,
        output
            .ensure_spare_capacity(0)
            .expect_err("capacity ensure must check poison")
            .kind()
    );
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = output.spare_raw_parts_mut();
        }))
        .is_err()
    );
    // SAFETY: The count is zero and no elements are initialized.
    assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe { output.advance(0) })).is_err());
    let calls_before_parts = state.lock().expect("state mutex should be available").calls;

    let (sink, recovery) = output.into_parts();
    assert_eq!(
        calls_before_parts,
        sink.0.lock().expect("state mutex should be available").calls
    );
    assert!(matches!(&recovery, BufferedOutputRecovery::Indeterminate(_)));
}

#[test]
fn test_inner_flush_panic_is_indeterminate_and_into_parts_does_not_retry() {
    let sink = PanicOnceSink::default();
    let state = Arc::clone(&sink.0);
    let mut output = BufferedOutput::with_capacity(sink, 4);
    output.write_fully(b"x").expect("buffer should accept input");
    state.lock().expect("state mutex should be available").panic_next_flush = true;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| output.flush()));
    assert!(result.is_err());
    let calls_after_panic = state.lock().expect("state mutex should be available").calls;
    let (sink, recovery) = output.into_parts();
    assert!(matches!(&recovery, BufferedOutputRecovery::Indeterminate(_)));
    drop(sink);
    drop(recovery);
    assert_eq!(
        calls_after_panic,
        state.lock().expect("state mutex should be available").calls
    );
}

#[test]
fn test_poisoned_buffered_output_drop_does_not_retry_pending_write() {
    let sink = PanicOnceSink::default();
    let state = Arc::clone(&sink.0);
    let mut output = BufferedOutput::with_capacity(sink, 8);
    output.write_fully(b"abc").expect("buffer accepts input");
    state.lock().expect("state lock").panic_next_write = true;
    assert!(catch_unwind(AssertUnwindSafe(|| output.flush())).is_err());
    assert_eq!(3, output.capacity() - output.spare_capacity());
    let before = {
        let state = state.lock().expect("state lock");
        (state.calls, state.bytes.clone())
    };
    assert_eq!(1, before.0);
    assert_eq!(b"a", before.1.as_slice());
    drop(output);
    let after = state.lock().expect("state lock");
    assert_eq!(before.0, after.calls, "drop must not retry inner I/O");
    assert_eq!(before.1, after.bytes, "drop must not replay uncertain bytes");
}

#[derive(Default)]
struct PanicSeekSink(AtomicBool);

impl Output for PanicSeekSink {
    type Item = u8;

    unsafe fn write_unchecked(&mut self, _input: &[u8], _index: usize, count: usize) -> std::io::Result<usize> {
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Seekable for PanicSeekSink {
    type Unit = u8;

    fn seek_to(&mut self, _position: std::io::SeekFrom) -> std::io::Result<u64> {
        if self.0.swap(false, Ordering::SeqCst) {
            panic!("scripted seek panic");
        }
        Ok(0)
    }
}

#[test]
fn test_seek_panic_is_indeterminate_and_blocks_position_queries() {
    let inner = PanicSeekSink::default();
    inner.0.store(true, Ordering::SeqCst);
    let mut output = BufferedOutput::with_capacity(inner, 4);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        output.seek_to(std::io::SeekFrom::Start(0))
    }));
    assert!(result.is_err());
    assert_eq!(
        ErrorKind::Other,
        output
            .stream_position()
            .expect_err("position query must check poison")
            .kind()
    );
    assert_eq!(
        ErrorKind::Other,
        output
            .seek_to(std::io::SeekFrom::Current(0))
            .expect_err("seek must check poison")
            .kind()
    );
    let (_, recovery) = output.into_parts();
    assert!(matches!(&recovery, BufferedOutputRecovery::Indeterminate(_)));
}

thread_local! {
    static DROP_COUNTS: RefCell<Option<Arc<(AtomicUsize, AtomicUsize)>>> = const { RefCell::new(None) };
}

struct DropItem(Arc<(AtomicUsize, AtomicUsize)>);

impl Default for DropItem {
    fn default() -> Self {
        DROP_COUNTS.with(|counts| {
            let counts = counts.borrow();
            let counts = counts.as_ref().expect("drop counters should be installed");
            counts.0.fetch_add(1, Ordering::SeqCst);
            Self(Arc::clone(counts))
        })
    }
}

impl Clone for DropItem {
    fn clone(&self) -> Self {
        self.0.0.fetch_add(1, Ordering::SeqCst);
        Self(Arc::clone(&self.0))
    }
}

impl Drop for DropItem {
    fn drop(&mut self) {
        self.0.1.fetch_add(1, Ordering::SeqCst);
    }
}

struct PanicDropSink;

impl Output for PanicDropSink {
    type Item = DropItem;

    unsafe fn write_unchecked(&mut self, _input: &[DropItem], _index: usize, _count: usize) -> std::io::Result<usize> {
        panic!("scripted non-Copy write panic");
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Default)]
struct ErrorDropSink {
    calls: usize,
}

impl Output for ErrorDropSink {
    type Item = DropItem;

    unsafe fn write_unchecked(&mut self, _input: &[DropItem], _index: usize, count: usize) -> std::io::Result<usize> {
        self.calls += 1;
        if self.calls == 1 {
            Ok(count.min(1))
        } else {
            Err(Error::other("scripted non-Copy write error"))
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_non_copy_items_are_dropped_once_after_panic_recovery() {
    let counts = Arc::new((AtomicUsize::new(0), AtomicUsize::new(0)));
    DROP_COUNTS.with(|slot| *slot.borrow_mut() = Some(Arc::clone(&counts)));
    let input = [DropItem::default()];
    let mut output = BufferedOutput::with_capacity(PanicDropSink, 4);
    output.write_fully(&input).expect("buffer should clone non-Copy item");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| output.flush()));
    assert!(result.is_err());
    let (_inner, recovery) = output.into_parts();
    assert!(matches!(recovery, BufferedOutputRecovery::Indeterminate(_)));
    drop(recovery);
    drop(input);
    assert_eq!(counts.0.load(Ordering::SeqCst), counts.1.load(Ordering::SeqCst));
    DROP_COUNTS.with(|slot| *slot.borrow_mut() = None);
}

#[test]
fn test_non_copy_items_are_dropped_once_after_pending_error_recovery() {
    let counts = Arc::new((AtomicUsize::new(0), AtomicUsize::new(0)));
    DROP_COUNTS.with(|slot| *slot.borrow_mut() = Some(Arc::clone(&counts)));
    let input = [DropItem::default(), DropItem::default()];
    let mut output = BufferedOutput::with_capacity(ErrorDropSink::default(), 4);
    output.write_fully(&input).expect("buffer should clone non-Copy items");
    let error = output.flush().expect_err("second write should fail");
    let (_inner, recovery) = output.into_parts();
    assert_eq!(ErrorKind::Other, error.kind());
    match &recovery {
        BufferedOutputRecovery::Pending(buffer) => assert_eq!(1, buffer.readable().len()),
        BufferedOutputRecovery::Indeterminate(_) => {
            panic!("normally returned write errors must remain retryable")
        }
    }
    drop(recovery);
    drop(input);
    assert_eq!(counts.0.load(Ordering::SeqCst), counts.1.load(Ordering::SeqCst));
    DROP_COUNTS.with(|slot| *slot.borrow_mut() = None);
}
