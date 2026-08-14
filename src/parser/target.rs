use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Target {
    pub label: Option<String>,
    pub os: String,
    pub arch: String,
    pub environment: Option<String>,
}

pub fn parse(raw: Vec<toml::Value>) -> Result<Vec<Target>, String> {
    raw.into_iter()
        .map(|value| {
            value
                .try_into()
                .map_err(|err| format!("Failed to parse [[target]] entry: {err}"))
        })
        .collect()
}
