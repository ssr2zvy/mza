mod cargo_bundler_v0_1_0;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::{ErrorCode, RunError};
use crate::parser::{Bundle, Target};

/// Dispatches a bundle build to its declared protocol implementation. Each
/// protocol has its own module/file, named after its protocol id.
pub fn run_bundle(
    bundle: &Bundle,
    target: &Target,
    artifacts_dir: &Path,
    run_id: &str,
    archive_paths: &HashMap<(String, String), PathBuf>,
) -> Result<PathBuf, RunError> {
    match bundle.protocol.as_str() {
        cargo_bundler_v0_1_0::PROTOCOL_ID => {
            cargo_bundler_v0_1_0::run(bundle, target, artifacts_dir, run_id, archive_paths)
        }
        other => Err(RunError::new(
            ErrorCode::BundleUnknownProtocol,
            format!(
                "Bundle \"{}\" uses unknown protocol \"{other}\"",
                bundle.label.as_deref().unwrap_or("<unlabeled>")
            ),
        )),
    }
}
