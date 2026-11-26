# Releasing AURIVO

This document describes the release process from a clean checkout to published
installers.

## Prerequisites

- Write access to the repository and permission to create releases.
- Platform toolchains: WebView2 on Windows, WebKit2GTK on Linux, Xcode command line tools on macOS.
- A version tag following semantic versioning, for example `v0.1.0`.

## Versioning

AURIVO follows Semantic Versioning:

- **patch** — bug fixes that do not change behaviour intentionally,
- **minor** — new features or new state fields with backwards-compatible presets,
- **major** — incompatible preset format or audio-path changes.

Update these files before tagging:

1. `apps/desktop/src-tauri/tauri.conf.json` → `version`
2. `crates/*/Cargo.toml` (only when publishing crates)
3. `CHANGELOG.md` → move entries from *Unreleased* into the new section
4. `Cargo.lock` → refresh with `cargo check --workspace`

## Local verification

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

Smoke test the built app:

- start the engine with default devices,
- confirm levels and spectrum move while speaking,
- save and reload a preset,
- record, save and play back a WAV,
- stop and confirm recording stops.

## Icons

Installer icons are generated from a deterministic source script:

```bash
node scripts/generate-icons.mjs
```

Replace the generated images with real artwork before a public release. The
script is only a placeholder generator so the bundle configuration is valid in
CI.

## Tagging

```bash
git tag -a v0.1.0 -m "AURIVO 0.1.0"
git push origin v0.1.0
```

The `Release` workflow builds Windows, macOS and Linux bundles, uploads them as
workflow artifacts, and creates a draft GitHub release with generated notes.

## Bundle outputs

| Platform | Output |
|----------|--------|
| Windows | MSI and NSIS installers under `target/release/bundle` |
| macOS | `.app` and `.dmg` |
| Linux | `.deb` and AppImage |

## After publishing

- Confirm every artifact can be downloaded and installed.
- Verify the app starts on a machine with no development tools installed.
- Announce breaking preset changes and provide a migration path.
- Start a new *Unreleased* section in `CHANGELOG.md`.
