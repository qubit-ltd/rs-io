/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::ffi::OsStr;
use std::path::Path;

/// File-name utility namespace.
///
/// This type is an uninstantiable namespace for lexical file-name helpers. The
/// path-based methods follow [`Path`] semantics, including Rust's handling of
/// dotfiles and non-UTF-8 names.
///
/// # Examples
/// ```
/// use qubit_io::Filenames;
/// use std::path::Path;
///
/// let path = Path::new("/tmp/archive.tar.gz");
///
/// assert_eq!(Some("archive.tar"), Filenames::file_stem_str(path));
/// assert_eq!(Some("gz"), Filenames::extension_str(path));
/// assert!(Filenames::has_extension(path, ".gz"));
/// ```
pub enum Filenames {}

impl Filenames {
    /// Returns the final file-name component of `path`.
    ///
    /// # Parameters
    /// - `path`: Path to inspect.
    ///
    /// # Returns
    /// The final file-name component, or `None` when `path` has no file-name
    /// component.
    #[inline]
    pub fn file_name(path: &Path) -> Option<&OsStr> {
        path.file_name()
    }

    /// Returns the final file-name component of `path` as UTF-8.
    ///
    /// # Parameters
    /// - `path`: Path to inspect.
    ///
    /// # Returns
    /// The final file-name component as `&str`, or `None` when `path` has no
    /// file-name component or when the component is not valid UTF-8.
    #[inline]
    pub fn file_name_str(path: &Path) -> Option<&str> {
        path.file_name().and_then(OsStr::to_str)
    }

    /// Returns the file stem of `path` as UTF-8.
    ///
    /// The stem follows [`Path::file_stem`] semantics. For example,
    /// `archive.tar.gz` has stem `archive.tar`.
    ///
    /// # Parameters
    /// - `path`: Path to inspect.
    ///
    /// # Returns
    /// The file stem as `&str`, or `None` when there is no stem or when the
    /// stem is not valid UTF-8.
    #[inline]
    pub fn file_stem_str(path: &Path) -> Option<&str> {
        path.file_stem().and_then(OsStr::to_str)
    }

    /// Returns the file prefix of `path` as UTF-8.
    ///
    /// The prefix follows [`Path::file_prefix`] semantics. For example,
    /// `archive.tar.gz` has prefix `archive`.
    ///
    /// # Parameters
    /// - `path`: Path to inspect.
    ///
    /// # Returns
    /// The file prefix as `&str`, or `None` when there is no prefix or when the
    /// prefix is not valid UTF-8.
    #[inline]
    pub fn file_prefix_str(path: &Path) -> Option<&str> {
        path.file_prefix().and_then(OsStr::to_str)
    }

    /// Returns the final extension of `path` as UTF-8.
    ///
    /// The extension follows [`Path::extension`] semantics. Dotfiles such as
    /// `.env` do not have an extension unless they contain another dot.
    ///
    /// # Parameters
    /// - `path`: Path to inspect.
    ///
    /// # Returns
    /// The extension without the leading dot, or `None` when there is no
    /// extension or when the extension is not valid UTF-8.
    #[inline]
    pub fn extension_str(path: &Path) -> Option<&str> {
        path.extension().and_then(OsStr::to_str)
    }

    /// Returns the final extension of `path` with a leading dot.
    ///
    /// This method follows [`Path::extension`] semantics. If the path has an
    /// empty extension, such as `name.`, it returns an empty string.
    ///
    /// # Parameters
    /// - `path`: Path to inspect.
    ///
    /// # Returns
    /// The extension with a leading dot, or `None` when there is no extension
    /// or when the extension is not valid UTF-8.
    pub fn dot_extension(path: &Path) -> Option<String> {
        Self::extension_str(path).map(|extension| {
            if extension.is_empty() {
                String::new()
            } else {
                format!(".{extension}")
            }
        })
    }

