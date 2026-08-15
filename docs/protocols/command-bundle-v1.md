# Protocol: `command-bundle-v1`

Use this protocol when the final packaging is produced by an external, project-specific system (a native tool, a script, an installer generator, etc.), driven by a small Rust adapter crate that runs on the build host.

Unlike `cargo-bundler-v0.1.0`, this protocol never cross-compiles the bundle crate: the crate is always built and run for the build host, and it is responsible for producing (by whatever means) an executable for the *requested* target.

```
MZA
  → cargo run <protocol crate> on build host
      → crate invokes its project-specific external bundler
          → external bundler produces the target executable
```

## `[[bundle]]` fields specific to this protocol

| Field | Requirement | Description |
|---|---|---|
| `build_targets` | required | Array of exact target triples (e.g. `"x86_64-unknown-linux-musl"`) this bundle must be produced for. Every input artifact must provide every listed triple; MZA validates this when parsing `artifacts.toml`, before any build runs. |

The bundle's `crate` is the adapter crate (same field Protocol 1 uses); its `Cargo.toml` `[package].name`/`.version` are the authoritative source for the bundle's own name/version, exactly as in Protocol 1. The crate must contain one unambiguous binary target (a single `[[bin]]`, or `default-run` set in `Cargo.toml`).

## What mza does, per `build_targets` entry

1. Resolves the bundle crate's `Cargo.toml` and reads its package name/version.
2. Resolves each input's already-produced `.tar.xz` archive for the current target.
3. Writes `bundle-spec.toml` to a run-scoped temp workspace (shared with Protocol 1): `<system-temp>/mza/<run_id>/<bundle-label>/<target>/bundle-spec.toml`.
4. Sets `MZA_BUNDLE_SPEC` to that file's absolute path.
5. Runs `cargo run --release --locked --manifest-path <bundle Cargo.toml>` on the build host (no `--target`, no `cargo zigbuild`).
6. After the command exits, verifies the file at the spec's `output_path` exists, then archives it the same way an artifact is archived, at `<bundle output_path>/<label>/<type>/command-bundle-v1/<version>/<target>/<label>-<version>-<target>.tar.xz`.

## `bundle-spec.toml` contract

```toml
protocol = "command-bundle-v1"
bundle = "lexicon"
output_path = "/tmp/mza/<run_id>/lexicon/x86_64-unknown-linux-musl/output/lexicon"

[bundle_target]
target = "x86_64-unknown-linux-musl"

[[bundle_target.inputs]]
label = "lexicon-cli"
archive = "/absolute/path/lexicon-cli-0.1.0-x86_64-unknown-linux-musl.tar.xz"
```

- `bundle_target.target` is the target this whole bundle execution is producing; each `bundle_target.inputs` entry only needs `label`/`archive` since the target is already established once, at the `[bundle_target]` level.
- `output_path` is the exact absolute file path the adapter crate must write its final executable to. MZA decides this path; the crate does not choose or report it back — there is no separate result manifest.
- `MZA_BUNDLE_SPEC` is set to the absolute path of this file; the adapter crate reads it with any TOML parser.
- After `cargo run` exits `0`, MZA requires `output_path` to exist as a regular file. This is the only success signal: there is no exit-code-plus-manifest handshake beyond "the process succeeded and the file is there."

## Failure behavior

- `build_targets` triples that don't match any `[[target]]`, or that some input artifact excludes, are caught during `artifacts.toml` parsing (`PARSE_INVALID_BUNDLE`).
- Any other failure (missing `Cargo.lock`, the `cargo run` command failing, or the expected `output_path` not existing afterward) is reported as `BUNDLE_EXECUTION_FAILED`, and causes the overall run to exit with code `2`.
- Like Protocol 1, target verification is trust-based: MZA does not inspect the executable's actual architecture/format, only that the file the crate was told to write to exists.

## What this protocol does not use

- No `build.rs`, no `$OUT_DIR/mza_bundle_inputs.rs`, no `MZA_BUNDLE_INPUTS`, no `cargo zigbuild` on the bundle crate — those belong to `cargo-bundler-v0.1.0`.
