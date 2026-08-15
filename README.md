# mza/ allows to build artifacts and bundles for Rust projects in the workspace

## 1) Define your artifact(s) and target(s) in artifacts

- Definition of fields provided in reference below
- Defining artifact(s) is choosing which Rust projects are in scope to make artifacts for
- Defining target(s) is choosing which OS, Architecture, and toolchains the created artifact(s) will be compatiable with

## 2) Execute build script

- Entrypoint is "making-artifacts/make-artifact.sh" for unix based systems and "making-artifacts/make-artifact.ps1" for windows
- Requirements before execution of entrypoint include Rust (compiles), Zig (allows linking), cargo-zigbuild (Rust module to use Zig's linking)
- Every cargo invocation uses `--locked`, so every crate built or run by this tool (artifacts, and bundle crates for either protocol) must have an up-to-date, committed `Cargo.lock`.
- Each artifact is packaged as `<artifact_output_path>/<label>/<type>/<crate-version>/<name>-<crate-version>-<target-triple>.tar.xz`, using the version declared in the crate's `Cargo.toml`.

## Bundling

- A `[[bundle]]` packages the archives of one or more `[[artifact]](s)` into a final bundle output, rather than shipping the artifacts on their own.
- Every bundle is packaged as `<artifact_output_path>/<label>/<type>/<protocol>/<crate-version>/<target-triple>/<label>-<crate-version>-<target-triple>.tar.xz` — the protocol id occupies the position after `type`, and the target triple gets its own directory level in addition to the filename.
- `protocol` selects which bundling contract the bundle's crate implements. Each protocol has its own doc under `docs/protocols/`:
  - `cargo-bundler-v0.1.0` ([docs/protocols/cargo-bundler-v0.1.0.md](docs/protocols/cargo-bundler-v0.1.0.md)) — use this when the bundling itself will be done by another Rust crate, cross-compiled and archived the same way as an artifact. Targets are derived from the set shared by all `inputs` (accounting for each artifact's `exclude`); this is validated when `artifacts.toml` is parsed.
  - `command-bundle-v1` ([docs/protocols/command-bundle-v1.md](docs/protocols/command-bundle-v1.md)) — use this when the final packaging is produced by an external, project-specific system driven by a small Rust adapter crate that runs on the build host (never cross-compiled). Requires `build_targets`.
- Bundle failures during `--build` exit with code `2` (distinct from artifact build failures, which exit with code `1`) so it's clear whether the failure happened before or during bundling.

## Description of Fields in artifacts.toml

### `[[artifact]]` (can define more than one)

| Field | Requirement | Description |
|---|---|---|
| `label` | required | Used to name the artifact directory within `artifact_output_path` |
| `crate` | mandatory | Absolute path, or relative path from mza/, to the crate directory the artifact will be built from. The directory must contain a Cargo.toml |
| `artifact_output_path` | mandatory | Absolute path, or relative path from mza/, of the directory that will contain all produced artifacts |
| `type` | mandatory | Classification of artifact type, must be "main", "snapshot", or "custom" |
| `name` | optional | File name of produced artifact, replacing name defined in either "[[bin]]" or "[[package]]" of crate manifest |
| `exclude` | optional | array of "label" value(s) from "[[target]](s)" to exclude the corresponding Target for this given artifact. |

### `[[target]]` (can define more than one)

| Field | Requirement | Description |
|---|---|---|
| `label` | optional (unless you want to exclude this target for any artifact(s) in scope) | Non functional label |
| `os` | mandatory | Target OS for artifact(s) produced. No MacOS unless that is also the OS of the device using "making-artifacts"/ |
| `arch` | mandatory | Target architecture for artifact(s) produced (user chooses) |
| `environment` | optional | Target ABI/runtime/toolchain environment for artifact(s) produced (glibc, gnu, musl, etc.) |

### `[[bundle]]` (can define more than one)

| Field | Requirement | Description |
|---|---|---|
| `label` | required | Used to name the bundle directory within `artifact_output_path` |
| `crate` | mandatory | Absolute path, or relative path from mza/, to the crate directory the bundle is associated with. The directory must contain a Cargo.toml |
| `artifact_output_path` | mandatory | Absolute path, or relative path from mza/, of the directory that will contain produced bundles |
| `type` | mandatory | Classification of bundle type, must be "main", "snapshot", or "custom" |
| `protocol` | mandatory | Identifies which bundling implementation (defined outside this codebase) is responsible for producing this bundle |
| `inputs` | mandatory | array of `label` value(s) from `[[artifact]](s)` that make up the contract of artifacts this bundle consumes |
| `build_targets` | required by some protocols (e.g. `command-bundle-v1`); optional otherwise | array of exact target triples (e.g. `"x86_64-unknown-linux-musl"`) this bundle must be produced for. Every input artifact must provide every listed triple. When omitted, targets are instead derived from the set shared by all `inputs`. |
