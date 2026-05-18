/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/

use std::fs;
use std::io::{
    Error,
    ErrorKind,
    Read,
    Write,
};
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{
    AtomicU64,
    Ordering,
};

use qubit_io::{
    atomic_write,
    atomic_write_with,
    create_buffered_writer_with_parent,
    create_file_with_parent,
    open_buffered_reader,
};

static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);
static CURRENT_DIR_LOCK: Mutex<()> = Mutex::new(());

fn temp_dir(name: &str) -> PathBuf {
    let id = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "qubit-io-file-helper-tests-{}-{name}-{id}",
        std::process::id()
    ));
    drop(fs::remove_dir_all(&path));
    fs::create_dir_all(&path).expect("temp dir should be created");
    path
}

struct CurrentDirGuard {
    original: PathBuf,
}

impl CurrentDirGuard {
    fn change_to(path: &std::path::Path) -> Self {
        let original = std::env::current_dir().expect("current dir should be readable");
        std::env::set_current_dir(path).expect("current dir should be changed");
        Self { original }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        drop(std::env::set_current_dir(&self.original));
    }
}

#[test]
fn test_atomic_write_creates_parent_directories_and_replaces_file() {
    let dir = temp_dir("atomic-replace");
    let path = dir.join("nested").join("out.txt");

    atomic_write(&path, b"first").expect("first atomic write should succeed");
    atomic_write(&path, b"second").expect("second atomic write should replace file");

    assert_eq!(b"second", fs::read(&path).unwrap().as_slice());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_atomic_write_supports_parentless_relative_path() {
    let _lock = CURRENT_DIR_LOCK
        .lock()
        .expect("current dir lock should be acquired");
    let dir = temp_dir("atomic-parentless");
    let _guard = CurrentDirGuard::change_to(&dir);

    atomic_write("out.txt", b"data").expect("parentless atomic write should succeed");

    assert_eq!(b"data", fs::read(dir.join("out.txt")).unwrap().as_slice());
    drop(_guard);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_atomic_write_with_preserves_existing_file_and_removes_temp_on_error() {
    let dir = temp_dir("atomic-error");
    let path = dir.join("out.txt");
    fs::write(&path, b"old").unwrap();

    let error = atomic_write_with(&path, |file| {
        file.write_all(b"new")?;
        Err(Error::other("write failed"))
    })
    .expect_err("writer error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("write failed", error.to_string());
    assert_eq!(b"old", fs::read(&path).unwrap().as_slice());
    let leftovers = fs::read_dir(&dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp."))
        .count();
    assert_eq!(0, leftovers);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_atomic_write_returns_temp_file_create_error() {
    let dir = temp_dir("atomic-temp-create-error");
    let process_id = std::process::id();
    for counter in 0..256 {
        fs::write(
            dir.join(format!(".atomic-write.tmp.{process_id}.{counter}")),
            b"collision",
        )
        .expect("collision temp file should be created");
    }

    let error = atomic_write(dir.join("out.txt"), b"data")
        .expect_err("temp file create errors should be returned");

    assert_eq!(ErrorKind::AlreadyExists, error.kind());
    assert!(!dir.join("out.txt").exists());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_create_file_with_parent_and_buffered_helpers() {
    let dir = temp_dir("buffered");
    let path = dir.join("a").join("b").join("data.txt");

    {
        let mut file = create_file_with_parent(&path).expect("file should be created");
        file.write_all(b"abc").unwrap();
    }

    {
        let mut writer =
            create_buffered_writer_with_parent(&path).expect("buffered writer should be created");
        writer.write_all(b"xyz").unwrap();
        writer.flush().unwrap();
    }

    let mut reader = open_buffered_reader(&path).expect("buffered reader should open");
    let mut content = Vec::new();
    reader.read_to_end(&mut content).unwrap();

    assert_eq!(b"xyz", content.as_slice());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_open_buffered_reader_returns_open_error() {
    let dir = temp_dir("open-error");

    let error = open_buffered_reader(dir.join("missing.txt"))
        .expect_err("missing file should return open error");

    assert_eq!(ErrorKind::NotFound, error.kind());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_create_file_with_parent_returns_parent_error() {
    let dir = temp_dir("parent-error");
    let file_parent = dir.join("file-parent");
    fs::write(&file_parent, b"not a directory").unwrap();

    let error = create_file_with_parent(file_parent.join("child.txt"))
        .expect_err("file parent should return create-dir error");

    assert!(matches!(
        error.kind(),
        ErrorKind::AlreadyExists | ErrorKind::NotADirectory
    ));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_atomic_write_with_returns_parent_error() {
    let dir = temp_dir("atomic-parent-error");
    let file_parent = dir.join("file-parent");
    fs::write(&file_parent, b"not a directory").unwrap();

    let error = atomic_write_with(file_parent.join("child.txt"), |_| Ok(()))
        .expect_err("file parent should return create-dir error");

    assert!(matches!(
        error.kind(),
        ErrorKind::AlreadyExists | ErrorKind::NotADirectory
    ));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_atomic_write_removes_temp_when_rename_fails() {
    let dir = temp_dir("rename-error");
    let path = dir.join("target-dir");
    fs::create_dir(&path).unwrap();

    let error = atomic_write(&path, b"data").expect_err("renaming over a directory should fail");

    assert!(matches!(
        error.kind(),
        ErrorKind::AlreadyExists
            | ErrorKind::IsADirectory
            | ErrorKind::Other
            | ErrorKind::PermissionDenied
    ));
    assert!(path.is_dir());
    let leftovers = fs::read_dir(&dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp."))
        .count();
    assert_eq!(0, leftovers);
    fs::remove_dir_all(dir).unwrap();
}
