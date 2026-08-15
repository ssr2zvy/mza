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
