# Protocol: `cargo-bundler-v0.1.0`

Use this protocol when a bundle's packaging logic is implemented by another Rust crate, and that crate's compiled output should embed its inputs' bytes directly (not just paths to them), since the crate is only ever cross-compiled by MZA and never executed on the build host.

## TOML Parsing

For each `[[target]]` shared by all of a bundle's `inputs`, the resulting bundle is archived at:

```text
<bundle output_path>/<label>/<type>/cargo-bundler-v0.1.0/<version>/<target>/<label>-<version>-<target>.tar.xz
```

- If the bundle's `inputs` don't share the same applicable `[[target]](s)`, this is caught during `artifacts.toml` parsing (before any build runs).
- Any other failure while running this protocol (missing archives, cargo build failure, archiving failure) is reported with error code `BUNDLE_EXECUTION_FAILED` (or `BUNDLE_MISSING_INPUT`/`BUNDLE_UNKNOWN_PROTOCOL` where applicable), and causes the overall run to exit with code `2`.

## Contract

### What info the crate receives

- A `bundle-spec.toml` file, written by MZA per shared target, under `<system-temp>/mza/<run_id>/<bundle-label>/<target>/bundle-spec.toml` (`<system-temp>` is `std::env::temp_dir()`, the OS standard temp directory). This scratch space is shared with `command-bundler-v0.1.0`.
- The spec's contents:

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

### How the crate receives it

- MZA sets the `MZA_BUNDLE_INPUTS` environment variable to the absolute path of that spec file, then runs `cargo zigbuild --release --locked --target <triple> --manifest-path <bundle Cargo.toml>` (or `cargo build` when natively targeting macOS).
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

## Notes

- Why a `build.rs` is required: this crate is only ever **compiled** by MZA for a (possibly foreign) target — MZA never runs it. So its own code cannot read `MZA_BUNDLE_INPUTS` at runtime (`std::env::var`/`std::fs::read`): by the time the compiled binary actually executes, it's running on a different machine entirely, where the build-host paths in the spec don't exist. Anything the crate needs must therefore be captured **during compilation**, as literal embedded bytes — and only `include_bytes!` with a source-level literal path can do that. `build.rs` bridges the two: it runs natively on the build host, as part of the same `cargo zigbuild` invocation, so it can read `MZA_BUNDLE_INPUTS` like ordinary runtime code, then generate the `include_bytes!` calls `main.rs` needs.
- Build/toolchain requirements: the bundle crate is built via `cargo zigbuild` (or `cargo build` only when natively targeting macOS); every invocation uses `--locked`, so the crate needs a committed, up-to-date `Cargo.lock`; the resolved target triple must be installable via `rustup target add`.
- Naming/versioning: the crate's own `Cargo.toml` `[package].name`/`.version` are used to name and archive the bundle's own output, exactly like an artifact, unless the bundle declares an optional `name` override.
- Full source code for this protocol: `src/protocol/cargo_bundler_v0_1_0.rs` (relative to `automation/build_and_bundle/mza/`).
