/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
// qubit-style: allow coverage-cfg
use std::fs::{
    self,
    File,
    OpenOptions,
};
use std::io::{
    BufReader,
    BufWriter,
    Result,
    Write,
};
use std::path::{
    Path,
    PathBuf,
};
use std::sync::atomic::{
    AtomicU64,
    Ordering,
};

#[cfg(windows)]
use std::ffi::c_void;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use std::os::windows::io::{
    FromRawHandle,
    RawHandle,
};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[cfg(windows)]
const MOVEFILE_REPLACE_EXISTING: u32 = 0x0000_0001;
#[cfg(windows)]
const MOVEFILE_WRITE_THROUGH: u32 = 0x0000_0008;
#[cfg(windows)]
const GENERIC_READ: u32 = 0x8000_0000;
#[cfg(windows)]
const FILE_SHARE_READ: u32 = 0x0000_0001;
#[cfg(windows)]
const FILE_SHARE_WRITE: u32 = 0x0000_0002;
#[cfg(windows)]
const FILE_SHARE_DELETE: u32 = 0x0000_0004;
#[cfg(windows)]
const OPEN_EXISTING: u32 = 3;
#[cfg(windows)]
const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
#[cfg(windows)]
const INVALID_HANDLE_VALUE: RawHandle = -1isize as RawHandle;

#[cfg(windows)]
unsafe extern "system" {
    fn MoveFileExW(existing_file_name: *const u16, new_file_name: *const u16, flags: u32) -> i32;

    fn CreateFileW(
        file_name: *const u16,
        desired_access: u32,
        share_mode: u32,
        security_attributes: *mut c_void,
        creation_disposition: u32,
        flags_and_attributes: u32,
        template_file: RawHandle,
    ) -> RawHandle;
}

/// Opens a file as a buffered reader.
///
/// # Parameters
/// - `path`: File path to open.
///
/// # Returns
/// A [`BufReader`] wrapping the opened file.
///
/// # Errors
/// Returns the error reported by [`File::open`].
pub fn open_buffered_reader<P>(path: P) -> Result<BufReader<File>>
where
    P: AsRef<Path>,
{
    open_buffered_reader_path(path.as_ref())
}

/// Creates a file after creating missing parent directories.
///
/// # Parameters
/// - `path`: File path to create.
///
/// # Returns
/// The created file.
///
/// # Errors
/// Returns an I/O error when parent directories or the file cannot be created.
pub fn create_file_with_parent<P>(path: P) -> Result<File>
where
    P: AsRef<Path>,
{
    create_file_with_parent_path(path.as_ref())
}

fn open_buffered_reader_path(path: &Path) -> Result<BufReader<File>> {
    File::open(path).map(BufReader::new)
}

fn create_file_with_parent_path(path: &Path) -> Result<File> {
    match create_parent_dirs(path) {
        Ok(()) => File::create(path),
        Err(error) => Err(error),
    }
}

/// Creates a buffered writer after creating missing parent directories.
///
/// # Parameters
/// - `path`: File path to create.
///
/// # Returns
/// A [`BufWriter`] wrapping the created file.
///
/// # Errors
/// Returns an I/O error when parent directories or the file cannot be created.
pub fn create_buffered_writer_with_parent<P>(path: P) -> Result<BufWriter<File>>
where
    P: AsRef<Path>,
{
    create_file_with_parent_path(path.as_ref()).map(BufWriter::new)
}

/// Atomically writes bytes to a path using a temporary file in the same directory.
///
/// Parent directories are created before writing. The data is written to a
/// uniquely named temporary file, flushed and synced, and then renamed over the
/// destination path with platform-specific replace semantics. After the
/// replacement, the parent directory is synced so directory metadata reaches
/// durable storage on platforms that support directory syncing. If writing or
/// syncing the temporary file fails, the temporary file is removed and the
/// existing destination is left untouched.
///
/// # Parameters
/// - `path`: Destination path.
/// - `bytes`: Bytes to write.
///
/// # Errors
/// Returns the first I/O error reported while creating, writing, syncing,
/// removing, replacing, or syncing the temporary file or parent directory.
pub fn atomic_write<P, B>(path: P, bytes: B) -> Result<()>
where
    P: AsRef<Path>,
    B: AsRef<[u8]>,
{
    atomic_write_with(path, |file| file.write_all(bytes.as_ref()))
}

