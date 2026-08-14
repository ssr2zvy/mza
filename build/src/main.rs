use tar::Builder;
use serde::Deserialize;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;
use xz2::write::XzEncoder;

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
    #[serde(rename = "crate")]
    crate_path: String,
    artifact_output_path: String,
    r#type: ArtifactType,
    version: String,
    name: Option<String>,
    #[serde(default)]
    exclude: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum ArtifactType {
    Main,
    Snapshot,
    Custom,
}

impl ArtifactType {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Snapshot => "snapshot",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Deserialize)]
struct Target {
    label: Option<String>,
    os: String,
    arch: String,
    environment: Option<String>,
}

impl Artifact {
    fn manifest_path(&self, artifacts_dir: &Path) -> Result<PathBuf, String> {
        let crate_dir = Path::new(&self.crate_path);
        let crate_dir = if crate_dir.is_absolute() {
            crate_dir.to_path_buf()
        } else {
            artifacts_dir.join(crate_dir)
        };
        let manifest_path = crate_dir.join("Cargo.toml");

        manifest_path
            .is_file()
            .then_some(manifest_path.clone())
            .ok_or_else(|| format!("Crate directory {} does not contain Cargo.toml", crate_dir.display()))
    }

    fn is_excluded(&self, target: &Target) -> bool {
        target
            .label
            .as_ref()
            .is_some_and(|label| self.exclude.iter().any(|excluded| excluded == label))
    }

    fn output_path(&self, artifacts_dir: &Path) -> PathBuf {
        let path = Path::new(&self.artifact_output_path);
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            artifacts_dir.join(path)
        }
    }
}

impl Target {
    fn triple(&self) -> Result<String, String> {
        let arch = self.arch.as_str();
        let environment = self.environment.as_deref();

        match self.os.to_lowercase().as_str() {
            "linux" => {
                let environment = environment.unwrap_or("gnu");
                Ok(format!("{arch}-unknown-linux-{environment}"))
            }
            "windows" => {
                let environment = environment.unwrap_or("gnu");
                if environment != "gnu" {
                    return Err(format!(
                        "Unsupported windows environment \"{environment}\": only \"gnu\" is supported via Zig"
                    ));
                }
                Ok(format!("{arch}-pc-windows-{environment}"))
            }
            "macos" => Ok(format!("{arch}-apple-darwin")),
            other => Err(format!("Unsupported target os \"{other}\"")),
        }
    }

    fn is_native_macos(&self) -> bool {
        self.os.eq_ignore_ascii_case("macos") && std::env::consts::OS == "macos"
    }
}

fn package_name(manifest_path: &Path) -> Result<String, String> {
    let contents = fs::read_to_string(manifest_path)
        .map_err(|err| format!("Failed to read {}: {err}", manifest_path.display()))?;
    let manifest: toml::Value = toml::from_str(&contents)
        .map_err(|err| format!("Failed to parse {}: {err}", manifest_path.display()))?;

    manifest
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(|name| name.as_str())
        .map(str::to_string)
        .ok_or_else(|| format!("{} is missing [package].name", manifest_path.display()))
}

fn ensure_rustup_target(triple: &str) -> Result<(), String> {
    let status = Command::new("rustup")
        .args(["target", "add", triple])
        .status()
        .map_err(|err| format!("Failed to run \"rustup target add {triple}\": {err}"))?;

    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("\"rustup target add {triple}\" failed"))
}

fn package_binary(
    compiled_binary: &Path,
    archive_path: &Path,
    archive_root: &str,
    binary_name: &str,
) -> Result<(), String> {
    let archive_file = File::create(archive_path)
        .map_err(|err| format!("Failed to create {}: {err}", archive_path.display()))?;
    let mut archive = Builder::new(XzEncoder::new(archive_file, 6));

    archive
        .append_path_with_name(compiled_binary, Path::new(archive_root).join(binary_name))
        .map_err(|err| format!("Failed to add {} to archive: {err}", compiled_binary.display()))?;

    let encoder = archive
        .into_inner()
        .map_err(|err| format!("Failed to finish {}: {err}", archive_path.display()))?;
    encoder
        .finish()
        .map_err(|err| format!("Failed to finish {}: {err}", archive_path.display()))?;

    Ok(())
}

fn build_artifact(
    artifact: &Artifact,
    target: &Target,
    artifacts_dir: &Path,
) -> Result<(), String> {
    let manifest_path = artifact.manifest_path(artifacts_dir)?;
    let triple = target.triple()?;
    let target_dir = artifacts_dir.join(".build-cache").join(&triple);

    ensure_rustup_target(&triple)?;

    let native_macos = target.is_native_macos();
    let mut command = Command::new("cargo");

    if native_macos {
        command.arg("build");
    } else {
        command.arg("zigbuild");
    }

    command
        .arg("--release")
        .arg("--target")
        .arg(&triple)
        .arg("--target-dir")
        .arg(&target_dir)
        .arg("--manifest-path")
        .arg(&manifest_path);

    let status = command
        .status()
        .map_err(|err| format!("Failed to run cargo for {}/{}: {err}", manifest_path.display(), triple))?;

    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("Build failed for {} targeting {}", manifest_path.display(), triple))?;

    let bin_name = package_name(&manifest_path)?;
    let bin_file_name = if target.os.eq_ignore_ascii_case("windows") {
        format!("{bin_name}.exe")
    } else {
        bin_name
    };
    let compiled_binary = target_dir.join(&triple).join("release").join(&bin_file_name);
    let output_name = artifact.name.clone().unwrap_or(bin_file_name);
    let label = artifact
        .label
        .as_deref()
        .ok_or_else(|| format!("Artifact for {} is missing a label", manifest_path.display()))?;
    let archive_root = format!("{}-{}", output_name, artifact.version);
    let archive_stem = format!("{}-{}-{}", output_name, artifact.version, triple);
    let output_dir = artifact
        .output_path(artifacts_dir)
        .join(label)
        .join(artifact.r#type.as_str())
        .join(&artifact.version);
    let archive_path = output_dir.join(format!("{archive_stem}.tar.xz"));

    fs::create_dir_all(&output_dir)
        .map_err(|err| format!("Failed to create output directory {}: {err}", output_dir.display()))?;
    package_binary(&compiled_binary, &archive_path, &archive_root, &output_name)?;

    Ok(())
}

fn main() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let artifacts_toml_path = manifest_dir.join("../artifacts.toml");

    let contents = fs::read_to_string(&artifacts_toml_path)
        .unwrap_or_else(|err| panic!("Failed to read {}: {err}", artifacts_toml_path.display()));

    let artifacts_file: ArtifactsFile = toml::from_str(&contents)
        .unwrap_or_else(|err| panic!("Failed to parse {}: {err}", artifacts_toml_path.display()));

    let artifacts_dir = artifacts_toml_path
        .parent()
        .expect("artifacts.toml must have a parent directory");

    for artifact in &artifacts_file.artifact {
        for target in &artifacts_file.target {
            if artifact.is_excluded(target) {
                continue;
            }

            build_artifact(artifact, target, artifacts_dir)
                .unwrap_or_else(|err| panic!("{err}"));
        }
    }
}

