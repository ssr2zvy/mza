use std::collections::BTreeSet;

use serde::Deserialize;

use super::artifact::{Artifact, ArtifactType};
use super::target::Target;
use crate::build;
use crate::error::{ErrorCode, RunError};

#[derive(Debug, Deserialize)]
pub struct Bundle {
    pub label: Option<String>,
    #[serde(rename = "crate")]
    pub crate_path: String,
    pub output_path: String,
    pub r#type: ArtifactType,
    pub protocol: String,
    pub inputs: Vec<String>,
    pub name: Option<String>,
    /// Explicit target triples this bundle must be produced for. When set,
    /// every input artifact must provide every listed triple (protocols like
    /// command-bundle-v1 require this); when absent, the targets shared by
    /// all inputs are derived automatically (as cargo-bundler-v0.1.0 does).
    #[serde(default)]
    pub build_targets: Option<Vec<String>>,
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

/// Resolves the [[target]] entries a bundle must build for. When
/// `build_targets` is set, each listed triple is matched against a
/// [[target]] and every input must provide it. Otherwise, the [[target]]
/// entries shared by every input are derived automatically. Both are
/// config-level contracts, so mismatches are reported as parse errors.
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
        input_artifacts.push((input_label.as_str(), artifact));
    }

    match &bundle.build_targets {
        Some(build_targets) => resolve_explicit_targets(bundle_label, &input_artifacts, targets, build_targets),
        None => resolve_shared_targets(bundle_label, &input_artifacts, targets),
    }
}

fn resolve_explicit_targets<'a>(
    bundle_label: &str,
    input_artifacts: &[(&str, &Artifact)],
    targets: &'a [Target],
    build_targets: &[String],
) -> Result<Vec<&'a Target>, RunError> {
    let mut resolved = Vec::with_capacity(build_targets.len());

    for requested_triple in build_targets {
        let target = targets
            .iter()
            .find(|target| build::triple(target).map(|triple| triple == *requested_triple).unwrap_or(false))
            .ok_or_else(|| {
                RunError::new(
                    ErrorCode::ParseInvalidBundle,
                    format!(
                        "Bundle \"{bundle_label}\" build_targets entry \"{requested_triple}\" does not match any [[target]]"
                    ),
                )
            })?;
        let target_label = target.label.as_deref().ok_or_else(|| {
            RunError::new(
                ErrorCode::ParseInvalidBundle,
                format!("Bundle \"{bundle_label}\" build_targets entry \"{requested_triple}\" matches a [[target]] without a label"),
            )
        })?;

        for (input_label, artifact) in input_artifacts {
            if artifact.exclude.iter().any(|excluded| excluded == target_label) {
                return Err(RunError::new(
                    ErrorCode::ParseInvalidBundle,
                    format!(
                        "Bundle \"{bundle_label}\" cannot be built for \"{requested_triple}\": input artifact \"{input_label}\" does not provide that target"
                    ),
                ));
            }
        }

        resolved.push(target);
    }

    Ok(resolved)
}

