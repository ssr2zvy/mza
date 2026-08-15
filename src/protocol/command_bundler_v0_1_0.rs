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

pub const PROTOCOL_ID: &str = "command-bundler-v0.1.0";

#[derive(Serialize)]
struct BundleSpec {
    protocol: String,
    bundle: String,
    output_path: String,
    bundle_target: BundleTargetSpec,
}

#[derive(Serialize)]
struct BundleTargetSpec {
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
/// docs/protocols/command-bundler-v0.1.0.md) and sets MZA_BUNDLE_SPEC to its path,
/// then runs `cargo run --release --locked --manifest-path <bundle Cargo.toml>`
/// on the build host (never cross-compiled). The bundle crate reads the spec,
/// invokes whatever external bundling system it wants, and must write the
/// final target executable to exactly the spec's output_path.
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
    let (_, version) = build::package_metadata(&manifest_path)?;

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
    let output_path = workspace_dir.join("output").join(bundle_label);
    if let Some(output_parent) = output_path.parent() {
        ensure_dir_all(output_parent, ErrorCode::BundleExecutionFailed)?;
    }
    let spec_path = workspace_dir.join("bundle-spec.toml");

    let spec = BundleSpec {
        protocol: PROTOCOL_ID.to_string(),
        bundle: bundle_label.to_string(),
        output_path: output_path.display().to_string(),
        bundle_target: BundleTargetSpec {
            target: triple.clone(),
            inputs: spec_inputs,
        },
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

    ensure_cargo_lock(&manifest_path)?;

    let status = Command::new("cargo")
        .arg("run")
        .arg("--release")
        .arg("--locked")
        .arg("--manifest-path")
        .arg(&manifest_path)
        .env("MZA_BUNDLE_SPEC", &spec_path)
        .status()
        .map_err(|err| {
            RunError::new(
                ErrorCode::BundleExecutionFailed,
                format!("Failed to run cargo for bundle \"{bundle_label}\": {err}"),
            )
        })?;
    if !status.success() {
        return Err(RunError::new(
            ErrorCode::BundleExecutionFailed,
            format!("Bundle \"{bundle_label}\" command failed for target \"{target_label}\""),
        ));
    }

    if !output_path.is_file() {
        return Err(RunError::new(
            ErrorCode::BundleExecutionFailed,
            format!(
                "Bundle \"{bundle_label}\" did not write its output to {}",
                output_path.display()
            ),
        ));
    }

    let output_name = bundle.name.clone().unwrap_or_else(|| bundle_label.to_string());
    let archive_root = format!("{output_name}-{version}");
    let archive_stem = format!("{output_name}-{version}-{triple}");
    let output_dir = bundle_output_dir(bundle, bundle_label, PROTOCOL_ID, &version, &triple, artifacts_dir);
    let archive_path = output_dir.join(format!("{archive_stem}.tar.xz"));

    ensure_dir_all(&output_dir, ErrorCode::BundleExecutionFailed)?;
    package_binary(&output_path, &archive_path, &archive_root, &output_name)
        .map_err(|err| RunError::new(ErrorCode::BundleExecutionFailed, err))?;

    Ok(archive_path)
}
