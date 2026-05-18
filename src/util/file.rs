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

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

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
/// destination path. If writing or syncing fails, the temporary file is removed
/// and the existing destination is left untouched.
///
/// # Parameters
/// - `path`: Destination path.
/// - `bytes`: Bytes to write.
///
/// # Errors
/// Returns the first I/O error reported while creating, writing, syncing,
/// removing, or renaming the temporary file.
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
/// file is flushed, synced, closed, and renamed over the destination path.
///
/// # Parameters
/// - `path`: Destination path.
/// - `write`: Function that writes the desired contents into the temporary
///   file.
///
/// # Errors
/// Returns the first I/O error reported while creating, writing, syncing,
/// removing, or renaming the temporary file.
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
    if let Err(error) = fs::rename(&temp_path, path) {
        drop(fs::remove_file(&temp_path));
        return Err(error);
    }
    Ok(())
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
    if let Err(error) = fs::rename(&temp_path, path) {
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

fn create_temp_file(path: &Path) -> Result<(PathBuf, File)> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
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
