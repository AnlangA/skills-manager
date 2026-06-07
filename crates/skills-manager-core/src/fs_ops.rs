//! Low-level filesystem helpers for atomic writes.
//!
//! Provides a crash-safe write primitive that writes to a temporary file
//! and renames it into place so that readers never observe a partially
//! written file.

use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use crate::Result;

/// Writes `contents` to `path` atomically by writing to a temporary file first.
///
/// The temporary file is placed in the same directory as `path` and renamed
/// into place after the write completes, guaranteeing that readers either see
/// the old content or the new content, never a partial write.
pub(crate) fn atomic_write(path: &Path, contents: impl AsRef<[u8]>) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let temporary = temporary_path(path);
    fs::write(&temporary, contents)?;
    fs::rename(&temporary, path)?;
    Ok(())
}

fn temporary_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config");
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();

    path.with_file_name(format!("{file_name}.tmp-{}-{nanos}", std::process::id()))
}
