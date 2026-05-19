/*******************************************************************************
 *
 *    Copyright (c) 2026 Haixing Hu.
 *
 *    SPDX-License-Identifier: Apache-2.0
 *
 *    Licensed under the Apache License, Version 2.0.
 *
 ******************************************************************************/
use std::fs::{
    self,
    File,
    OpenOptions,
};
use std::io::{
    BufReader,
    BufWriter,
    Error,
    ErrorKind,
    Result,
    Write,
};
use std::path::{
    Component,
    Path,
    PathBuf,
};
use std::time::{
    SystemTime,
    UNIX_EPOCH,
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

/// Default suffix used by atomic-write temporary files.
const ATOMIC_WRITE_TEMP_SUFFIX: &str = ".tmp";

/// Prefix used by atomic-write temporary files.
const ATOMIC_WRITE_TEMP_PREFIX: &str = ".atomic-write-";

/// Number of random bytes encoded into generated temporary file names.
const RANDOM_NAME_BYTES: usize = 16;

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

/// File-system utility namespace.
///
/// This type is an uninstantiable namespace. Use its associated methods for
/// small recurring file operations, including parent creation, random temporary
/// paths, and atomic replacement writes.
///
/// # Examples
/// ```
/// use qubit_io::Files;
///
/// let dir = Files::create_temp_dir_with(Some("qubit-io-doc-"), 16)?;
/// let path = dir.join("nested").join("data.txt");
///
/// Files::atomic_write(&path, b"payload")?;
/// assert_eq!(b"payload", std::fs::read(&path)?.as_slice());
///
/// std::fs::remove_dir_all(dir)?;
/// # Ok::<(), std::io::Error>(())
/// ```
pub enum Files {}

impl Files {
    /// Default prefix used when callers do not provide a temporary file prefix.
    pub const DEFAULT_TEMP_FILE_PREFIX: &str = "qubit-io-";

    /// Default number of attempts used when creating a random temporary entry.
    pub const DEFAULT_TEMP_FILE_RETRIES: usize = 256;

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
    #[inline]
    pub fn open_buffered_reader<P>(path: P) -> Result<BufReader<File>>
    where
        P: AsRef<Path>,
    {
        File::open(path).map(BufReader::new)
    }

    /// Ensures that a directory exists.
    ///
    /// # Parameters
    /// - `path`: Directory path to create if missing.
    ///
    /// # Errors
    /// Returns an I/O error when the directory or one of its ancestors cannot
    /// be created.
    #[inline]
    pub fn ensure_dir<P>(path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        ensure_dir_path(path.as_ref())
    }

    /// Ensures that a path's parent directory exists.
    ///
    /// Parentless paths and paths whose parent is empty are accepted without
    /// creating any directory.
    ///
    /// # Parameters
    /// - `path`: File path whose parent directory should be created.
    ///
    /// # Errors
    /// Returns an I/O error when the parent directory or one of its ancestors
    /// cannot be created.
    #[inline]
    pub fn ensure_parent<P>(path: P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        ensure_parent_path(path.as_ref())
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
    /// Returns an I/O error when parent directories or the file cannot be
    /// created.
    pub fn create_file_with_parent<P>(path: P) -> Result<File>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        ensure_parent_path(path)?;
        File::create(path)
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
    /// Returns an I/O error when parent directories or the file cannot be
    /// created.
    #[inline]
    pub fn create_buffered_writer_with_parent<P>(path: P) -> Result<BufWriter<File>>
    where
        P: AsRef<Path>,
    {
        Self::create_file_with_parent(path).map(BufWriter::new)
    }

    /// Builds a random file name from an optional prefix and suffix.
    ///
    /// The generated name contains a timestamp, process id, and random
    /// hexadecimal payload. The default prefix is
    /// [`Files::DEFAULT_TEMP_FILE_PREFIX`]; the default suffix is empty.
    ///
    /// # Parameters
    /// - `prefix`: Optional name prefix.
    /// - `suffix`: Optional name suffix.
    ///
    /// # Returns
    /// A random file name string that does not contain path separators added by
    /// this function.
    ///
    /// # Panics
    /// Panics if `prefix` or `suffix` is not a safe file-name fragment, or if
    /// the operating system random source cannot provide bytes.
    pub fn random_file_name(prefix: Option<&str>, suffix: Option<&str>) -> String {
        Self::try_random_file_name(prefix, suffix).expect("failed to build random file name")
    }

    /// Tries to build a random file name from an optional prefix and suffix.
    ///
    /// The generated name contains a timestamp, process id, and random
    /// hexadecimal payload. The caller-provided prefix and suffix must be file
    /// name fragments, not paths. Path separators, root components, parent
    /// directory components, platform prefixes, and NUL bytes are rejected.
    ///
    /// # Parameters
    /// - `prefix`: Optional name prefix.
    /// - `suffix`: Optional name suffix.
    ///
    /// # Returns
    /// A random file name string.
    ///
    /// # Errors
    /// Returns [`ErrorKind::InvalidInput`] when `prefix` or `suffix` is not a
    /// safe file-name fragment. Returns [`ErrorKind::Other`] when the operating
    /// system random source cannot provide bytes.
    pub fn try_random_file_name(prefix: Option<&str>, suffix: Option<&str>) -> Result<String> {
        validate_file_name_fragment("prefix", prefix.unwrap_or(Self::DEFAULT_TEMP_FILE_PREFIX))?;
        validate_file_name_fragment("suffix", suffix.unwrap_or(""))?;
        let timestamp = unix_timestamp_nanos();
        let process_id = std::process::id();
        let random = try_random_hex()?;
        Ok(format!(
            "{}{timestamp:x}-{process_id:x}-{random}{}",
            prefix.unwrap_or(Self::DEFAULT_TEMP_FILE_PREFIX),
            suffix.unwrap_or("")
        ))
    }

    /// Returns the process temporary directory.
    ///
    /// # Returns
    /// The path reported by [`std::env::temp_dir`].
    #[inline]
    pub fn temp_dir() -> PathBuf {
        std::env::temp_dir()
    }

    /// Builds a random path inside the process temporary directory.
    ///
    /// This method only constructs a path. It does not create the file or
    /// directory, and the path may already exist by the time callers use it.
    ///
    /// # Parameters
    /// - `prefix`: Optional file-name prefix.
    /// - `suffix`: Optional file-name suffix.
    ///
    /// # Returns
    /// A random path under [`Files::temp_dir`].
    #[inline]
    pub fn temp_path(prefix: Option<&str>, suffix: Option<&str>) -> PathBuf {
        Self::temp_dir().join(Self::random_file_name(prefix, suffix))
    }

    /// Creates a random temporary file in the process temporary directory.
    ///
    /// This method returns both the created path and the open file handle so
    /// callers can write to the file and later remove or publish it. The file is
    /// created with `create_new` semantics and opened for reading and writing.
    ///
    /// # Returns
    /// The created path and file handle.
    ///
    /// # Errors
    /// Returns an I/O error when the temporary directory cannot be created, the
    /// retry limit is zero, all generated names collide, or file creation fails.
    #[inline]
    pub fn create_temp_file() -> Result<(PathBuf, File)> {
        Self::create_temp_file_with(None, None, Self::DEFAULT_TEMP_FILE_RETRIES)
    }

    /// Creates a random temporary file in the process temporary directory.
    ///
    /// # Parameters
    /// - `prefix`: Optional file-name prefix.
    /// - `suffix`: Optional file-name suffix.
    /// - `max_tries`: Maximum number of random names to try.
    ///
    /// # Returns
    /// The created path and file handle.
    ///
    /// # Errors
    /// Returns an I/O error when the temporary directory cannot be created, the
    /// retry limit is zero, all generated names collide, or file creation fails.
    #[inline]
    pub fn create_temp_file_with(
        prefix: Option<&str>,
        suffix: Option<&str>,
        max_tries: usize,
    ) -> Result<(PathBuf, File)> {
        Self::create_temp_file_in(Self::temp_dir(), prefix, suffix, max_tries)
    }

    /// Creates a random temporary file in `dir`.
    ///
    /// This method creates `dir` if it is missing. It returns both the created
    /// path and the open file handle so callers can write to the file and later
    /// remove or publish it. The file is created with `create_new` semantics and
    /// opened for reading and writing.
    ///
    /// # Parameters
    /// - `dir`: Directory in which to create the temporary file.
    /// - `prefix`: Optional file-name prefix.
    /// - `suffix`: Optional file-name suffix.
    /// - `max_tries`: Maximum number of random names to try.
    ///
    /// # Returns
    /// The created path and file handle.
    ///
    /// # Errors
    /// Returns an I/O error when `dir` cannot be created, the retry limit is
    /// zero, all generated names collide, or file creation fails.
    #[inline]
    pub fn create_temp_file_in<P>(
        dir: P,
        prefix: Option<&str>,
        suffix: Option<&str>,
        max_tries: usize,
    ) -> Result<(PathBuf, File)>
    where
        P: AsRef<Path>,
    {
        create_temp_file_in_dir(dir.as_ref(), prefix, suffix, max_tries)
    }

    /// Creates a random temporary directory in the process temporary directory.
    ///
    /// # Parameters
    /// - `prefix`: Optional directory-name prefix.
    /// - `max_tries`: Maximum number of random names to try.
    ///
    /// # Returns
    /// The created directory path.
    ///
    /// # Errors
    /// Returns an I/O error when the temporary directory cannot be created, the
    /// retry limit is zero, all generated names collide, or directory creation
    /// fails.
    #[inline]
    pub fn create_temp_dir_with(prefix: Option<&str>, max_tries: usize) -> Result<PathBuf> {
        Self::create_temp_dir_in(Self::temp_dir(), prefix, max_tries)
    }

    /// Creates a random temporary directory in `dir`.
    ///
    /// This method creates `dir` if it is missing. The random child directory is
    /// created with non-recursive creation semantics so name collisions can be
    /// detected and retried.
    ///
    /// # Parameters
    /// - `dir`: Directory in which to create the temporary directory.
    /// - `prefix`: Optional directory-name prefix.
    /// - `max_tries`: Maximum number of random names to try.
    ///
    /// # Returns
    /// The created directory path.
    ///
    /// # Errors
    /// Returns an I/O error when `dir` cannot be created, the retry limit is
    /// zero, all generated names collide, or directory creation fails.
    #[inline]
    pub fn create_temp_dir_in<P>(dir: P, prefix: Option<&str>, max_tries: usize) -> Result<PathBuf>
    where
        P: AsRef<Path>,
    {
        create_temp_dir_in_dir(dir.as_ref(), prefix, max_tries)
    }

    /// Atomically writes bytes to a path using a temporary file in the same
    /// directory.
    ///
    /// Parent directories are created before writing. The data is written to a
    /// randomly named same-directory temporary file, flushed and synced, and
    /// then renamed over the destination path with platform-specific replace
    /// semantics. After the replacement, the parent directory is synced so
    /// directory metadata reaches durable storage on platforms that support
    /// directory syncing. If writing or syncing the temporary file fails, the
    /// temporary file is removed and the existing destination is left untouched.
    /// If replacement succeeds but syncing the parent directory fails, the
    /// destination may already contain the new contents even though this method
    /// returns an error.
    ///
    /// # Parameters
    /// - `path`: Destination path.
    /// - `bytes`: Bytes to write.
    ///
    /// # Errors
    /// Returns the first I/O error reported while creating, writing, syncing,
    /// removing, replacing, or syncing the temporary file or parent directory.
    #[inline]
    pub fn atomic_write<P, B>(path: P, bytes: B) -> Result<()>
    where
        P: AsRef<Path>,
        B: AsRef<[u8]>,
    {
        atomic_write_bytes_path(path.as_ref(), bytes.as_ref())
    }

    /// Atomically writes a file using caller-provided write logic.
    ///
    /// The closure receives the temporary file. After the closure succeeds, the
    /// file is flushed, synced, closed, replaced over the destination path, and
    /// the parent directory is synced. If replacement succeeds but syncing the
    /// parent directory fails, the destination may already contain the new
    /// contents even though this method returns an error.
    ///
    /// # Parameters
    /// - `path`: Destination path.
    /// - `write`: Function that writes the desired contents into the temporary
    ///   file.
    ///
    /// # Errors
    /// Returns the first I/O error reported while creating, writing, syncing,
    /// removing, replacing, or syncing the temporary file or parent directory.
    #[inline]
    pub fn atomic_write_with<P, F>(path: P, write: F) -> Result<()>
    where
        P: AsRef<Path>,
        F: FnMut(&mut File) -> Result<()>,
    {
        let mut write = write;
        atomic_write_with_path(path.as_ref(), &mut write)
    }
}

/// Ensures that the directory at `path` exists.
///
/// # Parameters
/// - `path`: Directory path to create.
///
/// # Errors
/// Returns an I/O error when the directory or one of its ancestors cannot be
/// created.
fn ensure_dir_path(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
}

/// Ensures that the parent directory of `path` exists.
///
/// # Parameters
/// - `path`: File path whose parent directory should be created.
///
/// # Errors
/// Returns an I/O error when the parent directory or one of its ancestors cannot
/// be created.
fn ensure_parent_path(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        ensure_dir_path(parent)?;
    }
    Ok(())
}

/// Atomically writes `bytes` to `path`.
///
/// # Parameters
/// - `path`: Destination path.
/// - `bytes`: Bytes to write.
///
/// # Errors
/// Returns the first I/O error reported while writing the temporary file,
/// replacing the destination, or syncing the parent directory.
fn atomic_write_bytes_path(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut write = |file: &mut File| file.write_all(bytes);
    atomic_write_with_path(path, &mut write)
}

/// Atomically writes a file at `path` using `write`.
///
/// # Parameters
/// - `path`: Destination path.
/// - `write`: Function that writes the desired contents into the temporary file.
///
/// # Errors
/// Returns the first I/O error reported while creating, writing, syncing,
/// replacing, or syncing the temporary file or parent directory.
fn atomic_write_with_path(
    path: &Path,
    write: &mut dyn FnMut(&mut File) -> Result<()>,
) -> Result<()> {
    ensure_parent_path(path)?;
    let existing_permissions = existing_file_permissions(path)?;
    let parent = parent_dir_for(path);
    let (temp_path, mut file) = Files::create_temp_file_in(
        parent,
        Some(ATOMIC_WRITE_TEMP_PREFIX),
        Some(ATOMIC_WRITE_TEMP_SUFFIX),
        Files::DEFAULT_TEMP_FILE_RETRIES,
    )?;

    let result = write(&mut file)
        .and_then(|()| apply_existing_permissions(&file, existing_permissions.as_ref(), &temp_path))
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

/// Returns existing destination permissions to preserve during atomic writes.
///
/// # Parameters
/// - `path`: Destination file path.
///
/// # Returns
/// Existing file permissions when `path` points to a regular file.
///
/// # Errors
/// Returns an I/O error when destination metadata exists but cannot be read.
fn existing_file_permissions(path: &Path) -> Result<Option<fs::Permissions>> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(Some(metadata.permissions())),
        Ok(_) => Ok(None),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(add_path_context(error, "read destination metadata", path)),
    }
}