fn resolve_shared_targets<'a>(
    bundle_label: &str,
    input_artifacts: &[(&str, &Artifact)],
    targets: &'a [Target],
) -> Result<Vec<&'a Target>, RunError> {
    let mut shared: Option<BTreeSet<&str>> = None;
    for (_, artifact) in input_artifacts {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(label: &str, exclude: Vec<&str>) -> Artifact {
        Artifact {
            label: Some(label.to_string()),
            crate_path: format!("../{label}"),
            output_path: "../out".to_string(),
            r#type: ArtifactType::Custom,
            name: None,
            exclude: exclude.into_iter().map(str::to_string).collect(),
        }
    }

    fn target(label: &str, os: &str, arch: &str, environment: Option<&str>) -> Target {
        Target {
            label: Some(label.to_string()),
            os: os.to_string(),
            arch: arch.to_string(),
            environment: environment.map(str::to_string),
        }
    }

    fn bundle(inputs: Vec<&str>, build_targets: Option<Vec<&str>>) -> Bundle {
        Bundle {
            label: Some("lexicon".to_string()),
            crate_path: "../..".to_string(),
            output_path: "../out".to_string(),
            r#type: ArtifactType::Custom,
            protocol: "cargo-bundler-v0.1.0".to_string(),
            inputs: inputs.into_iter().map(str::to_string).collect(),
            name: None,
            build_targets: build_targets.map(|list| list.into_iter().map(str::to_string).collect()),
        }
    }

    #[test]
    fn parses_valid_bundle() {
        let value: toml::Value = toml::from_str(
            "label = \"lexicon\"\ncrate = \"../..\"\noutput_path = \"../out\"\ntype = \"custom\"\nprotocol = \"cargo-bundler-v0.1.0\"\ninputs = [\"cli\"]",
        )
        .unwrap();
        let bundles = parse(vec![value]).unwrap();
        assert_eq!(bundles.len(), 1);
        assert_eq!(bundles[0].build_targets, None);
    }

    #[test]
    fn parses_build_targets_when_present() {
        let value: toml::Value = toml::from_str(
            "label = \"lexicon\"\ncrate = \"../..\"\noutput_path = \"../out\"\ntype = \"custom\"\nprotocol = \"command-bundle-v1\"\ninputs = [\"cli\"]\nbuild_targets = [\"x86_64-unknown-linux-musl\"]",
        )
        .unwrap();
        let bundles = parse(vec![value]).unwrap();
        assert_eq!(bundles[0].build_targets, Some(vec!["x86_64-unknown-linux-musl".to_string()]));
    }

    #[test]
    fn resolves_shared_targets_when_inputs_agree() {
        let artifacts = vec![artifact("cli", vec![]), artifact("framework", vec![])];
        let targets = vec![target("linux-musl", "linux", "x86_64", Some("musl"))];
        let b = bundle(vec!["cli", "framework"], None);

        let resolved = resolve_bundle_targets(&b, &artifacts, &targets).unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].label.as_deref(), Some("linux-musl"));
    }

    #[test]
    fn rejects_inputs_with_different_target_sets() {
        let artifacts = vec![artifact("cli", vec!["linux-musl"]), artifact("framework", vec![])];
        let targets = vec![target("linux-musl", "linux", "x86_64", Some("musl"))];
        let b = bundle(vec!["cli", "framework"], None);

        let err = resolve_bundle_targets(&b, &artifacts, &targets).unwrap_err();
        assert_eq!(err.code.as_str(), "PARSE_INVALID_BUNDLE");
    }

    #[test]
    fn rejects_unknown_input_label() {
        let artifacts = vec![artifact("cli", vec![])];
        let targets = vec![target("linux-musl", "linux", "x86_64", Some("musl"))];
        let b = bundle(vec!["missing"], None);

        let err = resolve_bundle_targets(&b, &artifacts, &targets).unwrap_err();
        assert_eq!(err.code.as_str(), "PARSE_INVALID_BUNDLE");
    }

    #[test]
    fn resolves_explicit_build_targets_matching_triple() {
        let artifacts = vec![artifact("cli", vec![])];
        let targets = vec![target("linux-musl", "linux", "x86_64", Some("musl"))];
        let b = bundle(vec!["cli"], Some(vec!["x86_64-unknown-linux-musl"]));

        let resolved = resolve_bundle_targets(&b, &artifacts, &targets).unwrap();
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].label.as_deref(), Some("linux-musl"));
    }

    #[test]
    fn rejects_build_targets_entry_with_no_matching_target() {
        let artifacts = vec![artifact("cli", vec![])];
        let targets = vec![target("linux-musl", "linux", "x86_64", Some("musl"))];
        let b = bundle(vec!["cli"], Some(vec!["aarch64-unknown-linux-musl"]));

        let err = resolve_bundle_targets(&b, &artifacts, &targets).unwrap_err();
        assert_eq!(err.code.as_str(), "PARSE_INVALID_BUNDLE");
    }

    #[test]
    fn rejects_build_targets_entry_excluded_by_input() {
        let artifacts = vec![artifact("cli", vec!["linux-musl"])];
        let targets = vec![target("linux-musl", "linux", "x86_64", Some("musl"))];
        let b = bundle(vec!["cli"], Some(vec!["x86_64-unknown-linux-musl"]));

        let err = resolve_bundle_targets(&b, &artifacts, &targets).unwrap_err();
        assert_eq!(err.code.as_str(), "PARSE_INVALID_BUNDLE");
    }
}

