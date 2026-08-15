use std::collections::BTreeSet;

use serde::Deserialize;

use super::artifact::{Artifact, ArtifactType};
use super::target::Target;
use crate::error::{ErrorCode, RunError};

#[derive(Debug, Deserialize)]
pub struct Bundle {
    pub label: Option<String>,
    #[serde(rename = "crate")]
    pub crate_path: String,
    pub artifact_output_path: String,
    pub r#type: ArtifactType,
    pub protocol: String,
    pub inputs: Vec<String>,
}

pub fn parse(raw: Vec<toml::Value>) -> Result<Vec<Bundle>, RunError> {
    raw.into_iter()
        .map(|value| {
            value.try_into().map_err(|err| {
                RunError::new(
                    ErrorCode::ParseInvalidBundle,
                    format!("Failed to parse [[bundle]] entry: {err}"),
                )
            })
        })
        .collect()
}

fn applicable_target_labels<'a>(artifact: &Artifact, targets: &'a [Target]) -> BTreeSet<&'a str> {
    targets
        .iter()
        .filter_map(|target| target.label.as_deref())
        .filter(|label| !artifact.exclude.iter().any(|excluded| excluded == label))
        .collect()
}

/// Resolves the [[target]] entries shared by every one of a bundle's inputs.
/// A bundle's inputs must all apply to the exact same set of targets; this is
/// a config-level contract, so mismatches are reported as parse errors.
pub fn resolve_bundle_targets<'a>(
    bundle: &Bundle,
    artifacts: &[Artifact],
    targets: &'a [Target],
) -> Result<Vec<&'a Target>, RunError> {
    let bundle_label = bundle.label.as_deref().unwrap_or("<unlabeled>");

    let mut input_artifacts = Vec::with_capacity(bundle.inputs.len());
    for input_label in &bundle.inputs {
        let artifact = artifacts
            .iter()
            .find(|artifact| artifact.label.as_deref() == Some(input_label.as_str()))
            .ok_or_else(|| {
                RunError::new(
                    ErrorCode::ParseInvalidBundle,
                    format!(
                        "Bundle \"{bundle_label}\" input \"{input_label}\" does not match any [[artifact]] label"
                    ),
                )
            })?;
        input_artifacts.push(artifact);
    }

    let mut shared: Option<BTreeSet<&str>> = None;
    for artifact in &input_artifacts {
        let labels = applicable_target_labels(artifact, targets);
        match &shared {
            None => shared = Some(labels),
            Some(existing) if *existing == labels => {}
            Some(_) => {
                return Err(RunError::new(
                    ErrorCode::ParseInvalidBundle,
                    format!(
                        "Bundle \"{bundle_label}\" inputs apply to different sets of [[target]] entries; all inputs must share the same targets"
                    ),
                ));
            }
        }
    }

    let shared_labels = shared.unwrap_or_default();
    Ok(targets
        .iter()
        .filter(|target| {
            target
                .label
                .as_deref()
                .is_some_and(|label| shared_labels.contains(label))
        })
        .collect())
}
