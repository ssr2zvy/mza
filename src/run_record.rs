use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize)]
pub struct RunMetadata {
    pub run_id: String,
    pub requested_at_unix: u64,
    pub args: Vec<String>,
}

#[derive(Serialize)]
pub struct ArtifactOutcome {
    pub label: Option<String>,
    pub target: Option<String>,
    pub status: String,
    pub error_code: Option<String>,
    pub message: Option<String>,
}

#[derive(Serialize)]
pub struct BundleOutcome {
    pub label: Option<String>,
    pub status: String,
    pub error_code: Option<String>,
    pub message: Option<String>,
}

#[derive(Default, Serialize)]
pub struct RunOutcome {
    pub dry_run: bool,
    pub artifacts: Vec<ArtifactOutcome>,
    pub bundles: Vec<BundleOutcome>,
}

/// A record of a single execution, kept under archive/<run-id>/ so every
/// invocation leaves behind what was requested, what was parsed, and what happened.
pub struct RunRecord {
    dir: PathBuf,
    run_id: String,
}

impl RunRecord {
    pub fn start(artifacts_dir: &Path, args: Vec<String>) -> Result<Self, String> {
        let run_id = generate_run_id();
        let dir = artifacts_dir.join("archive").join(&run_id);
        fs::create_dir_all(&dir)
            .map_err(|err| format!("Failed to create run archive directory {}: {err}", dir.display()))?;

        let requested_at_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);

        let metadata = RunMetadata {
            run_id: run_id.clone(),
            requested_at_unix,
            args,
        };
        write_toml(&dir.join("metadata.toml"), &metadata)?;

        Ok(Self { dir, run_id })
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn record_input(&self, raw_artifacts_toml: &str) -> Result<(), String> {
        fs::write(self.dir.join("input.toml"), raw_artifacts_toml)
            .map_err(|err| format!("Failed to write input record: {err}"))
    }

    pub fn record_outcome(&self, outcome: &RunOutcome) -> Result<(), String> {
        write_toml(&self.dir.join("outcome.toml"), outcome)
    }
}

fn write_toml<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let contents = toml::to_string_pretty(value)
        .map_err(|err| format!("Failed to serialize {}: {err}", path.display()))?;
    fs::write(path, contents).map_err(|err| format!("Failed to write {}: {err}", path.display()))
}

fn generate_run_id() -> String {
    let requested_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}-{}", requested_at.as_secs(), std::process::id())
}
