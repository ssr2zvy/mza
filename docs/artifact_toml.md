# Guide for Fields in artifacts.toml

## `[[artifact]]`

| Field | Requirement | Description | Mock syntax |
|---|---|---|---|
| `label` | required | Used to identify artifact, as well as to classify file paths | `label = "example_cli"` |
| `crate` | required | Absolute path, or relative path from mza/, to the directory of the crate from which the artifact will be built. The directory must contain a Cargo.lock | `crate = "../../example-cli"` |
| `output_path` | required | Absolute path, or relative path from mza/, of the directory that will contain produced artifacts; artifact output continues below it through `<label>/<type>/<crate-version>/` | `output_path = "../../artifacts/"` |
| `type` | required | Defined as either "main", "snapshot", or "custom", as well as to classify file paths | `type = "main"` |
| `name` | optional | Resolves the artifact file-name stem, replacing the name defined in either "[[bin]]" or "[[package]]" in the 'Cargo.toml' of the crate from which the artifact will be built; the resolved name begins the final `<name>-<crate-version>-<target-triple>.tar.xz` filename | `name = "example"` |
| `exclude` | optional | Array of `label` values from `[[target]]` entries to exclude for this artifact | `exclude = ["windows-x86_64-gnu"]` |

## `[[target]]`

| Field | Requirement | Description | Mock syntax |
|---|---|---|---|
| `label` | optional (unless you want to exclude this target for any artifact(s) in scope) | Identifies the target for artifact exclusions and target-specific bundle validation | `label = "linux-x86_64-gnu"` |
| `os` | required | Selects the target operating system used to compute the target triple written into produced artifact file names. No MacOS unless that is also the OS of the device using mza | `os = "linux"` |
| `arch` | required | Selects the target architecture used to compute the target triple written into produced artifact file names | `arch = "x86_64"` |
| `environment` | optional | Selects the target ABI/runtime/toolchain environment used to compute the target triple written into produced artifact file names (glibc, gnu, musl, etc.) | `environment = "musl"` |

## `[[bundle]]`

| Field | Requirement | Description | Mock syntax |
|---|---|---|---|
| `label` | required | Used to identify bundle, as well as to classify file paths | `label = "example_bundle"` |
| `crate` | required | Absolute path, or relative path from mza/, to the directory of the crate that implements the bundling protocol. The directory must contain a Cargo.lock | `crate = "../../example-bundle"` |
| `output_path` | required | Absolute path, or relative path from mza/, of the directory that will contain produced bundles; bundle output continues below it through `<label>/<type>/<protocol>/<crate-version>/<target-triple>/` | `output_path = "../../artifacts/"` |
| `type` | required | Defined as either "main", "snapshot", or "custom", as well as to classify file paths | `type = "custom"` |
| `protocol` | required | Identifies the bundling contract user must implement. See [cargo-bundler-v0.1.0](docs/protocols/cargo-bundler-v0.1.0.md) when another Rust crate is cross-compiled into the final bundle executable, or [command-bundle-v1](docs/protocols/command-bundle-v1.md) when a host-run Rust adapter invokes an external bundling system. Also used to clarify file paths | `protocol = "cargo-bundler-v0.1.0"` |
| `name` | optional | Resolves the bundle file-name stem, replacing the name defined in either "[[bin]]" or "[[package]]" in the `Cargo.toml` of the crate that implements the bundling protocol; the resolved name begins the final `<name>-<crate-version>-<target-triple>.tar.xz` filename | `name = "example_installer"` |
| `inputs` | required | Array of artifact `label` values that identifies the archives to combine into this bundle; every input must provide the target selected for the bundle, and missing inputs are rejected rather than silently skipped | `inputs = ["example_cli", "example_framework"]` |
| `build_targets` | required (for `command-bundle-v1`); optional (for other protocols) | Array of exact target triples (e.g. `"x86_64-unknown-linux-musl"`) this bundle must be produced for. Each selected triple supplies the bundle target path component and must be provided by every input artifact; when omitted, targets are derived from the exact set shared by all `inputs` after exclusions are applied | `build_targets = ["x86_64-unknown-linux-musl", "aarch64-unknown-linux-musl"]` |