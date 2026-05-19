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

use qubit_io::Filenames;

#[test]
fn test_file_name_returns_final_component() {
    let path = Path::new("/tmp/archive.tar.gz");

    assert_eq!(
        Some(OsStr::new("archive.tar.gz")),
        Filenames::file_name(path)
    );
    assert_eq!(Some("archive.tar.gz"), Filenames::file_name_str(path));
    assert_eq!(None, Filenames::file_name(Path::new("/")));
}

#[test]
fn test_file_stem_prefix_and_extension_follow_path_semantics() {
    let path = Path::new("/tmp/archive.tar.gz");

    assert_eq!(Some("archive.tar"), Filenames::file_stem_str(path));
    assert_eq!(Some("archive"), Filenames::file_prefix_str(path));
    assert_eq!(Some("gz"), Filenames::extension_str(path));
    assert_eq!(Some(".gz".to_owned()), Filenames::dot_extension(path));
}

#[test]
fn test_extension_helpers_handle_missing_and_empty_extensions() {
    assert_eq!(None, Filenames::extension_str(Path::new("README")));
    assert_eq!(None, Filenames::dot_extension(Path::new("README")));
    assert_eq!(Some(""), Filenames::extension_str(Path::new("name.")));
    assert_eq!(
        Some(String::new()),
        Filenames::dot_extension(Path::new("name."))
    );
}

#[test]
fn test_dotfiles_follow_rust_path_semantics() {
    assert_eq!(Some(".env"), Filenames::file_stem_str(Path::new(".env")));
    assert_eq!(None, Filenames::extension_str(Path::new(".env")));

    assert_eq!(
        Some(".config"),
        Filenames::file_stem_str(Path::new(".config.toml"))
    );
    assert_eq!(
        Some("toml"),
        Filenames::extension_str(Path::new(".config.toml"))
    );
}

#[test]
fn test_has_extension_accepts_optional_leading_dot() {
    let path = Path::new("report.PDF");

    assert!(Filenames::has_extension(path, "PDF"));
    assert!(Filenames::has_extension(path, ".PDF"));
    assert!(!Filenames::has_extension(path, "pdf"));
    assert!(Filenames::has_extension_ignore_ascii_case(path, "pdf"));
    assert!(Filenames::has_extension_ignore_ascii_case(path, ".pdf"));
}

#[test]
fn test_file_name_from_path_handles_common_separators() {
    assert_eq!(
        "file.txt",
        Filenames::file_name_from_path("/tmp/data/file.txt")
    );
    assert_eq!(
        "file.txt",
        Filenames::file_name_from_path(r"C:\tmp\data\file.txt")
    );
    assert_eq!("file.txt", Filenames::file_name_from_path("file.txt"));
    assert_eq!("", Filenames::file_name_from_path("/tmp/data/"));
}

#[test]
fn test_file_name_from_url_removes_query_and_fragment() {
    assert_eq!(
        "file.txt",
        Filenames::file_name_from_url("https://example.com/path/file.txt?download=1")
    );
    assert_eq!(
        "file.txt",
        Filenames::file_name_from_url("https://example.com/path/file.txt#section")
    );
    assert_eq!(
        "file.txt",
        Filenames::file_name_from_url("https://example.com/path/file.txt?download=1#section")
    );
}

#[test]
fn test_file_name_from_url_decodes_percent_encoded_utf8() {
    assert_eq!(
        "my file.txt",
        Filenames::file_name_from_url("https://example.com/path/my%20file.txt")
    );
    assert_eq!(
        format!("caf{}.txt", '\u{00e9}'),
        Filenames::file_name_from_url("https://example.com/path/caf%C3%A9.txt")
    );
    assert_eq!(
        "file+plus.txt",
        Filenames::file_name_from_url("https://example.com/path/file%2Bplus.txt")
    );
}

#[test]
fn test_file_name_from_url_keeps_invalid_percent_encoding() {
    assert_eq!(
        "file%ZZ.txt",
        Filenames::file_name_from_url("https://example.com/path/file%ZZ.txt")
    );
    assert_eq!(
        "file%2.txt",
        Filenames::file_name_from_url("https://example.com/path/file%2.txt")
    );
}