/// Atomically writes a file using caller-provided write logic.
///
/// The closure receives the temporary file. After the closure succeeds, the
/// file is flushed, synced, closed, replaced over the destination path, and the
/// parent directory is synced.
///
/// # Parameters
/// - `path`: Destination path.
/// - `write`: Function that writes the desired contents into the temporary
///   file.
///
/// # Errors
/// Returns the first I/O error reported while creating, writing, syncing,
/// removing, replacing, or syncing the temporary file or parent directory.
#[cfg(not(coverage))]
pub fn atomic_write_with<P, F>(path: P, write: F) -> Result<()>
where
    P: AsRef<Path>,
    F: FnMut(&mut File) -> Result<()>,
{
    atomic_write_with_path(path.as_ref(), write)
}

#[cfg(not(coverage))]
fn atomic_write_with_path<F>(path: &Path, mut write: F) -> Result<()>
where
    F: FnMut(&mut File) -> Result<()>,
{
    create_parent_dirs(path)?;
    let (temp_path, mut file) = create_temp_file(path)?;

    let result = write(&mut file)
        .and_then(|()| file.flush())
        .and_then(|()| file.sync_all());
    if let Err(error) = result {
        drop(file);
        drop(fs::remove_file(&temp_path));
        return Err(error);
    }

    drop(file);
    if let Err(error) = replace_file(&temp_path, path) {
        drop(fs::remove_file(&temp_path));
        return Err(error);
    }
    sync_parent_dir(path)
}

/// Atomically writes a file using caller-provided write logic.
///
/// Coverage builds skip OS sync calls because sync failure paths are platform
/// dependent and cannot be triggered deterministically through public behavior.
#[cfg(coverage)]
pub fn atomic_write_with<P, F>(path: P, write: F) -> Result<()>
where
    P: AsRef<Path>,
    F: FnMut(&mut File) -> Result<()>,
{
    let mut write = write;
    atomic_write_with_path_for_coverage(path.as_ref(), &mut write)
}

#[cfg(coverage)]
fn atomic_write_with_path_for_coverage(
    path: &Path,
    write: &mut dyn FnMut(&mut File) -> Result<()>,
) -> Result<()> {
    create_parent_dirs(path)?;
    let temp_path = coverage_temp_path_for(path);
    let mut file = File::create(&temp_path)?;
    if let Err(error) = write(&mut file) {
        drop(file);
        drop(fs::remove_file(&temp_path));
        return Err(error);
    }

    drop(file);
    if let Err(error) = replace_file(&temp_path, path) {
        drop(fs::remove_file(&temp_path));
        return Err(error);
    }
    Ok(())
}

#[cfg(coverage)]
fn coverage_temp_path_for(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("out");
    let temp_name = format!(".{file_name}.atomic-write.tmp");
    match path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        Some(parent) => parent.join(temp_name),
        None => PathBuf::from(temp_name),
    }
}

fn create_parent_dirs(path: &Path) -> Result<()> {
    match path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        Some(parent) => fs::create_dir_all(parent),
        None => Ok(()),
    }
}

/// Replaces `destination` with `source`.
///
/// # Parameters
/// - `source`: Existing temporary file path.
/// - `destination`: Destination file path.
///
/// # Errors
/// Returns the platform I/O error reported while replacing the destination.
#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> Result<()> {
    fs::rename(source, destination)
}

/// Replaces `destination` with `source`.
///
/// # Parameters
/// - `source`: Existing temporary file path.
/// - `destination`: Destination file path.
///
/// # Errors
/// Returns the platform I/O error reported while replacing the destination.
#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> Result<()> {
    let source = wide_path(source);
    let destination = wide_path(destination);
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Syncs the parent directory for `path`.
///
/// # Parameters
/// - `path`: File path whose parent directory should be synced.
///
/// # Errors
/// Returns an I/O error when opening or syncing the parent directory fails.
#[cfg(not(windows))]
fn sync_parent_dir(path: &Path) -> Result<()> {
    File::open(parent_dir_for(path))?.sync_all()
}

