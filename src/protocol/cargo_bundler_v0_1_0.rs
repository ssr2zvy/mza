use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::archive::package_binary;
use crate::build;
use crate::error::{ErrorCode, RunError};
use crate::parser::{Bundle, Target};
use crate::shared::{ensure_cargo_lock, ensure_dir_all, resolve_dir};

use super::shared::{bundle_output_dir, temp_workspace_dir};

pub const PROTOCOL_ID: &str = "cargo-bundler-v0.1.0";

#[derive(Serialize)]
struct BundleSpec {
    protocol: String,
    bundle: String,
    target: String,
    #[serde(rename = "inputs")]
    inputs: Vec<BundleSpecInput>,
}

#[derive(Serialize)]
struct BundleSpecInput {
    label: String,
    archive: String,
}

/// Contract: mza writes a bundle-spec.toml (see
/// docs/protocols/cargo-bundler-v0.1.0.md) and sets MZA_BUNDLE_INPUTS to its
/// path. Because this crate is only ever compiled for the target (never run
/// on the build host), the spec is consumed by the crate's own `build.rs`,
/// which embeds the input archives' bytes (via `include_bytes!`) into a
/// generated file under $OUT_DIR for `main.rs` to `include!`. A build-host
/// path is never usable once the compiled artifact runs elsewhere, so only
/// embedded bytes — not paths — belong in the final compiled crate.
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

    let mut spec_inputs = Vec::with_capacity(bundle.inputs.len());
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
        spec_inputs.push(BundleSpecInput {
            label: input_label.clone(),
            archive: archive_path.display().to_string(),
        });
    }

    let workspace_dir = temp_workspace_dir(run_id, bundle_label, &triple);
    ensure_dir_all(&workspace_dir, ErrorCode::BundleExecutionFailed)?;
    let spec_path = workspace_dir.join("bundle-spec.toml");

    let spec = BundleSpec {
        protocol: PROTOCOL_ID.to_string(),
        bundle: bundle_label.to_string(),
        target: triple.clone(),
        inputs: spec_inputs,
    };
    let spec_contents = toml::to_string_pretty(&spec).map_err(|err| {
        RunError::new(
            ErrorCode::BundleExecutionFailed,
            format!("Failed to serialize bundle spec for \"{bundle_label}\": {err}"),
        )
    })?;
    fs::write(&spec_path, spec_contents).map_err(|err| {
        RunError::new(
            ErrorCode::BundleExecutionFailed,
            format!("Failed to write {}: {err}", spec_path.display()),
        )
    })?;

    let target_dir = artifacts_dir.join("target_artifacts").join(&triple);
    build::ensure_rustup_target(&triple)?;
    ensure_cargo_lock(&manifest_path)?;

    let native_macos = build::is_native_macos(target);
    let mut command = Command::new("cargo");
    command.arg(if native_macos { "build" } else { "zigbuild" });
    command
        .arg("--release")
        .arg("--locked")
        .arg("--target")
        .arg(&triple)
        .arg("--target-dir")
        .arg(&target_dir)
        .arg("--manifest-path")
        .arg(&manifest_path)
        .env("MZA_BUNDLE_INPUTS", &spec_path);

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
    let output_dir = bundle_output_dir(bundle, bundle_label, PROTOCOL_ID, &version, &triple, artifacts_dir);
    let archive_path = output_dir.join(format!("{archive_stem}.tar.xz"));

    ensure_dir_all(&output_dir, ErrorCode::BundleExecutionFailed)?;
    package_binary(&compiled_binary, &archive_path, &archive_root, &bin_name)
        .map_err(|err| RunError::new(ErrorCode::BundleExecutionFailed, err))?;

    Ok(archive_path)
}
