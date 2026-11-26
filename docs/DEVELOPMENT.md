# Development setup

This guide covers setting up a machine for AURIVO development, including the
platform audio dependencies that CI installs automatically.

## Windows

1. Install Rust stable with the MSVC toolchain (`rustup default stable-x86_64-pc-windows-msvc`).
2. Install the Visual Studio Build Tools with the C++ workload.
3. WebView2 is preinstalled on Windows 11 and current Windows 10 builds.
4. Clone the repository and run `cargo test --workspace`.

If the C: drive is full, set `CARGO_HOME` to a directory on another drive and
invoke the cargo shim explicitly, as shown in `AGENTS.md`.

## Linux

```bash
sudo apt-get install -y libasound2-dev libwebkit2gtk-4.1-dev \
  libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
cargo test --workspace
```

For a headless machine, skip the desktop crate and test the libraries only:

```bash
cargo test -p aurivo-core -p aurivo-dsp -p aurivo-audio-engine
```

## macOS

Install Xcode command line tools, then `cargo test --workspace`. WebKit is part
of the operating system.

## Recommended workflow

1. Create a branch named after the change (see `AGENTS.md`).
2. Make a small change and run `cargo test -p <crate>` immediately.
3. Run the full gate before opening a PR:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
```

On Windows, `scripts/check.ps1` runs all four steps. On Linux and macOS, use
`scripts/check.sh`.

## Debugging audio issues

- Start with `cargo run -p aurivo-desktop` and watch the console for cpal errors.
- Use `RUST_LOG=debug` if a logging plugin is added later.
- Dump state from the UI and compare with `aurivo-cli state diff`.
- For DSP-only problems, write a waveform test in
  `crates/aurivo-dsp/tests/waveforms.rs`; it avoids device dependencies.
- Use release builds for timing measurements; debug builds confuse DSP profiling.

## Working on the frontend

The frontend has no build step. Edit `apps/desktop/dist/index.html`,
`styles.css` or `app.js`, then restart `cargo run -p aurivo-desktop`. Validate
JavaScript syntax with `node --check apps/desktop/dist/app.js`.

Remember that Rust command arguments are camelCase in JavaScript. A mismatch
appears as a rejected promise and is shown by `toast()`.

## Adding Tauri commands

1. Add the `#[tauri::command]` function in `main.rs`.
2. Register it in `generate_handler!`.
3. Call it from `app.js` with camelCase keys.
4. Surface both success and failure through `toast()`.
5. Add a Rust unit test for any non-trivial logic behind the command.

## Common problems

| Symptom | Cause | Fix |
|---------|-------|-----|
| `window.__TAURI__ is undefined` | `withGlobalTauri` disabled | Keep it enabled in `tauri.conf.json` |
| `missing required key` | snake_case argument from JS | Use camelCase keys |
| No audio devices listed | cpal host initialisation failed | Check OS permissions and drivers |
| Tests fail only in release | overflow or denormal difference | Add explicit clamping and denormal flushing |
| Disk full during build | large registry/target dirs | Move `CARGO_HOME` and clear `target/` |