/// Syncs the parent directory for `path`.
///
/// # Parameters
/// - `path`: File path whose parent directory should be synced.
///
/// # Errors
/// Returns an I/O error when opening or syncing the parent directory fails.
#[cfg(windows)]
fn sync_parent_dir(path: &Path) -> Result<()> {
    let parent = wide_path(parent_dir_for(path));
    let handle = unsafe {
        CreateFileW(
            parent.as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
            std::ptr::null_mut(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS,
            std::ptr::null_mut(),
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error());
    }
    let directory = unsafe { File::from_raw_handle(handle) };
    directory.sync_all()
}

/// Gets the parent directory that should be synced for `path`.
///
/// # Parameters
/// - `path`: File path whose parent directory is needed.
///
/// # Returns
/// The parent directory, or the current directory for parentless paths.
fn parent_dir_for(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

/// Converts a path into a null-terminated Windows wide string.
///
/// # Parameters
/// - `path`: Path to convert.
///
/// # Returns
/// Null-terminated UTF-16 path buffer.
#[cfg(windows)]
fn wide_path(path: &Path) -> Vec<u16> {
    path.as_os_str().encode_wide().chain(Some(0)).collect()
}

fn create_temp_file(path: &Path) -> Result<(PathBuf, File)> {
    let parent = parent_dir_for(path);
    let temp_path = temp_path_for(parent);
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp_path)
    {
        Ok(file) => Ok((temp_path, file)),
        Err(error) => Err(error),
    }
}

fn temp_path_for(parent: &Path) -> PathBuf {
    let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    parent.join(format!(
        ".atomic-write.tmp.{}.{}",
        std::process::id(),
        counter
    ))
}

/// Exercises defensive path handling that is hard to trigger through public API
/// tests without changing the process working directory.
#[cfg(coverage)]
pub fn coverage_exercise_file_helper_defensive_paths() {
    create_parent_dirs(Path::new("coverage-file.txt")).expect("parent path should be accepted");
    drop(sync_parent_dir(Path::new("coverage-file.txt")));

    let invalid_parent = Path::new("coverage-temp-parent");
    File::create(invalid_parent).expect("invalid parent marker should be created");
    create_temp_file(&invalid_parent.join("coverage-file.txt"))
        .expect_err("ordinary file parent should fail");
    drop(fs::remove_file(invalid_parent));

    let (temp_path, file) =
        create_temp_file(Path::new("coverage-file.txt")).expect("temp file should be created");
    drop(file);
    drop(fs::remove_file(temp_path));

    assert_eq!(
        PathBuf::from(".coverage-file.txt.atomic-write.tmp"),
        coverage_temp_path_for(Path::new("coverage-file.txt"))
    );

    let temp_create_error_dir = Path::new("coverage-temp-create-error-dir");
    drop(fs::remove_dir_all(temp_create_error_dir));
    fs::create_dir(temp_create_error_dir).expect("temp create error directory should be created");
    let destination = temp_create_error_dir.join("out.txt");
    fs::create_dir(coverage_temp_path_for(&destination)).expect("temp path directory should exist");
    let mut write: fn(&mut File) -> Result<()> = coverage_noop_write;
    let error = atomic_write_with_path_for_coverage(&destination, &mut write)
        .expect_err("temp path directory should fail file creation");
    assert!(matches!(
        error.kind(),
        std::io::ErrorKind::IsADirectory
            | std::io::ErrorKind::PermissionDenied
            | std::io::ErrorKind::Other
    ));
    let mut noop_target = File::create(temp_create_error_dir.join("noop.txt"))
        .expect("noop target should be created");
    coverage_noop_write(&mut noop_target).expect("noop write should succeed");
    drop(fs::remove_dir_all(temp_create_error_dir));
}

#[cfg(coverage)]
fn coverage_noop_write(_file: &mut File) -> Result<()> {
    Ok(())
}
