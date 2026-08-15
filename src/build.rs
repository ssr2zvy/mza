use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::archive::package_binary;
use crate::error::{ErrorCode, RunError};
use crate::parser::{Artifact, Target};
use crate::shared::{ensure_cargo_lock, ensure_dir_all, resolve_dir};

pub fn is_excluded(artifact: &Artifact, target: &Target) -> bool {
    target
        .label
        .as_ref()
        .is_some_and(|label| artifact.exclude.iter().any(|excluded| excluded == label))
}

fn manifest_path(artifact: &Artifact, artifacts_dir: &Path) -> Result<PathBuf, RunError> {
    let crate_dir = resolve_dir(&artifact.crate_path, artifacts_dir);
    let manifest_path = crate_dir.join("Cargo.toml");

    manifest_path.is_file().then_some(manifest_path.clone()).ok_or_else(|| {
        RunError::new(
            ErrorCode::ArtifactMissingManifest,
            format!("Crate directory {} does not contain Cargo.toml", crate_dir.display()),
        )
    })
}

fn output_path(artifact: &Artifact, artifacts_dir: &Path) -> PathBuf {
    resolve_dir(&artifact.output_path, artifacts_dir)
}

pub fn triple(target: &Target) -> Result<String, RunError> {
    let arch = target.arch.as_str();
    let environment = target.environment.as_deref();

    match target.os.to_lowercase().as_str() {
        "linux" => {
            let environment = environment.unwrap_or("gnu");
            Ok(format!("{arch}-unknown-linux-{environment}"))
        }
        "windows" => {
            let environment = environment.unwrap_or("gnu");
            if environment != "gnu" {
                return Err(RunError::new(
                    ErrorCode::ArtifactUnsupportedTarget,
                    format!("Unsupported windows environment \"{environment}\": only \"gnu\" is supported via Zig"),
                ));
            }
            Ok(format!("{arch}-pc-windows-{environment}"))
        }
        "macos" => Ok(format!("{arch}-apple-darwin")),
        other => Err(RunError::new(
            ErrorCode::ArtifactUnsupportedTarget,
            format!("Unsupported target os \"{other}\""),
        )),
    }
}

pub fn is_native_macos(target: &Target) -> bool {
    target.os.eq_ignore_ascii_case("macos") && std::env::consts::OS == "macos"
}

pub fn package_metadata(manifest_path: &Path) -> Result<(String, String), RunError> {
    let contents = fs::read_to_string(manifest_path).map_err(|err| {
        RunError::new(
            ErrorCode::ArtifactMissingPackageMetadata,
            format!("Failed to read {}: {err}", manifest_path.display()),
        )
    })?;
    let manifest: toml::Value = toml::from_str(&contents).map_err(|err| {
        RunError::new(
            ErrorCode::ArtifactMissingPackageMetadata,
            format!("Failed to parse {}: {err}", manifest_path.display()),
        )
    })?;

    let package = manifest.get("package").ok_or_else(|| {
        RunError::new(
            ErrorCode::ArtifactMissingPackageMetadata,
            format!("{} is missing [package]", manifest_path.display()),
        )
    })?;
    let name = package
        .get("name")
        .and_then(|name| name.as_str())
        .map(str::to_string)
        .ok_or_else(|| {
            RunError::new(
                ErrorCode::ArtifactMissingPackageMetadata,
                format!("{} is missing [package].name", manifest_path.display()),
            )
        })?;
    let version = package
        .get("version")
        .and_then(|version| version.as_str())
        .map(str::to_string)
        .ok_or_else(|| {
            RunError::new(
                ErrorCode::ArtifactMissingPackageMetadata,
                format!("{} is missing [package].version", manifest_path.display()),
            )
        })?;

    Ok((name, version))
}

pub fn ensure_rustup_target(triple: &str) -> Result<(), RunError> {
    let status = Command::new("rustup")
        .args(["target", "add", triple])
        .status()
        .map_err(|err| {
            RunError::new(
                ErrorCode::ArtifactRustupFailed,
                format!("Failed to run \"rustup target add {triple}\": {err}"),
            )
        })?;

    status.success().then_some(()).ok_or_else(|| {
        RunError::new(
            ErrorCode::ArtifactRustupFailed,
            format!("\"rustup target add {triple}\" failed"),
        )
    })
}

