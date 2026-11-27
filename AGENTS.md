# AGENTS.md

Commands and conventions for humans and agents working in this repository.

## Commands

```bash
# Build the library crates
cargo build -p aurivo-core -p aurivo-dsp -p aurivo-audio-engine

# Build everything (requires Tauri system dependencies)
cargo build --workspace

# Run the app
cargo run -p aurivo-desktop

# Tests
cargo test --workspace
cargo test -p aurivo-dsp
cargo test -p aurivo-audio-engine
cargo test -p aurivo-core

# Lint and format
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

On machines where the C: drive is full, set `CARGO_HOME` to a directory on a
drive with free space and call the rustup cargo shim explicitly:

```powershell
$env:CARGO_HOME = 'D:\opencode\cargo-home'
$env:CARGO_INCREMENTAL = '0'
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --workspace
```

## Repository layout

- `crates/aurivo-core` — shared types only.
- `crates/aurivo-dsp` — pure DSP. Do not add I/O or OS dependencies here.
- `crates/aurivo-audio-engine` — cpal streams and the synchronous engine API.
- `apps/desktop/src-tauri` — Tauri commands and persistence modules.
- `apps/desktop/dist` — no bundler; plain HTML/CSS/JS.

## Conventions

- Every DSP effect implements `AudioProcessor`. New effects must provide
  `name`, `process`, `bypass` and `set_bypass`, and be added to the chain in
  `AudioEngine::new`.
- Audio callbacks must not allocate or block. Use `RtParam` or atomics for
  parameters shared with the UI; use `try_lock` for shared state.
- Parameter values from JSON are untrusted. Pass them through
  `aurivo_dsp::validate` helpers before using them in DSP math.
- New state fields belong in `AppState`. Update `state()`, `apply_state()`,
  factory presets and the UI sync path together.
- User-supplied file paths must go through `apps/desktop/src-tauri/src/paths.rs`.
- Persistence writes must be atomic (temporary file + rename).
- Tests live next to the code for unit tests and in `tests/` for integration
  tests. Add a test for every bug fix.

## Before opening a pull request

1. `cargo fmt --all`
2. `cargo clippy --workspace --all-targets -- -D warnings`
3. `cargo test --workspace`
4. `cargo build --workspace`
5. Update `README.md` and `CHANGELOG.md` when behaviour changes.

## Branch naming

`feature/<short-name>` for features, `fix/<short-name>` for bug fixes,
`test/<short-name>` for coverage-only work, `docs/<short-name>` for docs.

Do not push or merge unless explicitly asked.