/// Applies preserved destination permissions to the temporary file.
///
/// # Parameters
/// - `file`: Temporary file handle.
/// - `permissions`: Optional permissions to apply.
/// - `temp_path`: Temporary file path used for error context.
///
/// # Errors
/// Returns an I/O error when permissions cannot be applied.
fn apply_existing_permissions(
    file: &File,
    permissions: Option<&fs::Permissions>,
    temp_path: &Path,
) -> Result<()> {
    if let Some(permissions) = permissions
        && let Err(error) = file.set_permissions(permissions.clone())
    {
        return Err(add_path_context(
            error,
            "set temporary file permissions",
            temp_path,
        ));
    }
    Ok(())
}

/// Creates a unique temporary file in `dir`.
///
/// # Parameters
/// - `dir`: Directory in which to create the file.
/// - `prefix`: Optional file-name prefix.
/// - `suffix`: Optional file-name suffix.
/// - `max_tries`: Maximum number of generated names to try.
///
/// # Returns
/// The created temporary path and open file handle.
///
/// # Errors
/// Returns an I/O error when `dir` cannot be created, `max_tries` is zero, all
/// generated names collide, or file creation fails.
fn create_temp_file_in_dir(
    dir: &Path,
    prefix: Option<&str>,
    suffix: Option<&str>,
    max_tries: usize,
) -> Result<(PathBuf, File)> {
    validate_max_tries(max_tries)?;
    ensure_dir_path(dir)?;
    let mut attempt = 0;
    loop {
        attempt += 1;
        let path = dir.join(Files::try_random_file_name(prefix, suffix)?);
        match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == ErrorKind::AlreadyExists && attempt < max_tries => {}
            Err(error) => return Err(add_path_context(error, "create temporary file", &path)),
        }
    }
}

