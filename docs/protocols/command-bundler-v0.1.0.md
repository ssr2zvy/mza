# Protocol: `command-bundler-v0.1.0`

Use this protocol when the final packaging is produced by an external, project-specific system (a native tool, a script, an installer generator, etc.), driven by a small Rust adapter crate that runs on the build host. Unlike `cargo-bundler-v0.1.0`, this protocol never cross-compiles the bundle crate: the crate is always built and run for the build host, and it is responsible for producing (by whatever means) an executable for the *requested* target.

## TOML Parsing

- `build_targets` (required): Array of exact target triples (e.g. `"x86_64-unknown-linux-musl"`) this bundle must be produced for. Every input artifact must provide every listed triple; MZA validates this when parsing `artifacts.toml`, before any build runs. Triples that don't match any `[[target]]`, or that some input artifact excludes, are caught at this parse stage (`PARSE_INVALID_BUNDLE`).
- After `cargo run` exits, MZA requires the spec's `output_path` to exist as a regular file — the only success signal (no exit-code-plus-manifest handshake). It then archives that file the same way an artifact is archived, at:

```text
<bundle output_path>/<label>/<type>/command-bundler-v0.1.0/<version>/<target>/<label>-<version>-<target>.tar.xz
```

- Any other runtime failure (missing `Cargo.lock`, the `cargo run` command failing, or the expected `output_path` not existing afterward) is reported as `BUNDLE_EXECUTION_FAILED`, and causes the overall run to exit with code `2`.

## Contract

### What info the crate receives

- A `bundle-spec.toml` file, written by MZA to a run-scoped temp workspace (shared with Protocol 1): `<system-temp>/mza/<run_id>/<bundle-label>/<target>/bundle-spec.toml`.
- The spec's contents:

  ```toml
  protocol = "command-bundler-v0.1.0"
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

### How the crate receives it

- MZA sets `MZA_BUNDLE_SPEC` to the spec file's absolute path, then runs `cargo run --release --locked --manifest-path <bundle Cargo.toml>` on the build host (no `--target`, no `cargo zigbuild`).
- The adapter crate reads `MZA_BUNDLE_SPEC` with any TOML parser, at its own runtime — this works because, unlike Protocol 1, this crate genuinely executes on the build host during MZA's process.
- The adapter crate must write its final executable to exactly the spec's `output_path` before exiting `0`.

## Notes

- Overall flow:

  ```text
  MZA
    → cargo run <protocol crate> on build host
        → crate invokes its project-specific external bundler
            → external bundler produces the target executable
  ```

- Build/toolchain requirements: the bundle's `crate` is the adapter crate (same field Protocol 1 uses); it must contain one unambiguous binary target (a single `[[bin]]`, or `default-run` set in `Cargo.toml`). This protocol does not use a `build.rs`, `$OUT_DIR/mza_bundle_inputs.rs`, `MZA_BUNDLE_INPUTS`, or `cargo zigbuild` on the bundle crate — those belong to `cargo-bundler-v0.1.0`.
- Naming/versioning: the crate's `Cargo.toml` `[package].name`/`.version` are the authoritative source for the bundle's own name/version, exactly as in Protocol 1.
- Target verification is trust-based: MZA does not inspect the executable's actual architecture/format, only that the file the crate was told to write to exists (same as Protocol 1).
- Full source code for this protocol: `src/protocol/command_bundler_v0_1_0.rs` (relative to `automation/build_and_bundle/mza/`).
