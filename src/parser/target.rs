use serde::Deserialize;

use crate::error::{ErrorCode, RunError};

#[derive(Debug, Deserialize)]
pub struct Target {
    pub label: Option<String>,
    pub os: String,
    pub arch: String,
    pub environment: Option<String>,
}

pub fn parse(raw: Vec<toml::Value>) -> Result<Vec<Target>, RunError> {
    raw.into_iter()
        .map(|value| {
            value.try_into().map_err(|err| {
                RunError::new(
                    ErrorCode::ParseInvalidTarget,
                    format!("Failed to parse [[target]] entry: {err}"),
                )
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_target() {
        let value: toml::Value = toml::from_str("label = \"linux-musl\"\nos = \"linux\"\narch = \"x86_64\"\nenvironment = \"musl\"").unwrap();
        let targets = parse(vec![value]).unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].label.as_deref(), Some("linux-musl"));
        assert_eq!(targets[0].os, "linux");
        assert_eq!(targets[0].arch, "x86_64");
        assert_eq!(targets[0].environment.as_deref(), Some("musl"));
    }

    #[test]
    fn parses_target_without_optional_fields() {
        let value: toml::Value = toml::from_str("os = \"macos\"\narch = \"aarch64\"").unwrap();
        let targets = parse(vec![value]).unwrap();
        assert_eq!(targets[0].label, None);
        assert_eq!(targets[0].environment, None);
    }

    #[test]
    fn rejects_missing_required_field() {
        let value: toml::Value = toml::from_str("os = \"linux\"").unwrap();
        let err = parse(vec![value]).unwrap_err();
        assert_eq!(err.code.as_str(), "PARSE_INVALID_TARGET");
    }
}