    /// Tests whether `path` has the specified final extension.
    ///
    /// The `extension` argument may be written with or without a leading dot.
    /// The comparison is case-sensitive.
    ///
    /// # Parameters
    /// - `path`: Path to inspect.
    /// - `extension`: Expected final extension.
    ///
    /// # Returns
    /// `true` when `path` has `extension` as its final extension.
    #[inline]
    pub fn has_extension(path: &Path, extension: &str) -> bool {
        Self::extension_str(path) == Some(normalize_extension(extension))
    }

    /// Tests whether `path` has the specified final extension, ignoring ASCII
    /// case.
    ///
    /// The `extension` argument may be written with or without a leading dot.
    /// Only ASCII case is folded; non-ASCII characters are compared exactly.
    ///
    /// # Parameters
    /// - `path`: Path to inspect.
    /// - `extension`: Expected final extension.
    ///
    /// # Returns
    /// `true` when `path` has `extension` as its final extension ignoring ASCII
    /// case.
    pub fn has_extension_ignore_ascii_case(path: &Path, extension: &str) -> bool {
        Self::extension_str(path)
            .map(|actual| actual.eq_ignore_ascii_case(normalize_extension(extension)))
            .unwrap_or(false)
    }

    /// Returns the final file-name segment from a path-like string.
    ///
    /// This is a lexical helper for strings that may contain `/` or `\`
    /// separators. It does not touch the filesystem and does not normalize the
    /// input.
    ///
    /// # Parameters
    /// - `path`: Path-like string to inspect.
    ///
    /// # Returns
    /// The substring after the final slash or backslash. If no separator is
    /// present, the original string is returned.
    #[inline]
    pub fn file_name_from_path(path: &str) -> &str {
        match path.rfind(['/', '\\']) {
            Some(index) => &path[index + 1..],
            None => path,
        }
    }

    /// Returns the final decoded file-name segment from a URL-like string.
    ///
    /// Query strings and fragments are removed before the final slash-delimited
    /// segment is selected. Percent-encoded UTF-8 sequences are decoded. If the
    /// selected segment contains invalid percent encoding or invalid UTF-8
    /// after decoding, the original selected segment is returned unchanged.
    ///
    /// # Parameters
    /// - `url`: URL-like string to inspect.
    ///
    /// # Returns
    /// The decoded final URL path segment.
    pub fn file_name_from_url(url: &str) -> String {
        let path = strip_query_and_fragment(url);
        let name = match path.rfind('/') {
            Some(index) => &path[index + 1..],
            None => path,
        };
        percent_decode_utf8(name).unwrap_or_else(|| name.to_owned())
    }
}

/// Removes one leading dot from an extension argument.
///
/// # Parameters
/// - `extension`: Extension argument supplied by a caller.
///
/// # Returns
/// The extension without one leading dot.
fn normalize_extension(extension: &str) -> &str {
    extension.strip_prefix('.').unwrap_or(extension)
}

/// Removes query and fragment suffixes from a URL-like string.
///
/// # Parameters
/// - `url`: URL-like string to inspect.
///
/// # Returns
/// The prefix before the first `?` or `#`, or the full input when neither is
/// present.
fn strip_query_and_fragment(url: &str) -> &str {
    match (url.find('?'), url.find('#')) {
        (Some(query), Some(fragment)) => &url[..query.min(fragment)],
        (Some(index), None) | (None, Some(index)) => &url[..index],
        (None, None) => url,
    }
}

/// Decodes percent-encoded UTF-8.
///
/// # Parameters
/// - `value`: Percent-encoded string.
///
/// # Returns
/// The decoded string, or `None` when the input contains malformed percent
/// encoding or decoded bytes are not valid UTF-8.
fn percent_decode_utf8(value: &str) -> Option<String> {
    if !value.as_bytes().contains(&b'%') {
        return Some(value.to_owned());
    }
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() {
                return None;
            }
            let high = hex_value(bytes[index + 1])?;
            let low = hex_value(bytes[index + 2])?;
            output.push((high << 4) | low);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(output).ok()
}

/// Converts an ASCII hexadecimal digit to its numeric value.
///
/// # Parameters
/// - `byte`: ASCII byte to convert.
///
/// # Returns
/// The hexadecimal value, or `None` when `byte` is not an ASCII hexadecimal
/// digit.
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
