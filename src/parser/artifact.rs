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

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact_toml(body: &str) -> toml::Value {
        toml::from_str(body).unwrap()
    }

    #[test]
    fn parses_valid_artifact() {
        let value = artifact_toml(
            "label = \"cli\"\ncrate = \"../cli\"\nartifact_output_path = \"../out\"\ntype = \"custom\"",
        );
        let artifacts = parse(vec![value]).unwrap();
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].label.as_deref(), Some("cli"));
        assert_eq!(artifacts[0].r#type.as_str(), "custom");
        assert_eq!(artifacts[0].name, None);
        assert!(artifacts[0].exclude.is_empty());
    }

    #[test]
    fn parses_optional_name_and_exclude() {
        let value = artifact_toml(
            "label = \"cli\"\ncrate = \"../cli\"\nartifact_output_path = \"../out\"\ntype = \"main\"\nname = \"lexicon\"\nexclude = [\"windows\"]",
        );
        let artifacts = parse(vec![value]).unwrap();
        assert_eq!(artifacts[0].name.as_deref(), Some("lexicon"));
        assert_eq!(artifacts[0].exclude, vec!["windows".to_string()]);
        assert_eq!(artifacts[0].r#type.as_str(), "main");
    }

    #[test]
    fn rejects_unknown_type() {
        let value = artifact_toml("label = \"cli\"\ncrate = \"../cli\"\nartifact_output_path = \"../out\"\ntype = \"bogus\"");
        let err = parse(vec![value]).unwrap_err();
        assert_eq!(err.code.as_str(), "PARSE_INVALID_ARTIFACT");
    }

    #[test]
    fn rejects_missing_required_field() {
        let value = artifact_toml("label = \"cli\"\ntype = \"custom\"");
        let err = parse(vec![value]).unwrap_err();
        assert_eq!(err.code.as_str(), "PARSE_INVALID_ARTIFACT");
    }
}