/// Creates a unique temporary directory in `dir`.
///
/// # Parameters
/// - `dir`: Directory in which to create the directory.
/// - `prefix`: Optional directory-name prefix.
/// - `max_tries`: Maximum number of generated names to try.
///
/// # Returns
/// The created temporary directory path.
///
/// # Errors
/// Returns an I/O error when `dir` cannot be created, `max_tries` is zero, all
/// generated names collide, or directory creation fails.
fn create_temp_dir_in_dir(dir: &Path, prefix: Option<&str>, max_tries: usize) -> Result<PathBuf> {
    validate_max_tries(max_tries)?;
    ensure_dir_path(dir)?;
    let mut attempt = 0;
    loop {
        attempt += 1;
        let path = dir.join(Files::try_random_file_name(prefix, None)?);
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == ErrorKind::AlreadyExists && attempt < max_tries => {}
            Err(error) => return Err(add_path_context(error, "create temporary directory", &path)),
        }
    }
}

/// Validates a retry count.
///
/// # Parameters
/// - `max_tries`: Retry count to validate.
///
/// # Errors
/// Returns [`ErrorKind::InvalidInput`] when `max_tries` is zero.
fn validate_max_tries(max_tries: usize) -> Result<()> {
    if max_tries == 0 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "temporary entry retry count must be greater than zero",
        ));
    }
    Ok(())
}