pub fn build_artifact(artifact: &Artifact, target: &Target, artifacts_dir: &Path) -> Result<PathBuf, RunError> {
    let manifest_path = manifest_path(artifact, artifacts_dir)?;
    let triple = triple(target)?;
    let target_dir = artifacts_dir.join("target_artifacts").join(&triple);

    ensure_rustup_target(&triple)?;
    ensure_cargo_lock(&manifest_path)?;

    let native_macos = is_native_macos(target);
    let mut command = Command::new("cargo");

    if native_macos {
        command.arg("build");
    } else {
        command.arg("zigbuild");
    }

    command
        .arg("--release")
        .arg("--locked")
        .arg("--target")
        .arg(&triple)
        .arg("--target-dir")
        .arg(&target_dir)
        .arg("--manifest-path")
        .arg(&manifest_path);

    let status = command.status().map_err(|err| {
        RunError::new(
            ErrorCode::ArtifactCargoInvocationFailed,
            format!("Failed to run cargo for {}/{}: {err}", manifest_path.display(), triple),
        )
    })?;

    status.success().then_some(()).ok_or_else(|| {
        RunError::new(
            ErrorCode::ArtifactBuildFailed,
            format!("Build failed for {} targeting {}", manifest_path.display(), triple),
        )
    })?;

    let (bin_name, version) = package_metadata(&manifest_path)?;
    let bin_file_name = if target.os.eq_ignore_ascii_case("windows") {
        format!("{bin_name}.exe")
    } else {
        bin_name
    };
    let compiled_binary = target_dir.join(&triple).join("release").join(&bin_file_name);
    let output_name = artifact.name.clone().unwrap_or(bin_file_name);
    let label = artifact.label.as_deref().ok_or_else(|| {
        RunError::new(
            ErrorCode::ArtifactMissingLabel,
            format!("Artifact for {} is missing a label", manifest_path.display()),
        )
    })?;
    let archive_root = format!("{}-{version}", output_name);
    let archive_stem = format!("{}-{version}-{triple}", output_name);
    let output_dir = output_path(artifact, artifacts_dir)
        .join(label)
        .join(artifact.r#type.as_str())
        .join(&version);
    let archive_path = output_dir.join(format!("{archive_stem}.tar.xz"));

    ensure_dir_all(&output_dir, ErrorCode::ArtifactArchiveFailed)?;
    package_binary(&compiled_binary, &archive_path, &archive_root, &output_name)
        .map_err(|err| RunError::new(ErrorCode::ArtifactArchiveFailed, err))?;

    Ok(archive_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ArtifactType;

    fn target(label: Option<&str>, os: &str, arch: &str, environment: Option<&str>) -> Target {
        Target {
            label: label.map(str::to_string),
            os: os.to_string(),
            arch: arch.to_string(),
            environment: environment.map(str::to_string),
        }
    }

    fn artifact(exclude: Vec<&str>) -> Artifact {
        Artifact {
            label: Some("cli".to_string()),
            crate_path: "../cli".to_string(),
            output_path: "../out".to_string(),
            r#type: ArtifactType::Custom,
            name: None,
            exclude: exclude.into_iter().map(str::to_string).collect(),
        }
    }

    #[test]
    fn triple_defaults_linux_environment_to_gnu() {
        let t = target(Some("t"), "linux", "x86_64", None);
        assert_eq!(triple(&t).unwrap(), "x86_64-unknown-linux-gnu");
    }

    #[test]
    fn triple_uses_explicit_linux_environment() {
        let t = target(Some("t"), "linux", "aarch64", Some("musl"));
        assert_eq!(triple(&t).unwrap(), "aarch64-unknown-linux-musl");
    }

    #[test]
    fn triple_windows_defaults_to_gnu() {
        let t = target(Some("t"), "windows", "x86_64", None);
        assert_eq!(triple(&t).unwrap(), "x86_64-pc-windows-gnu");
    }

    #[test]
    fn triple_rejects_non_gnu_windows_environment() {
        let t = target(Some("t"), "windows", "x86_64", Some("msvc"));
        assert!(triple(&t).is_err());
    }

    #[test]
    fn triple_macos_ignores_environment() {
        let t = target(Some("t"), "macos", "aarch64", None);
        assert_eq!(triple(&t).unwrap(), "aarch64-apple-darwin");
    }

    #[test]
    fn triple_rejects_unsupported_os() {
        let t = target(Some("t"), "freebsd", "x86_64", None);
        assert!(triple(&t).is_err());
    }

    #[test]
    fn is_excluded_true_when_target_label_is_excluded() {
        let t = target(Some("linux-musl"), "linux", "x86_64", Some("musl"));
        let a = artifact(vec!["linux-musl"]);
        assert!(is_excluded(&a, &t));
    }

    #[test]
    fn is_excluded_false_when_target_label_not_excluded() {
        let t = target(Some("windows"), "windows", "x86_64", None);
        let a = artifact(vec!["linux-musl"]);
        assert!(!is_excluded(&a, &t));
    }

    #[test]
    fn is_excluded_false_when_target_has_no_label() {
        let t = target(None, "linux", "x86_64", Some("musl"));
        let a = artifact(vec!["linux-musl"]);
        assert!(!is_excluded(&a, &t));
    }
}

