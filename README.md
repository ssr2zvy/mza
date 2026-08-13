# making-rust-artifacts/ allows to build artifacts for Rust projects in the workspace

## 1) Define your artifact(s) and target(s) in artifacts

- Definition of fields provided in reference below
- Defining artifact(s) is choosing which Rust projects are in scope to make artifacts for
- Defining target(s) is choosing which OS, Architecture, and toolchains the created artifact(s) will be compatiable with

## 2) Execute build script

- Entrypoint is "making-artifacts/build.sh" for unix based systems and "making-artifacts/build.ps1" for windows
- Requirements before execution of entrypoint include Rust (compiles), Zig (allows linking), cargo-zigbuild (Rust module to use Zig's linking)
- Artifacts will be output to the directory configured via `artifact_output_path` in artifacts.toml

## Description of Fields in artifacts.toml

### Top level fields

| Field | Requirement | Description |
|---|---|---|
| `artifact_output_path` | mandatory | Absolute path, or relative path from making-rust-artifacts/, of the directory produced artifact(s) will be placed in |

### `[[artifact]]` (can define more than one)

| Field | Requirement | Description |
|---|---|---|
| `label` | optional | Non functional label |
| `crate` | mandatory | Absolute path, or relative path from making-rust-artifacts/, to the crate directory the artifact will be built from. The directory must contain a Cargo.toml |
| `type` | mandatory | Classification of artifact type, must be "release", "snapshot", or "custom" |
| `version` | mandatory | Version number |
| `name` | optional | File name of produced artifact, replacing name defined in either "[[bin]]" or "[[package]]" of crate manifest |
| `exclude` | optional | array of "label" value(s) from "[[target]](s)" to exclude the corresponding Target for this given artifact. |

### `[[target]]` (can define more than one)

| Field | Requirement | Description |
|---|---|---|
| `label` | optional (unless you want to exclude this target for any artifact(s) in scope) | Non functional label |
| `os` | mandatory | Target OS for artifact(s) produced. No MacOS unless that is also the OS of the device using "making-artifacts"/ |
| `arch` | mandatory | Target architecture for artifact(s) produced (user chooses) |
| `environment` | optional | Target ABI/runtime/toolchain environment for artifact(s) produced (glibc, gnu, musl, etc.) |
