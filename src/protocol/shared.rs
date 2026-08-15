use std::path::{Path, PathBuf};

use crate::parser::Bundle;
use crate::shared::resolve_dir;

/// Scratch space unique per run/bundle/target, shared by every protocol:
/// <system-temp>/mza/<run_id>/<bundle_label>/<target>/
pub fn temp_workspace_dir(run_id: &str, bundle_label: &str, target: &str) -> PathBuf {
    std::env::temp_dir().join("mza").join(run_id).join(bundle_label).join(target)
}

/// Bundle output directory: <artifact_output_path>/<label>/<type>/<protocol>/<version>/<target>/
/// The protocol occupies the position an ordinary artifact's type occupies,
/// alongside (not replacing) the bundle's own `type`.
pub fn bundle_output_dir(
    bundle: &Bundle,
    bundle_label: &str,
    protocol_id: &str,
    version: &str,
    target: &str,
    artifacts_dir: &Path,
) -> PathBuf {
    resolve_dir(&bundle.artifact_output_path, artifacts_dir)
        .join(bundle_label)
        .join(bundle.r#type.as_str())
        .join(protocol_id)
        .join(version)
        .join(target)
}
