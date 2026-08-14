mod artifact;
mod bundle;
mod target;

pub use artifact::{Artifact, ArtifactType};
pub use bundle::Bundle;
pub use target::Target;

use serde::Deserialize;

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

pub fn parse(contents: &str) -> Result<ParsedConfig, String> {
    let raw: RawArtifactsFile =
        toml::from_str(contents).map_err(|err| format!("Failed to parse artifacts.toml: {err}"))?;

    Ok(ParsedConfig {
        artifacts: artifact::parse(raw.artifact)?,
        targets: target::parse(raw.target)?,
        bundles: bundle::parse(raw.bundle)?,
    })
}
