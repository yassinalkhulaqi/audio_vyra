# Contributing to AURIVO

Thanks for helping improve AURIVO. This document describes the workflow that
keeps changes small, reviewable and safe for a real-time audio application.

## Ground rules

1. The DSP crate stays pure. No filesystem, networking, OS, or audio-device
   dependencies belong in `aurivo-dsp`.
2. Audio callbacks never allocate or block. Parameters cross the UI/audio
   boundary through atomics or `RtParam` snapshots.
3. Every user-visible behaviour change gets a test and a changelog line.
4. Do not commit secrets, generated files, recordings, or local config.

## Branches

Create a branch from the current `main`:

```bash
git switch main
git pull --ff-only
git switch -c feature/<short-name>
```

Use the prefixes defined in `AGENTS.md`:

- `feature/` — new capability
- `fix/` — bug fix
- `test/` — coverage-only work
- `docs/` — documentation only

Keep each branch focused on one concern. A branch that changes DSP behaviour
and the UI at the same time is hard to review and hard to revert.

## Commits

Write imperative, specific messages:

```text
fix(dsp): clamp compressor ratio to avoid division by zero
feat(presets): add versioned bundle export
test(engine): cover WAV round-trip at 44.1 kHz
```

Do not mix formatting-only changes with behaviour changes unless the formatter
is required to touch the code you changed anyway.

## Before you open a PR

Run the full local gate:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

If the Tauri build cannot run on your machine (missing WebKit2GTK or WebView2
prerequisites), say so explicitly in the PR description and report which of
the four commands passed.

## Adding an effect

1. Create a module in `crates/aurivo-dsp/src/`.
2. Store parameters behind an `Arc<AtomicU32>` or `RtParam<T>`.
3. Implement `AudioProcessor` in `pipeline.rs` with a stable `name()`.
4. Add the processor to the chain in `AudioEngine::new`.
5. Add a shared handle field on `AudioEngine` and forwarding getters/setters.
6. Add Tauri commands only if the UI needs direct control.
7. Extend `AppState`, `state()`, `apply_state()` and the factory presets.
8. Add unit tests for the processor and a state round-trip test.

## Adding a UI control

The frontend is plain HTML/CSS/JS in `apps/desktop/dist`:

- `index.html` — markup only
- `styles.css` — theme and layout
- `app.js` — all behaviour, wrapped in `boot()`

Use `invoke()` for every backend call and never assume a command succeeded:
show errors through `toast()`. If you add a command argument with an underscore
in Rust, remember Tauri expects camelCase in JavaScript.

## Reporting bugs

Include the OS, device sample rates and channel counts from the **stream info**
line, the values from **Audio health**, and whether the issue persists after a
fresh start. Audio bugs are much easier to reproduce with those numbers.

## Review checklist for reviewers

- Does any audio-callback path allocate or block?
- Are new parameters validated and round-tripped by `AppState`?
- Are user paths validated by `paths.rs`?
- Are persistence writes atomic?
- Are tests covering the changed behaviour?
