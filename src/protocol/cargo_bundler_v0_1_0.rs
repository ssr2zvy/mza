use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::archive::package_binary;
use crate::build;
use crate::error::{ErrorCode, RunError};
use crate::parser::{Bundle, Target};

pub const PROTOCOL_ID: &str = "cargo-bundler-v0.1.0";

/// Contract: making-rust-artifacts generates `mza_bundle_inputs.rs` (see
/// docs/protocols/cargo-bundler-v0.1.0.md) and sets MZA_BUNDLE_INPUTS
/// to its path. The bundle crate consumes it via
/// `include!(env!("MZA_BUNDLE_INPUTS"));` and is then built/archived like an artifact.
pub fn run(
    bundle: &Bundle,
    target: &Target,
    artifacts_dir: &Path,
    run_id: &str,
    archive_paths: &HashMap<(String, String), PathBuf>,
) -> Result<PathBuf, RunError> {
    let bundle_label = bundle
        .label
        .as_deref()
        .ok_or_else(|| RunError::new(ErrorCode::BundleExecutionFailed, "Bundle is missing a label".to_string()))?;
    let target_label = target.label.as_deref().ok_or_else(|| {
        RunError::new(
            ErrorCode::BundleExecutionFailed,
            format!("Bundle \"{bundle_label}\" target is missing a label"),
        )
    })?;

    let crate_dir = resolve_dir(&bundle.crate_path, artifacts_dir);
    let manifest_path = crate_dir.join("Cargo.toml");
    if !manifest_path.is_file() {
        return Err(RunError::new(
            ErrorCode::BundleExecutionFailed,
            format!(
                "Bundle \"{bundle_label}\" crate directory {} does not contain Cargo.toml",
                crate_dir.display()
            ),
        ));
    }

    let triple = build::triple(target)?;

    let mut inputs = Vec::with_capacity(bundle.inputs.len());
    for input_label in &bundle.inputs {
        let archive_path = archive_paths
            .get(&(input_label.clone(), target_label.to_string()))
            .ok_or_else(|| {
                RunError::new(
                    ErrorCode::BundleMissingInput,
                    format!(
                        "Bundle \"{bundle_label}\" input \"{input_label}\" has no archived artifact for target \"{target_label}\""
                    ),
                )
            })?;
        inputs.push((input_label.clone(), archive_path.clone()));
    }

    let inputs_rs_path = std::env::temp_dir()
        .join("making-zig-archive")
        .join(run_id)
        .join(bundle_label)
        .join("mza_bundle_inputs.rs");
    if let Some(parent) = inputs_rs_path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            RunError::new(
                ErrorCode::BundleExecutionFailed,
                format!("Failed to create {}: {err}", parent.display()),
            )
        })?;
    }
    fs::write(&inputs_rs_path, render_mza_bundle_inputs(&inputs)).map_err(|err| {
        RunError::new(
            ErrorCode::BundleExecutionFailed,
            format!("Failed to write {}: {err}", inputs_rs_path.display()),
        )
    })?;

    let target_dir = artifacts_dir.join("target_artifacts").join(&triple);
    build::ensure_rustup_target(&triple)?;

    let native_macos = build::is_native_macos(target);
    let mut command = Command::new("cargo");
    command.arg(if native_macos { "build" } else { "zigbuild" });
    command
        .arg("--release")
        .arg("--target")
        .arg(&triple)
        .arg("--target-dir")
        .arg(&target_dir)
        .arg("--manifest-path")
        .arg(&manifest_path)
        .env("MZA_BUNDLE_INPUTS", &inputs_rs_path);

    let status = command.status().map_err(|err| {
        RunError::new(
            ErrorCode::BundleExecutionFailed,
            format!("Failed to run cargo for bundle \"{bundle_label}\": {err}"),
        )
    })?;
    if !status.success() {
        return Err(RunError::new(
            ErrorCode::BundleExecutionFailed,
            format!("Bundle \"{bundle_label}\" build failed for target \"{target_label}\""),
        ));
    }

    let (bin_name, version) = build::package_metadata(&manifest_path)?;
    let bin_file_name = if target.os.eq_ignore_ascii_case("windows") {
        format!("{bin_name}.exe")
    } else {
        bin_name.clone()
    };
    let compiled_binary = target_dir.join(&triple).join("release").join(&bin_file_name);
    let archive_root = format!("{bin_name}-{version}");
    let archive_stem = format!("{bin_name}-{version}-{triple}");
    let output_dir = resolve_dir(&bundle.artifact_output_path, artifacts_dir)
        .join(bundle_label)
        .join(bundle.r#type.as_str())
        .join(&version);
    let archive_path = output_dir.join(format!("{archive_stem}.tar.xz"));

    fs::create_dir_all(&output_dir).map_err(|err| {
        RunError::new(
            ErrorCode::BundleExecutionFailed,
            format!("Failed to create output directory {}: {err}", output_dir.display()),
        )
    })?;
    package_binary(&compiled_binary, &archive_path, &archive_root, &bin_name)
        .map_err(|err| RunError::new(ErrorCode::BundleExecutionFailed, err))?;

    Ok(archive_path)
}

fn resolve_dir(raw: &str, artifacts_dir: &Path) -> PathBuf {
    let path = Path::new(raw);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        artifacts_dir.join(path)
    }
}

fn render_mza_bundle_inputs(inputs: &[(String, PathBuf)]) -> String {
    let mut out = String::new();
    out.push_str("// Generated by making-rust-artifacts (protocol cargo-bundler-v0.1.0). Do not edit.\n");
    out.push_str("pub struct MzaBundleInput {\n    pub label: &'static str,\n    pub archive: &'static str,\n}\n\n");
    out.push_str("pub static MZA_BUNDLE_INPUTS: &[MzaBundleInput] = &[\n");
    for (label, archive_path) in inputs {
        out.push_str(&format!(
            "    MzaBundleInput {{ label: \"{label}\", archive: \"{}\" }},\n",
            archive_path.display()
        ));
    }
    out.push_str("];\n");
    out
}
