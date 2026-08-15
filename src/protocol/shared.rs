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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ArtifactType;

    fn bundle() -> Bundle {
        Bundle {
            label: Some("lexicon".to_string()),
            crate_path: "../..".to_string(),
            artifact_output_path: "../out".to_string(),
            r#type: ArtifactType::Custom,
            protocol: "cargo-bundler-v0.1.0".to_string(),
            inputs: vec!["cli".to_string()],
            build_targets: None,
        }
    }

    #[test]
    fn temp_workspace_dir_is_scoped_by_run_bundle_and_target() {
        let path = temp_workspace_dir("run1", "lexicon", "x86_64-unknown-linux-musl");
        assert!(path.ends_with("mza/run1/lexicon/x86_64-unknown-linux-musl"));
    }

    #[test]
    fn bundle_output_dir_places_protocol_after_type() {
        let artifacts_dir = Path::new("/artifacts");
        let dir = bundle_output_dir(&bundle(), "lexicon", "cargo-bundler-v0.1.0", "0.1.0", "x86_64-unknown-linux-musl", artifacts_dir);
        assert_eq!(
            dir,
            PathBuf::from("/artifacts/../out/lexicon/custom/cargo-bundler-v0.1.0/0.1.0/x86_64-unknown-linux-musl")
        );
    }
}