/// Validates a caller-provided file-name fragment.
///
/// # Parameters
/// - `role`: Fragment role used in error messages.
/// - `fragment`: File-name fragment to validate.
///
/// # Errors
/// Returns [`ErrorKind::InvalidInput`] when `fragment` can behave like a path
/// instead of a plain file-name fragment.
fn validate_file_name_fragment(role: &str, fragment: &str) -> Result<()> {
    if fragment.contains('\0') {
        return Err(invalid_file_name_fragment_error(
            role,
            "NUL bytes are not allowed",
        ));
    }
    if fragment.contains('/') || fragment.contains('\\') {
        return Err(invalid_file_name_fragment_error(
            role,
            "path separators are not allowed",
        ));
    }
    if Path::new(fragment).components().any(|component| {
        matches!(
            component,
            Component::Prefix(_) | Component::RootDir | Component::ParentDir
        )
    }) {
        return Err(invalid_file_name_fragment_error(
            role,
            "path components are not allowed",
        ));
    }
    Ok(())
}

/// Builds an invalid file-name fragment error.
///
/// # Parameters
/// - `role`: Fragment role used in error messages.
/// - `reason`: Validation failure reason.
///
/// # Returns
/// An [`ErrorKind::InvalidInput`] error.
fn invalid_file_name_fragment_error(role: &str, reason: &str) -> Error {
    Error::new(
        ErrorKind::InvalidInput,
        format!("temporary file name {role} is invalid: {reason}"),
    )
}

