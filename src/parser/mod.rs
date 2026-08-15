mod artifact;
mod bundle;
mod target;

pub use artifact::{Artifact, ArtifactType};
pub use bundle::{resolve_bundle_targets, Bundle};
pub use target::Target;

use serde::Deserialize;

use crate::error::{ErrorCode, RunError};

/// Raw sections kept untyped so each dedicated parser can validate and
/// report errors scoped to its own section.
#[derive(Debug, Deserialize)]
struct RawArtifactsFile {
    #[serde(default)]
    artifact: Vec<toml::Value>,
    #[serde(default)]
    bundle: Vec<toml::Value>,
    #[serde(default)]
    target: Vec<toml::Value>,
}

pub struct ParsedConfig {
    pub artifacts: Vec<Artifact>,
    pub targets: Vec<Target>,
    pub bundles: Vec<Bundle>,
}

pub fn parse(contents: &str) -> Result<ParsedConfig, RunError> {
    let raw: RawArtifactsFile = toml::from_str(contents).map_err(|err| {
        RunError::new(
            ErrorCode::ParseInvalidToml,
            format!("Failed to parse artifacts.toml: {err}"),
        )
    })?;

    let artifacts = artifact::parse(raw.artifact)?;
    let targets = target::parse(raw.target)?;
    let bundles = bundle::parse(raw.bundle)?;

    // Each bundle's inputs must share one applicable target set; this is a
    // config-level contract, so it is validated as part of parsing.
    for bundle in &bundles {
        resolve_bundle_targets(bundle, &artifacts, &targets)?;
    }

    Ok(ParsedConfig {
        artifacts,
        targets,
        bundles,
    })
}
