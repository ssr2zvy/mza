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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_dir_returns_absolute_path_untouched() {
        let base = Path::new("/artifacts");
        assert_eq!(resolve_dir("/abs/output", base), PathBuf::from("/abs/output"));
    }

    #[test]
    fn resolve_dir_joins_relative_path_to_base() {
        let base = Path::new("/artifacts");
        assert_eq!(resolve_dir("../out", base), PathBuf::from("/artifacts/../out"));
    }

    #[test]
    fn ensure_cargo_lock_finds_lockfile_beside_manifest() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Cargo.lock"), "").unwrap();
        let manifest = dir.path().join("Cargo.toml");
        fs::write(&manifest, "").unwrap();

        assert!(ensure_cargo_lock(&manifest).is_ok());
    }

    #[test]
    fn ensure_cargo_lock_finds_lockfile_in_ancestor_dir() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Cargo.lock"), "").unwrap();
        let crate_dir = dir.path().join("nested");
        fs::create_dir_all(&crate_dir).unwrap();
        let manifest = crate_dir.join("Cargo.toml");
        fs::write(&manifest, "").unwrap();

        assert!(ensure_cargo_lock(&manifest).is_ok());
    }

    #[test]
    fn ensure_cargo_lock_errors_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = dir.path().join("Cargo.toml");
        fs::write(&manifest, "").unwrap();

        let err = ensure_cargo_lock(&manifest).unwrap_err();
        assert_eq!(err.code.as_str(), "CARGO_LOCKFILE_MISSING");
    }

    #[test]
    fn ensure_dir_all_creates_nested_directories() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b");

        ensure_dir_all(&nested, ErrorCode::BundleExecutionFailed).unwrap();

        assert!(nested.is_dir());
    }
}

