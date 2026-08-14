use serde::Deserialize;

use super::artifact::ArtifactType;

#[derive(Debug, Deserialize)]
pub struct Bundle {
    pub label: Option<String>,
    #[serde(rename = "crate")]
    pub crate_path: String,
    pub artifact_output_path: String,
    pub r#type: ArtifactType,
    pub protocol: String,
    pub inputs: Vec<String>,
}

pub fn parse(raw: Vec<toml::Value>) -> Result<Vec<Bundle>, String> {
    raw.into_iter()
        .map(|value| {
            value
                .try_into()
                .map_err(|err| format!("Failed to parse [[bundle]] entry: {err}"))
        })
        .collect()
}
