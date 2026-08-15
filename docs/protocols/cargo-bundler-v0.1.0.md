# Protocol: `cargo-bundler-v0.1.0`

Use this protocol when a bundle's packaging logic is implemented by another Rust crate, and that crate's compiled output should embed its inputs' bytes directly (not just paths to them), since the crate is only ever cross-compiled by MZA and never executed on the build host.

## What mza does

For each `[[target]]` shared by all of a bundle's `inputs`:

1. Resolves the bundle's `crate` to a `Cargo.toml`.
2. Resolves the `.tar.xz` archive path already produced for each of the bundle's `inputs` for that target (inputs are built as ordinary `[[artifact]](s)` first).
3. Writes `bundle-spec.toml` (see format below) under `<system-temp>/mza/<run_id>/<bundle-label>/<target>/bundle-spec.toml`, where `<system-temp>` is `std::env::temp_dir()` (OS standard temp directory). This scratch space is shared with `command-bundle-v1`.
4. Sets the `MZA_BUNDLE_INPUTS` environment variable to the absolute path of that spec file, then runs `cargo zigbuild --release --locked --target <triple> --manifest-path <bundle Cargo.toml>` (or `cargo build` when natively targeting macOS). This requires an up-to-date, committed `Cargo.lock` for the bundle crate.
5. Archives the resulting binary the same way an artifact is archived, at `<bundle artifact_output_path>/<label>/<type>/cargo-bundler-v0.1.0/<version>/<target>/<label>-<version>-<target>.tar.xz`.

## Why a `build.rs`, not a direct `include!`

This crate is only ever **compiled** by MZA for a (possibly foreign) target — MZA never runs it. So its own code cannot read `MZA_BUNDLE_INPUTS` at runtime (`std::env::var`/`std::fs::read`): by the time the compiled binary actually executes, it's running on a different machine entirely, where the build-host paths in the spec don't exist. Anything the crate needs must therefore be captured **during compilation**, as literal embedded bytes — and only `include_bytes!` with a source-level literal path can do that. A `build.rs` is required to bridge the two: it runs natively on the build host, as part of this same `cargo zigbuild` invocation, so it can read `MZA_BUNDLE_INPUTS` like ordinary runtime code, then generate the `include_bytes!` calls `main.rs` needs.

## `bundle-spec.toml` format

```toml
protocol = "cargo-bundler-v0.1.0"
bundle = "lexicon"
target = "x86_64-unknown-linux-musl"

[[inputs]]
label = "lexicon_cli"
archive = "/absolute/path/lexicon_cli-0.1.0-x86_64-unknown-linux-musl.tar.xz"

[[inputs]]
label = "lexicon_framework"
archive = "/absolute/path/lexicon_framework-0.1.0-x86_64-unknown-linux-musl.tar.xz"
```

## Contract the bundle crate must implement

- A `build.rs` that:
  1. Reads `MZA_BUNDLE_INPUTS` (falling back to no inputs if unset, so the crate still compiles standalone outside of MZA).
  2. Parses that TOML and, for each input, copies its `archive` file into `$OUT_DIR`.
  3. Generates `$OUT_DIR/mza_bundle_inputs.rs`, containing a self-contained type/static using `include_bytes!(concat!(env!("OUT_DIR"), "/<file-name>"))` for each input — real embedded bytes, not paths.

- Somewhere in `src/`, include that generated file at compile time:

  ```rust
  include!(concat!(env!("OUT_DIR"), "/mza_bundle_inputs.rs"));
  ```

- This defines, in the including module, a self-contained type and static:

  ```rust
  pub struct MzaBundleInput {
      pub label: &'static str,
      pub archive: &'static [u8],
  }

  pub static MZA_BUNDLE_INPUTS: &[MzaBundleInput] = &[
      MzaBundleInput { label: "lexicon_cli", archive: include_bytes!(concat!(env!("OUT_DIR"), "/lexicon_cli-0.1.0-x86_64-unknown-linux-musl.tar.xz")) },
      MzaBundleInput { label: "lexicon_framework", archive: include_bytes!(concat!(env!("OUT_DIR"), "/lexicon_framework-0.1.0-x86_64-unknown-linux-musl.tar.xz")) },
  ];
  ```

- `label` matches the `label` of the corresponding `[[artifact]]` in `artifacts.toml`.
- `archive` is the input's `.tar.xz` **bytes**, embedded directly in the compiled binary — never a path, since a build-host path is meaningless once this binary runs elsewhere.
- The crate's own `Cargo.toml` `[package].name`/`.version` are used to name and archive the bundle's own output, exactly like an artifact.

## Failure behavior

- If the bundle's `inputs` don't share the same applicable `[[target]](s)`, this is caught during `artifacts.toml` parsing (before any build runs).
- Any other failure while running this protocol (missing archives, cargo build failure, archiving failure) is reported with error code `BUNDLE_EXECUTION_FAILED` (or `BUNDLE_MISSING_INPUT`/`BUNDLE_UNKNOWN_PROTOCOL` where applicable), and causes the overall run to exit with code `2`.
