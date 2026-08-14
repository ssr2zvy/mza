mod archive;
mod build;
mod parser;

use std::fs;
use std::path::Path;

use parser::ParsedConfig;

fn main() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let artifacts_toml_path = manifest_dir.join("artifacts.toml");

    let contents = fs::read_to_string(&artifacts_toml_path)
        .unwrap_or_else(|err| panic!("Failed to read {}: {err}", artifacts_toml_path.display()));

    let config = parser::parse(&contents)
        .unwrap_or_else(|err| panic!("Failed to parse {}: {err}", artifacts_toml_path.display()));

    echo_parsed(&config);

    // Chokepoint: building only runs when explicitly requested, so this
    // binary can be used to inspect parsing without side effects.
    if !std::env::args().any(|arg| arg == "--build") {
        return;
    }

    let artifacts_dir = artifacts_toml_path
        .parent()
        .expect("artifacts.toml must have a parent directory");

    for artifact in &config.artifacts {
        for target in &config.targets {
            if build::is_excluded(artifact, target) {
                continue;
            }

            build::build_artifact(artifact, target, artifacts_dir)
                .unwrap_or_else(|err| panic!("{err}"));
        }
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

