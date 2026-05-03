# Qubit IO

Small I/O trait utilities for Rust.

This crate provides object-safe composition traits for common `std::io` trait
combinations:

- `ReadSeek`
- `ReadWrite`
- `WriteSeek`
- `ReadWriteSeek`

These traits are useful when an API needs a trait object such as
`&mut dyn ReadSeek` instead of a generic bound like `R: Read + Seek`.
