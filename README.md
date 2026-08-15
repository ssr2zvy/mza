# mza automates target-specific Rust artifact creation, packaging, and bundling

## 1) Define your artifact(s), target(s), and bundle(s) in artifacts.toml

- The types of table elements are `[[artifacts]]`, `[[targets]]`, and `[[bundles]]`. 
	- Through [[artifacts]], defining artifact(s) is choosing which Rust projects are in scope to make artifacts for
	- Through [[targets]], defining target(s) is choosing which OS, Architecture, and toolchains the created artifact(s) will be compatiable with
	- Through [[bundles]], defining bundle(s) is choosing if you want to bundle artifacts into a final executable, and several bundling protocols are availabe in mza. Refer to implementation details for bundling protocols in docs/protocol/
- Each type of table element can be added multiple times. Refer to guides for artifact.toml fields in docs/artifact_toml/fields.md

## 2) Execute make-artifact script

- Entrypoint is `make-artifact.sh` for Unix-based systems and `make-artifact.ps1` for Windows
- Requirements before execution of entrypoint include Rust (compiles), Zig (allows linking), cargo-zigbuild (Rust module to use Zig's linking)
- Artifact versions come from the artifact crate's `Cargo.toml`. Bundle versions likewise come from the `Cargo.toml` of the protocol-implementing bundle crate; neither version is duplicated in `artifacts.toml`.
- Bundle-stage failures during --build exit with code 2, distinct from artifact-stage failures, which exit with code 1

