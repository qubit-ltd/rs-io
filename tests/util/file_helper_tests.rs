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
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{
    AtomicU64,
    Ordering,
};

use qubit_io::Files;

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

fn count_atomic_temp_files(dir: &std::path::Path) -> usize {
    fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".atomic-write-")
        })
        .count()
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

    Files::atomic_write(&path, b"first").expect("first atomic write should succeed");
    Files::atomic_write(&path, b"second").expect("second atomic write should replace file");

    assert_eq!(b"second", fs::read(&path).unwrap().as_slice());
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn test_atomic_write_preserves_existing_file_permissions() {
    let dir = temp_dir("atomic-permissions");
    let path = dir.join("out.txt");
    fs::write(&path, b"old").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o754)).unwrap();

    Files::atomic_write(&path, b"new").expect("atomic write should preserve permissions");

    let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(0o754, mode);
    assert_eq!(b"new", fs::read(&path).unwrap().as_slice());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_atomic_write_supports_parentless_relative_path() {
    let _lock = CURRENT_DIR_LOCK
        .lock()
        .expect("current dir lock should be acquired");
    let dir = temp_dir("atomic-parentless");
    let _guard = CurrentDirGuard::change_to(&dir);

    Files::atomic_write("out.txt", b"data").expect("parentless atomic write should succeed");

    assert_eq!(b"data", fs::read(dir.join("out.txt")).unwrap().as_slice());
    drop(_guard);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_atomic_write_with_preserves_existing_file_and_removes_temp_on_error() {
    let dir = temp_dir("atomic-error");
    let path = dir.join("out.txt");
    fs::write(&path, b"old").unwrap();

    let error = Files::atomic_write_with(&path, |file| {
        file.write_all(b"new")?;
        Err(Error::other("write failed"))
    })
    .expect_err("writer error should be returned");

    assert_eq!(ErrorKind::Other, error.kind());
    assert_eq!("write failed", error.to_string());
    assert_eq!(b"old", fs::read(&path).unwrap().as_slice());
    assert_eq!(0, count_atomic_temp_files(&dir));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_random_file_name_uses_prefix_suffix_pid_and_hex_payload() {
    let name = Files::random_file_name(Some("pre-"), Some(".suf"));
    let body = name
        .strip_prefix("pre-")
        .and_then(|value| value.strip_suffix(".suf"))
        .expect("name should include requested prefix and suffix");
    let parts = body.split('-').collect::<Vec<_>>();

    assert_eq!(3, parts.len());
    assert!(!parts[0].is_empty());
    assert_eq!(format!("{:x}", std::process::id()), parts[1]);
    assert_eq!(32, parts[2].len());
    assert!(parts[2].chars().all(|ch| ch.is_ascii_hexdigit()));
}

#[test]
fn test_try_random_file_name_rejects_path_fragments() {
    let error = Files::try_random_file_name(Some("../escape-"), None)
        .expect_err("prefix with path separators should be rejected");
    assert_eq!(ErrorKind::InvalidInput, error.kind());

    let error = Files::try_random_file_name(None, Some("/suffix"))
        .expect_err("suffix with path separators should be rejected");
    assert_eq!(ErrorKind::InvalidInput, error.kind());

    let error = Files::try_random_file_name(Some("bad\0prefix"), None)
        .expect_err("prefix with NUL bytes should be rejected");
    assert_eq!(ErrorKind::InvalidInput, error.kind());

    let error = Files::try_random_file_name(Some(".."), None)
        .expect_err("parent directory component should be rejected");
    assert_eq!(ErrorKind::InvalidInput, error.kind());

    let name = Files::try_random_file_name(Some("safe-"), Some(".tmp"))
        .expect("safe fragments should be accepted");
    assert!(name.starts_with("safe-"));
    assert!(name.ends_with(".tmp"));
}

#[test]
fn test_temp_path_uses_system_temp_directory() {
    let path = Files::temp_path(Some("qubit-io-test-"), Some(".tmp"));
    let name = path
        .file_name()
        .expect("temp path should have a file name")
        .to_string_lossy();

    assert!(path.starts_with(Files::temp_dir()));
    assert!(name.starts_with("qubit-io-test-"));
    assert!(name.ends_with(".tmp"));
}

#[test]
fn test_create_temp_file_creates_unique_existing_files() {
    let (first_path, first_file) = Files::create_temp_file().expect("first temp file should exist");
    let (second_path, second_file) =
        Files::create_temp_file().expect("second temp file should exist");

    assert_ne!(first_path, second_path);
    assert!(first_path.exists());
    assert!(second_path.exists());

    drop(first_file);
    drop(second_file);
    fs::remove_file(first_path).unwrap();
    fs::remove_file(second_path).unwrap();
}

#[test]
fn test_create_temp_file_in_creates_unique_existing_files() {
    let dir = temp_dir("temp-file-in");
    let (first_path, mut first_file) =
        Files::create_temp_file_in(&dir, Some("local-"), Some(".tmp"), 4)
            .expect("first temp file should be created in dir");
    let (second_path, second_file) =
        Files::create_temp_file_in(&dir, Some("local-"), Some(".tmp"), 4)
            .expect("second temp file should be created in dir");

    first_file.write_all(b"abc").unwrap();

    assert_ne!(first_path, second_path);
    assert_eq!(Some(dir.as_path()), first_path.parent());
    assert_eq!(Some(dir.as_path()), second_path.parent());
    assert!(first_path.exists());
    assert!(second_path.exists());

    drop(first_file);
    drop(second_file);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_create_temp_dir_with_creates_existing_directory() {
    let dir = Files::create_temp_dir_with(Some("qubit-io-dir-"), 4)
        .expect("temp directory should be created");
    let name = dir
        .file_name()
        .expect("temp directory should have a name")
        .to_string_lossy();

    assert!(dir.starts_with(Files::temp_dir()));
    assert!(dir.is_dir());
    assert!(name.starts_with("qubit-io-dir-"));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_create_temp_file_with_rejects_zero_retry_count() {
    let error =
        Files::create_temp_file_with(None, None, 0).expect_err("zero retries should be invalid");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
    assert_eq!(
        "temporary entry retry count must be greater than zero",
        error.to_string()
    );
}

#[test]
fn test_create_temp_file_in_rejects_path_prefix_fragment() {
    let dir = temp_dir("temp-file-create-error");

    let error = Files::create_temp_file_in(&dir, Some("missing-parent/"), None, 1)
        .expect_err("path-like prefix should be rejected");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_create_temp_dir_in_rejects_path_prefix_fragment() {
    let dir = temp_dir("temp-dir-create-error");

    let error = Files::create_temp_dir_in(&dir, Some("missing-parent/"), 1)
        .expect_err("path-like prefix should be rejected");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_create_temp_dir_in_rejects_zero_retry_count() {
    let dir = temp_dir("temp-dir-zero-retries");

    let error =
        Files::create_temp_dir_in(&dir, None, 0).expect_err("zero retries should be invalid");

    assert_eq!(ErrorKind::InvalidInput, error.kind());
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn test_create_temp_dir_in_returns_create_error() {
    let dir = temp_dir("temp-dir-permission-error");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o500)).unwrap();

    let error = Files::create_temp_dir_in(&dir, Some("local-"), 1)
        .expect_err("unwritable directory should return create-dir error");

    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(ErrorKind::PermissionDenied, error.kind());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_create_file_with_parent_and_buffered_helpers() {
    let dir = temp_dir("buffered");
    let path = dir.join("a").join("b").join("data.txt");

    {
        let mut file = Files::create_file_with_parent(&path).expect("file should be created");
        file.write_all(b"abc").unwrap();
    }

    {
        let mut writer = Files::create_buffered_writer_with_parent(&path)
            .expect("buffered writer should be created");
        writer.write_all(b"xyz").unwrap();
        writer.flush().unwrap();
    }

    let mut reader = Files::open_buffered_reader(&path).expect("buffered reader should open");
    let mut content = Vec::new();
    reader.read_to_end(&mut content).unwrap();

    assert_eq!(b"xyz", content.as_slice());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_open_buffered_reader_returns_open_error() {
    let dir = temp_dir("open-error");

    let error = Files::open_buffered_reader(dir.join("missing.txt"))
        .expect_err("missing file should return open error");

    assert_eq!(ErrorKind::NotFound, error.kind());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_create_file_with_parent_returns_parent_error() {
    let dir = temp_dir("parent-error");
    let file_parent = dir.join("file-parent");
    fs::write(&file_parent, b"not a directory").unwrap();

    let error = Files::create_file_with_parent(file_parent.join("child.txt"))
        .expect_err("file parent should return create-dir error");

    assert!(matches!(
        error.kind(),
        ErrorKind::AlreadyExists | ErrorKind::NotADirectory
    ));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_ensure_dir_and_ensure_parent_create_missing_directories() {
    let dir = temp_dir("ensure");
    let child_dir = dir.join("a").join("b");
    let child_file = dir.join("c").join("d").join("out.txt");

    Files::ensure_dir(&child_dir).expect("directory should be created");
    Files::ensure_parent(&child_file).expect("parent should be created");

    assert!(child_dir.is_dir());
    assert!(child_file.parent().unwrap().is_dir());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_atomic_write_with_returns_parent_error() {
    let dir = temp_dir("atomic-parent-error");
    let file_parent = dir.join("file-parent");
    fs::write(&file_parent, b"not a directory").unwrap();

    let error = Files::atomic_write_with(file_parent.join("child.txt"), |_| Ok(()))
        .expect_err("file parent should return create-dir error");

    assert!(matches!(
        error.kind(),
        ErrorKind::AlreadyExists | ErrorKind::NotADirectory
    ));
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn test_atomic_write_returns_temp_create_error() {
    let dir = temp_dir("atomic-temp-create-error");
    let path = dir.join("out.txt");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o500)).unwrap();

    let error =
        Files::atomic_write(&path, b"data").expect_err("unwritable dir should fail temp creation");

    fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(ErrorKind::PermissionDenied, error.kind());
    assert!(!path.exists());
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn test_atomic_write_returns_metadata_error() {
    use std::os::unix::fs::symlink;

    let dir = temp_dir("atomic-metadata-error");
    let path = dir.join("loop");
    symlink(&path, &path).unwrap();

    let error = Files::atomic_write(&path, b"data").expect_err("symlink loop metadata should fail");

    assert!(
        error
            .to_string()
            .contains("failed to read destination metadata")
    );
    fs::remove_file(&path).unwrap();
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_atomic_write_removes_temp_when_rename_fails() {
    let dir = temp_dir("rename-error");
    let path = dir.join("target-dir");
    fs::create_dir(&path).unwrap();

    let error =
        Files::atomic_write(&path, b"data").expect_err("renaming over a directory should fail");

    assert!(matches!(
        error.kind(),
        ErrorKind::AlreadyExists
            | ErrorKind::IsADirectory
            | ErrorKind::Other
            | ErrorKind::PermissionDenied
    ));
    assert!(path.is_dir());
    assert_eq!(0, count_atomic_temp_files(&dir));
    fs::remove_dir_all(dir).unwrap();
}
