# Roadmap

This file records the work that remains after the current workstreams. Items
are grouped by value and risk so they can be picked up independently.

## High value, low risk

1. **Desktop crate in CI** — the workflow now checks and tests it on
   `windows-latest`; add a macOS and Linux build matrix with WebKit dependencies.
2. **Formatter gate** — `cargo fmt --all -- --check` is now blocking; run the
   formatter across the workspace once to make the whole tree compliant.
3. **Factory preset artwork** — replace the generated placeholder icons with
   designed artwork before a public release.
4. **Config migration** — detect older flat configs (only `gain_db` and device
   names) and wrap them in a versioned envelope.
5. **Preset collision policy** — ask before overwriting a preset whose
   sanitised file name matches an existing one.

## High value, medium risk

6. **Device-rate chain rebuild** — processors currently cache the sample rate
   from construction. Add `from_shared` constructors so the chain can be
   rebuilt at the real device rate when the stream starts.
7. **Per-channel DSP** — process input channels independently instead of
   downmixing, with a UI choice between mono and stereo operation.
8. **Anti-alias filtering** — add a short FIR decimation stage before extreme
   downsampling in `LinearResampler`.
9. **Look-ahead limiter** — add a small delay line and peak detector for a
   true brick-wall ceiling.
10. **Real MIDI input** — feed `ControlMap::apply` from `midir` messages with a
    Tauri event bridge.

## Features

11. **A/B compare** — hold two `AppState` snapshots and crossfade between them.
12. **Recording latency compensation** — align the recorded file with the
    monitored signal using the buffered-latency estimate.
13. **Effect slot registry** — allow adding arbitrary effects at runtime with
    a factory keyed by name; requires stable shared handles for every effect.
14. **Automation curves** — expose LFO phase and shape per route in presets.
15. **Spectrum peak-hold settings** — configurable decay and log-frequency
    resolution.

## Quality

16. **Benchmarks** — add `criterion` benches for the DSP chain and measure
    per-effect cost in a release build.
17. **Fuzz tests** — feed random JSON into `apply_preset` and `apply_state`
    to verify no panics or non-finite parameters.
18. **Snapshot tests** — store golden output buffers for each effect at a
    fixed sample rate and compare with a tolerance.
19. **Accessibility audit** — verify labels, focus order and screen reader
    announcements in the WebView.
20. **Localisation** — extract UI strings into a resource file.

## Explicitly out of scope for now

- CLAP/VST plugin hosting
- Mobile targets
- Cloud preset sync
- Speech-to-text or transcription
