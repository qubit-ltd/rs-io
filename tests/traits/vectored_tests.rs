// =============================================================================
//    Copyright (c) 2026 Haixing Hu.
//
//    SPDX-License-Identifier: Apache-2.0
//
//    Licensed under the Apache License, Version 2.0.
// =============================================================================

use std::io;

use qubit_io::Input;
use qubit_io::Output;

struct Source<T> {
    value: T,
    calls: usize,
}

impl<T: Copy> Input for Source<T> {
    type Item = T;

    unsafe fn read_unchecked(&mut self, output: &mut [T], index: usize, count: usize) -> io::Result<usize> {
        self.calls += 1;
        if count == 0 {
            return Ok(0);
        }
        output[index] = self.value;
        Ok(1)
    }
}

struct Sink<T> {
    values: Vec<T>,
    calls: usize,
}

impl<T: Copy> Output for Sink<T> {
    type Item = T;

    unsafe fn write_unchecked(&mut self, input: &[T], index: usize, count: usize) -> io::Result<usize> {
        self.calls += 1;
        if count == 0 {
            return Ok(0);
        }
        self.values.push(input[index]);
        Ok(1)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_vectored_defaults_use_first_nonempty_slice_and_keep_suffix() {
    let mut source = Source { value: '甲', calls: 0 };
    let mut first = ['!'; 2];
    let mut second = ['?'; 2];
    let mut buffers: [&mut [char]; 3] = [&mut [], &mut first, &mut second];
    assert_eq!(
        <Source<char> as Input>::read_vectored(&mut source, &mut buffers).unwrap(),
        1
    );
    assert_eq!(first, ['甲', '!']);
    assert_eq!(second, ['?', '?']);
    assert_eq!(source.calls, 1);

    let mut sink = Sink {
        values: Vec::new(),
        calls: 0,
    };
    let buffers: [&[char]; 3] = [&[], &['甲', '乙'], &['丙']];
    assert_eq!(<Sink<char> as Output>::write_vectored(&mut sink, &buffers).unwrap(), 1);
    assert_eq!(sink.values, vec!['甲']);
    assert_eq!(sink.calls, 1);
}

#[test]
fn test_vectored_empty_lists_do_not_call_inner() {
    let mut source = Source { value: 7_u8, calls: 0 };
    let mut sink = Sink::<u8> {
        values: Vec::new(),
        calls: 0,
    };
    let mut empty: [&mut [u8]; 0] = [];
    let mut all_empty: [&mut [u8]; 2] = [&mut [], &mut []];
    let empty_out: [&[u8]; 0] = [];
    let all_empty_out: [&[u8]; 2] = [&[], &[]];
    assert_eq!(
        <Source<u8> as Input>::read_vectored(&mut source, &mut empty).unwrap(),
        0
    );
    assert_eq!(
        <Source<u8> as Input>::read_vectored(&mut source, &mut all_empty).unwrap(),
        0
    );
    assert_eq!(<Sink<u8> as Output>::write_vectored(&mut sink, &empty_out).unwrap(), 0);
    assert_eq!(
        <Sink<u8> as Output>::write_vectored(&mut sink, &all_empty_out).unwrap(),
        0
    );
    assert_eq!((source.calls, sink.calls), (0, 0));
}

struct Report {
    count: usize,
    error: Option<io::ErrorKind>,
    calls: usize,
}

impl Input for Report {
    type Item = u8;

    unsafe fn read_unchecked(&mut self, _: &mut [u8], _: usize, _: usize) -> io::Result<usize> {
        self.calls += 1;
        match self.error {
            Some(kind) => Err(io::Error::from(kind)),
            None => Ok(self.count),
        }
    }
}

impl Output for Report {
    type Item = u8;

    unsafe fn write_unchecked(&mut self, _: &[u8], _: usize, _: usize) -> io::Result<usize> {
        self.calls += 1;
        match self.error {
            Some(kind) => Err(io::Error::from(kind)),
            None => Ok(self.count),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_vectored_defaults_preserve_errors_and_validate_progress() {
    for kind in [
        io::ErrorKind::Interrupted,
        io::ErrorKind::WouldBlock,
        io::ErrorKind::PermissionDenied,
    ] {
        let mut stream = Report {
            count: 0,
            error: Some(kind),
            calls: 0,
        };
        assert_eq!(
            Input::read_vectored(&mut stream, &mut [&mut [0_u8; 2][..]])
                .unwrap_err()
                .kind(),
            kind
        );
        assert_eq!(Output::write_vectored(&mut stream, &[b"ab"]).unwrap_err().kind(), kind);
        assert_eq!(stream.calls, 2);
    }

    for count in 0..=3 {
        let mut stream = Report {
            count,
            error: None,
            calls: 0,
        };
        let read = Input::read_vectored(&mut stream, &mut [&mut [0_u8; 2][..]]);
        let write = Output::write_vectored(&mut stream, &[b"ab"]);
        if count > 2 {
            assert_eq!(read.unwrap_err().kind(), io::ErrorKind::InvalidData);
            assert_eq!(write.unwrap_err().kind(), io::ErrorKind::InvalidData);
        } else {
            assert_eq!(read.unwrap(), count);
            assert_eq!(write.unwrap(), count);
        }
    }
}

struct Token;

impl Input for Token {
    type Item = Token;

    unsafe fn read_unchecked(&mut self, _: &mut [Token], _: usize, _: usize) -> io::Result<usize> {
        Ok(0)
    }
}

impl Output for Token {
    type Item = Token;

    unsafe fn write_unchecked(&mut self, _: &[Token], _: usize, _: usize) -> io::Result<usize> {
        Ok(0)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_vectored_defaults_accept_non_copy_items_and_trait_objects() {
    let mut input_value = Token;
    let mut output_value = Token;
    let input: &mut dyn Input<Item = Token> = &mut input_value;
    let output: &mut dyn Output<Item = Token> = &mut output_value;
    let mut empty_input: [&mut [Token]; 0] = [];
    let empty_output: [&[Token]; 0] = [];
    assert_eq!(
        <dyn Input<Item = Token> as Input>::read_vectored(input, &mut empty_input).unwrap(),
        0
    );
    assert_eq!(
        <dyn Output<Item = Token> as Output>::write_vectored(output, &empty_output).unwrap(),
        0
    );

    let mut byte_input = std::io::Cursor::new(b"bytes");
    let byte_input: &mut dyn qubit_io::ByteInput = &mut byte_input;
    let mut byte_output = Vec::<u8>::new();
    let byte_output: &mut dyn qubit_io::ByteOutput = &mut byte_output;
    let mut byte_buffer = [0; 2];
    assert_eq!(Input::read_vectored(byte_input, &mut [&mut byte_buffer]).unwrap(), 2);
    assert_eq!(Output::write_vectored(byte_output, &[b"ok"]).unwrap(), 2);
}

struct SafeOverride;

impl Input for SafeOverride {
    type Item = u8;

    unsafe fn read_unchecked(&mut self, _: &mut [u8], _: usize, _: usize) -> io::Result<usize> {
        Ok(0)
    }

    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Ok(3)
    }
}

impl Output for SafeOverride {
    type Item = u8;

    unsafe fn write_unchecked(&mut self, _: &[u8], _: usize, _: usize) -> io::Result<usize> {
        Ok(0)
    }

    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Ok(3)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_vectored_defaults_validate_overreported_safe_methods() {
    let mut stream = SafeOverride;
    assert_eq!(
        Input::read_vectored(&mut stream, &mut [&mut [0_u8; 2][..]])
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidData
    );
    assert_eq!(
        Output::write_vectored(&mut stream, &[b"ab"]).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
}
