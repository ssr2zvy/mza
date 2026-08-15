# Protocol: `cargo-bundler-v0.1.0`

Use this protocol when a bundle's packaging logic is implemented by another Rust crate, and that crate should embed its inputs at compile time.

## What making-rust-artifacts does

For each `[[target]]` shared by all of a bundle's `inputs`:

1. Resolves the bundle's `crate` to a `Cargo.toml`.
2. Resolves the `.tar.xz` archive path already produced for each of the bundle's `inputs` for that target (inputs are built as ordinary `[[artifact]](s)` first).
3. Generates `mza_bundle_inputs.rs` (see format below) under `<temp_dir>/making-zig-archive/<run_id>/<bundle-label>/mza_bundle_inputs.rs`, where `<temp_dir>` is `std::env::temp_dir()` (OS standard temp directory).
4. Sets the `MZA_BUNDLE_INPUTS` environment variable to the absolute path of that generated file, then runs `cargo zigbuild --release --target <triple> --manifest-path <bundle Cargo.toml>` (or `cargo build` when natively targeting macOS).
5. Archives the resulting binary the same way an artifact is archived, at `<bundle artifact_output_path>/<bundle label>/<bundle type>/<crate version>/<name>-<version>-<triple>.tar.xz`.

## Contract the bundle crate must implement

- Somewhere in `src/`, include the generated file at compile time:

  ```rust
  include!(env!("MZA_BUNDLE_INPUTS"));
  ```

- This defines, in the including module, a self-contained type and static — no imports or additional dependencies required:

  ```rust
  pub struct MzaBundleInput {
      pub label: &'static str,
      pub archive: &'static str,
  }

  pub static MZA_BUNDLE_INPUTS: &[MzaBundleInput] = &[
      MzaBundleInput { label: "lexicon_cli", archive: "/abs/path/lexicon_cli-0.1.0-x86_64-unknown-linux-musl.tar.xz" },
      MzaBundleInput { label: "lexicon_framework", archive: "/abs/path/lexicon_framework-0.1.0-x86_64-unknown-linux-musl.tar.xz" },
  ];
  ```

- `label` matches the `label` of the corresponding `[[artifact]]` in `artifacts.toml`.
- `archive` is the absolute path to that input's already-produced `.tar.xz` archive (not the raw binary); the bundle crate is responsible for extracting/embedding whatever it needs from that archive.
- The crate's own `Cargo.toml` `[package].name`/`.version` are used to name and archive the bundle's own output, exactly like an artifact.

## Failure behavior

- If the bundle's `inputs` don't share the same applicable `[[target]](s)`, this is caught during `artifacts.toml` parsing (before any build runs).
- Any other failure while running this protocol (missing archives, cargo build failure, archiving failure) is reported with error code `BUNDLE_EXECUTION_FAILED` (or `BUNDLE_MISSING_INPUT`/`BUNDLE_UNKNOWN_PROTOCOL` where applicable), and causes the overall run to exit with code `2`.
