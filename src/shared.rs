use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{ErrorCode, RunError};

/// Resolves a possibly-relative path against the artifacts.toml directory.
/// Shared by artifact and bundle path resolution alike.
pub fn resolve_dir(raw: &str, artifacts_dir: &Path) -> PathBuf {
    let path = Path::new(raw);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        artifacts_dir.join(path)
    }
}

/// `cargo --locked` requires a Cargo.lock resolvable from the manifest;
/// checked explicitly so a missing lockfile gets its own error code instead
/// of a generic cargo invocation failure.
pub fn ensure_cargo_lock(manifest_path: &Path) -> Result<(), RunError> {
    let mut dir = manifest_path.parent().map(Path::to_path_buf);
    while let Some(current) = dir {
        if current.join("Cargo.lock").is_file() {
            return Ok(());
        }
        dir = current.parent().map(Path::to_path_buf);
    }

    Err(RunError::new(
        ErrorCode::CargoLockfileMissing,
        format!(
            "No Cargo.lock found for {} or any parent directory; required because building uses --locked",
            manifest_path.display()
        ),
    ))
}

/// Creates `path` (and parents), reporting failures under the given error code.
pub fn ensure_dir_all(path: &Path, error_code: ErrorCode) -> Result<(), RunError> {
    fs::create_dir_all(path)
        .map_err(|err| RunError::new(error_code, format!("Failed to create directory {}: {err}", path.display())))
}