/// Adds path context to an I/O error while preserving its kind.
///
/// # Parameters
/// - `error`: Original I/O error.
/// - `operation`: Operation that failed.
/// - `path`: Path involved in the operation.
///
/// # Returns
/// A new I/O error with the same [`ErrorKind`] and a more descriptive message.
fn add_path_context(error: Error, operation: &str, path: &Path) -> Error {
    Error::new(
        error.kind(),
        format!("failed to {operation} '{}': {error}", path.display()),
    )
}

/// Returns the current Unix timestamp in nanoseconds.
///
/// # Returns
/// Nanoseconds since the Unix epoch, or zero if the system clock is earlier than
/// the epoch.
fn unix_timestamp_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

/// Tries to return random bytes encoded as lowercase hexadecimal.
///
/// # Returns
/// A hexadecimal string derived from operating-system randomness.
///
/// # Errors
/// Returns [`ErrorKind::Other`] if the operating system random source cannot
/// provide bytes.
fn try_random_hex() -> Result<String> {
    let mut bytes = [0_u8; RANDOM_NAME_BYTES];
    fill_random_bytes(&mut bytes)?;
    Ok(hex_encode(&bytes))
}

/// Fills a byte slice with random bytes.
///
/// # Parameters
/// - `bytes`: Destination buffer.
///
/// # Errors
/// Returns [`ErrorKind::Other`] if the operating system random source cannot
/// provide bytes.
fn fill_random_bytes(bytes: &mut [u8]) -> Result<()> {
    getrandom::fill(bytes).map_err(Error::other)
}

/// Encodes bytes as lowercase hexadecimal.
///
/// # Parameters
/// - `bytes`: Bytes to encode.
///
/// # Returns
/// Lowercase hexadecimal string.
fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 0x0f) as usize] as char);
    }
    result
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
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        return parent;
    }
    Path::new(".")
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
