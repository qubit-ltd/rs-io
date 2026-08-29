// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================
//! Demonstrates a typed Map/Reduce record pipeline without byte plumbing.
//!
//! Limits, buffering, and counters operate on whole records in this pipeline.
//! The local adapters implement the low-level [`Input`] and [`Output`] safety
//! contracts, while the mapper uses only their checked methods.

use std::io;

use qubit_io::BufferedInput;
use qubit_io::CountingInput;
use qubit_io::CountingOutput;
use qubit_io::Input;
use qubit_io::LimitInput;
use qubit_io::Output;
use qubit_io::TeeOutput;

/// A typed sales record consumed by the mapper.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct Sale {
    /// Store that produced the sale.
    store_id: u32,
    /// Category associated with the sale.
    category_id: u16,
    /// Sale amount in cents.
    amount_cents: u64,
}

/// A record emitted by the mapper for the category revenue stream.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct CategoryRevenue {
    /// Category associated with the revenue.
    category_id: u16,
    /// Revenue amount in cents.
    amount_cents: u64,
}

/// Input adapter over a borrowed record slice.
///
/// # Type Parameters
///
/// - `T`: Copyable record type supplied by the slice.
struct SliceRecordInput<'a, T> {
    /// Source records.
    items: &'a [T],
    /// Index of the next unread record.
    position: usize,
}

impl<T> Input for SliceRecordInput<'_, T>
where
    T: Copy,
{
    /// Item type transferred by this adapter.
    type Item = T;

    /// Copies up to `count` unread records into the requested output range.
    ///
    /// # Parameters
    ///
    /// - `output`: Destination record storage.
    /// - `index`: First destination index.
    /// - `count`: Maximum number of records to transfer.
    ///
    /// # Returns
    ///
    /// Returns the number of copied records.
    ///
    /// # Safety
    ///
    /// `index..index + count` must be valid for `output`.
    unsafe fn read_unchecked(&mut self, output: &mut [T], index: usize, count: usize) -> io::Result<usize> {
        let read = count.min(self.items.len() - self.position);
        output[index..index + read].copy_from_slice(&self.items[self.position..self.position + read]);
        self.position += read;
        Ok(read)
    }
}

/// Output adapter that retains all accepted records in memory.
///
/// # Type Parameters
///
/// - `T`: Copyable record type retained by the output.
#[derive(Default)]
struct VecRecordOutput<T> {
    /// Records accepted by this output.
    items: Vec<T>,
}

impl<T> Output for VecRecordOutput<T>
where
    T: Copy,
{
    /// Item type accepted by this adapter.
    type Item = T;

    /// Appends the requested record range to the retained output.
    ///
    /// # Parameters
    ///
    /// - `input`: Source record storage.
    /// - `index`: First source index.
    /// - `count`: Number of records to append.
    ///
    /// # Returns
    ///
    /// Returns `count` after accepting every requested record.
    ///
    /// # Safety
    ///
    /// `index..index + count` must be valid for `input`.
    unsafe fn write_unchecked(&mut self, input: &[T], index: usize, count: usize) -> io::Result<usize> {
        self.items.extend_from_slice(&input[index..index + count]);
        Ok(count)
    }

    /// Completes the in-memory output without external I/O.
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Maps sales into category revenue records.
///
/// # Parameters
///
/// - `input`: Sales record stream.
/// - `output`: Category revenue record stream.
///
/// # Errors
///
/// Returns any error reported while reading sales or writing mapped revenue.
fn map_partition<I, O>(input: &mut I, output: &mut O) -> io::Result<()>
where
    I: Input<Item = Sale>,
    O: Output<Item = CategoryRevenue>,
{
    let mut sales = [Sale::default(); 2];
    loop {
        let count = input.read(&mut sales)?;
        if count == 0 {
            return Ok(());
        }

        let mut revenues = [CategoryRevenue::default(); 2];
        for (sale, revenue) in sales[..count].iter().zip(&mut revenues) {
            *revenue = CategoryRevenue {
                category_id: sale.category_id,
                amount_cents: sale.amount_cents,
            };
        }
        output.write_fully(&revenues[..count])?;
    }
}

/// Runs the typed record pipeline with a record limit and mirrored output.
///
/// # Errors
///
/// Returns an error when the mapper or either output reports an I/O failure.
fn main() -> io::Result<()> {
    let source = SliceRecordInput {
        items: &[
            Sale {
                store_id: 1,
                category_id: 7,
                amount_cents: 300,
            },
            Sale {
                store_id: 2,
                category_id: 7,
                amount_cents: 500,
            },
            Sale {
                store_id: 3,
                category_id: 9,
                amount_cents: 900,
            },
        ],
        position: 0,
    };
    let limited = LimitInput::new(source, 2);
    let buffered = BufferedInput::with_capacity(limited, 2);
    let mut input = CountingInput::new(buffered);

    let output = TeeOutput::new(VecRecordOutput::default(), VecRecordOutput::default());
    let mut output = CountingOutput::new(output);
    map_partition(&mut input, &mut output)?;

    assert_eq!(2, input.items_read());
    assert_eq!(2, output.items_written());
    let (shuffle, audit) = output.into_inner().into_parts();
    assert_eq!(shuffle.items, audit.items);
    assert_eq!(
        vec![
            CategoryRevenue {
                category_id: 7,
                amount_cents: 300,
            },
            CategoryRevenue {
                category_id: 7,
                amount_cents: 500,
            },
        ],
        shuffle.items,
    );
    Ok(())
}
