mod archive;
mod build;
mod error;
mod parser;
mod protocol;
mod run_record;
mod shared;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use parser::ParsedConfig;
use run_record::{ArtifactOutcome, BundleOutcome, RunOutcome, RunRecord};

fn main() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let artifacts_toml_path = manifest_dir.join("artifacts.toml");
    let artifacts_dir = artifacts_toml_path
        .parent()
        .expect("artifacts.toml must have a parent directory");

    let args: Vec<String> = std::env::args().collect();
    let record = RunRecord::start(artifacts_dir, args.clone())
        .unwrap_or_else(|err| panic!("Failed to start run record: {err}"));

    let contents = fs::read_to_string(&artifacts_toml_path)
        .unwrap_or_else(|err| panic!("Failed to read {}: {err}", artifacts_toml_path.display()));
    record
        .record_input(&contents)
        .unwrap_or_else(|err| panic!("Failed to record run input: {err}"));

    let config = match parser::parse(&contents) {
        Ok(config) => config,
        Err(err) => {
            let outcome = RunOutcome {
                dry_run: !args.iter().any(|arg| arg == "--build"),
                artifacts: vec![ArtifactOutcome {
                    label: None,
                    target: None,
                    status: "error".to_string(),
                    error_code: Some(err.code.as_str().to_string()),
                    message: Some(err.message.clone()),
                }],
                bundles: Vec::new(),
            };
            record
                .record_outcome(&outcome)
                .unwrap_or_else(|record_err| panic!("Failed to record run outcome: {record_err}"));
            panic!("Failed to parse {}: {err}", artifacts_toml_path.display());
        }
    };

    echo_parsed(&config);

    // Chokepoint: building only runs when explicitly requested, so this
    // binary can be used to inspect parsing without side effects.
    if !args.iter().any(|arg| arg == "--build") {
        let bundle_outcomes: Vec<BundleOutcome> = config
            .bundles
            .iter()
            .map(|bundle| BundleOutcome {
                label: bundle.label.clone(),
                status: "skipped".to_string(),
                error_code: None,
                message: Some("Bundle execution is not yet implemented".to_string()),
            })
            .collect();
        let outcome = RunOutcome {
            dry_run: true,
            artifacts: Vec::new(),
            bundles: bundle_outcomes,
        };
        record
            .record_outcome(&outcome)
            .unwrap_or_else(|err| panic!("Failed to record run outcome: {err}"));
        return;
    }

    let mut artifact_outcomes = Vec::new();
    let mut archive_paths: HashMap<(String, String), PathBuf> = HashMap::new();
    let mut had_artifact_failure = false;

    for artifact in &config.artifacts {
        for target in &config.targets {
            if build::is_excluded(artifact, target) {
                continue;
            }

            match build::build_artifact(artifact, target, artifacts_dir) {
                Ok(archive_path) => {
                    if let (Some(label), Some(target_label)) = (artifact.label.clone(), target.label.clone()) {
                        archive_paths.insert((label, target_label), archive_path);
                    }
                    artifact_outcomes.push(ArtifactOutcome {
                        label: artifact.label.clone(),
                        target: target.label.clone(),
                        status: "ok".to_string(),
                        error_code: None,
                        message: None,
                    })
                }
                Err(err) => {
                    had_artifact_failure = true;
                    artifact_outcomes.push(ArtifactOutcome {
                        label: artifact.label.clone(),
                        target: target.label.clone(),
                        status: "error".to_string(),
                        error_code: Some(err.code.as_str().to_string()),
                        message: Some(err.message.clone()),
                    });
                }
            }
        }
    }

    let mut bundle_outcomes = Vec::new();
    let mut had_bundle_failure = false;

    for bundle in &config.bundles {
        if bundle.protocol == "command-bundle-v1" && bundle.build_targets.is_none() {
            had_bundle_failure = true;
            bundle_outcomes.push(BundleOutcome {
                label: bundle.label.clone(),
                status: "error".to_string(),
                error_code: Some(error::ErrorCode::ParseInvalidBundle.as_str().to_string()),
                message: Some(format!(
                    "Bundle \"{}\" uses protocol command-bundle-v1, which requires build_targets",
                    bundle.label.as_deref().unwrap_or("<unlabeled>")
                )),
            });
            continue;
        }

        let bundle_targets = match parser::resolve_bundle_targets(bundle, &config.artifacts, &config.targets) {
            Ok(targets) => targets,
            Err(err) => {
                had_bundle_failure = true;
                bundle_outcomes.push(BundleOutcome {
                    label: bundle.label.clone(),
                    status: "error".to_string(),
                    error_code: Some(err.code.as_str().to_string()),
                    message: Some(err.message.clone()),
                });
                continue;
            }
        };

        for target in bundle_targets {
            match protocol::run_bundle(bundle, target, artifacts_dir, record.run_id(), &archive_paths) {
                Ok(_archive_path) => bundle_outcomes.push(BundleOutcome {
                    label: bundle.label.clone(),
                    status: "ok".to_string(),
                    error_code: None,
                    message: None,
                }),
                Err(err) => {
                    had_bundle_failure = true;
                    bundle_outcomes.push(BundleOutcome {
                        label: bundle.label.clone(),
                        status: "error".to_string(),
                        error_code: Some(err.code.as_str().to_string()),
                        message: Some(err.message.clone()),
                    });
                }
            }
        }
    }

    let outcome = RunOutcome {
        dry_run: false,
        artifacts: artifact_outcomes,
        bundles: bundle_outcomes,
    };
    record
        .record_outcome(&outcome)
        .unwrap_or_else(|err| panic!("Failed to record run outcome: {err}"));

    // Distinct exit codes make it clear whether artifacts (1) or only the
    // bundling stage (2) is responsible for the failure.
    if had_artifact_failure {
        std::process::exit(1);
    }
    if had_bundle_failure {
        std::process::exit(2);
    }
}

fn echo_parsed(config: &ParsedConfig) {
    println!("Parsed {} artifact(s):", config.artifacts.len());
    for artifact in &config.artifacts {
        println!(
            "  - label={:?} crate={} type={} output={} name={:?} exclude={:?}",
            artifact.label,
            artifact.crate_path,
            artifact.r#type.as_str(),
            artifact.artifact_output_path,
            artifact.name,
            artifact.exclude
        );
    }

    println!("Parsed {} target(s):", config.targets.len());
    for target in &config.targets {
        println!(
            "  - label={:?} os={} arch={} environment={:?}",
            target.label, target.os, target.arch, target.environment
        );
    }

    println!("Parsed {} bundle(s):", config.bundles.len());
    for bundle in &config.bundles {
        println!(
            "  - label={:?} crate={} type={} protocol={} inputs={:?}",
            bundle.label,
            bundle.crate_path,
            bundle.r#type.as_str(),
            bundle.protocol,
            bundle.inputs
        );
    }
}

