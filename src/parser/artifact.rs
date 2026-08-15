use serde::Deserialize;

use crate::error::{ErrorCode, RunError};

#[derive(Debug, Deserialize)]
pub struct Artifact {
    pub label: Option<String>,
    #[serde(rename = "crate")]
    pub crate_path: String,
    pub artifact_output_path: String,
    pub r#type: ArtifactType,
    pub name: Option<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactType {
    Main,
    Snapshot,
    Custom,
}

impl ArtifactType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Snapshot => "snapshot",
            Self::Custom => "custom",
        }
    }
}

pub fn parse(raw: Vec<toml::Value>) -> Result<Vec<Artifact>, RunError> {
    raw.into_iter()
        .map(|value| {
            value.try_into().map_err(|err| {
                RunError::new(
                    ErrorCode::ParseInvalidArtifact,
                    format!("Failed to parse [[artifact]] entry: {err}"),
                )
            })
        })
        .collect()
}
