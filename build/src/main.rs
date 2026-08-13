use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct ArtifactsFile {
    #[serde(default)]
    artifact: Vec<Artifact>,
    #[serde(default)]
    target: Vec<Target>,
}

#[derive(Debug, Deserialize)]
struct Artifact {
    label: Option<String>,
    manifest: String,
    r#type: ArtifactType,
    version: String,
    name: Option<String>,
    #[serde(default)]
    exclude: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ArtifactType {
    Release,
    Snapshot,
    Custom,
}

#[derive(Debug, Deserialize)]
struct Target {
    label: Option<String>,
    os: String,
    arch: String,
    environment: Option<String>,
}

fn main() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let artifacts_toml_path = manifest_dir.join("../artifacts.toml");

    let contents = fs::read_to_string(&artifacts_toml_path)
        .unwrap_or_else(|err| panic!("Failed to read {}: {err}", artifacts_toml_path.display()));

    let artifacts_file: ArtifactsFile = toml::from_str(&contents)
        .unwrap_or_else(|err| panic!("Failed to parse {}: {err}", artifacts_toml_path.display()));

    println!("{artifacts_file:#?}");
}
