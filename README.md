# AURIVO

Real-time voice processing for the desktop. AURIVO is a low-latency effects chain you can drive live from a Tauri application: speak into your microphone, hear the processed signal on your output device, and shape it with dynamics, tone, modulation, pitch, vocal character and recording.

The codebase is a Rust workspace with a pure-DSP core, a `cpal` audio engine, and a thin desktop shell.

---

## Features

| Area | What you get |
|------|----------------|
| Device I/O | Enumerate input/output devices, start/stop a duplex stream, pick by name or use defaults |
| Dynamics | Input gain, noise gate, compressor, output peak limiter |
| Tone | High-pass filter and multiband parametric EQ (peaking, shelves, low-pass, high-pass) |
| Time-based | Feedback delay with damping, Freeverb-style reverb |
| Distortion | tanh soft clip, hard clip, fuzz, bitcrush + downsample |
| Modulation | Chorus, flanger, phaser, vibrato |
| Pitch | Granular pitch shifter (±24 semitones) with linear interpolation |
| Vocal FX | Ring modulator, robotizer, vowel formant bank (A/E/I/O/U morph), tremolo |
| Visualizer | FFT spectrum with log-frequency axis, RMS history, peak and clip indicator |
| Recording | Record the processed mono signal to WAV, play it back with rate/channel adaptation |
| Presets | Versioned full-state JSON presets, bundle import/export, eight factory presets |
| Config | Full session state and device selection persisted in the platform config directory |
| Telemetry | Overruns, underruns, lock-contention skips, buffered latency |

Signal flow:

```text
input -> downmix -> gain -> gate -> compressor -> high-pass -> delay -> reverb
       -> distortion -> modulation -> EQ -> pitch -> vocal FX -> limiter
       -> mono ring buffer -> resample -> output gain/mute -> speakers
```

The meter and spectrum analyzer tap the processed mono signal.

---

## Workspace layout

```text
vyra/
├── Cargo.toml                         # workspace root (resolver 2)
├── README.md
├── AGENTS.md                          # commands for humans and agents
├── CONTRIBUTING.md                    # branch/commit/test expectations
├── CHANGELOG.md
├── docs/
│   ├── ARCHITECTURE.md
│   ├── IMPROVEMENT-PLAN.md
│   └── decisions/                     # architecture decision records
├── crates/
│   ├── aurivo-core/                   # errors, AudioConfig, DeviceInfo, LevelMeter
│   ├── aurivo-dsp/                    # pure DSP, no I/O, no external dependencies
│   │   ├── src/                       # gain, gate, compressor, limiter, filter,
│   │   │                              # pipeline, delay, reverb, distortion,
│   │   │                              # modulation, eq, pitch, vocal, analysis,
│   │   │                              # denormal, validate, rt_param
│   │   └── tests/                     # basic, effects, bypass, rt_safety,
│   │                                  # correctness, property
│   └── aurivo-audio-engine/           # cpal streams, ring buffer, recorder, API
│       └── src/                       # lib, chain, channels, audio_path,
│                                      # resampler, latency, rt_stats,
│                                      # recorder, state
└── apps/desktop/
    ├── dist/                          # index.html + styles.css + app.js
    └── src-tauri/                     # commands, presets, config, paths
```

### Crate responsibilities

- **`aurivo-core`** — shared types and errors. No audio logic.
- **`aurivo-dsp`** — dependency-free processors. Every processor implements `AudioProcessor { name, process, bypass, set_bypass }` and composes through `ProcessingPipeline`. Shared parameters use atomics or `RtParam` caches so the audio callback never blocks.
- **`aurivo-audio-engine`** — owns the host, streams, mono ring buffer, meter, analyzer, recorder, resampler, limiter and the sync API used by Tauri.
- **`aurivo-desktop`** — Tauri 2 binary with thin command wrappers plus preset/config/path persistence.

---

## Requirements

- Rust stable (edition 2021)
- Platform audio via `cpal` (WASAPI / CoreAudio / ALSA)
- Tauri 2 prerequisites for desktop builds
- F32-capable input and output devices (non-F32 defaults are rejected with a `Stream` error)

---

## Build & run

```bash
# Library checks
cargo build -p aurivo-core -p aurivo-dsp -p aurivo-audio-engine

# Desktop app
cargo run -p aurivo-desktop
```

Verification:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
```

The window is configured in `apps/desktop/src-tauri/tauri.conf.json` (1100×720, frontend from `../dist`).

---

## Using the app

1. **Start** — pick input/output (or leave the lists empty for defaults), press **Start** or hit Space.
2. **Gain** — drag *Gain dB* (−24…+24); applied live through a lock-free atomic.
3. **Levels** — the bar tracks RMS, the peak readout and CLIP flag show headroom; R resets the clip flag.
4. **Effects** — each section has live sliders; the sidebar toggles bypass and reorders the chain (buttons or drag and drop).
5. **Output** — set *Out gain*, check *Mute* (or press M) to control monitoring; the limiter catches peaks.
6. **Recorder** — press R or *Record*, save the processed mono signal as WAV, then play it back.
7. **Presets** — save, load, delete, install factory presets, or export/import a JSON bundle.
8. **Session state** — dump/apply the complete `AppState` JSON for debugging or sharing.
9. **Stop** — tears down all streams and stops recording.

---

## Architecture notes

- **Real-time safety** — the audio callback never takes a blocking parameter lock. Effects use `RtParam` snapshots with `try_lock` and a last-known-good cache; the chain and analyzer use `try_lock`; counters expose skipped stages in the UI. Callback buffers are allocated at stream start.
- **Mono DSP** — interleaved input is downmixed explicitly, processed once, stored mono in the ring buffer, then expanded to the output channel count. Resampling happens on the mono stream before expansion.
- **Sample rates** — the audio path reports both device rates and resamples when they differ. Effects that depend on time receive a rate at construction; device-rate reconciliation is documented in `docs/ARCHITECTURE.md`.
- **Persistence** — config and preset writes are atomic (temp file + rename). All user-supplied file paths are validated to stay inside the app config directory.
- **Errors** — engine methods return `aurivo_core::Result`; Tauri commands map errors to strings.

---

## Testing

- DSP: processor behaviour plus silence/finite/extreme-parameter property tests.
- Engine: chain reorder/bypass state, state round-trips, WAV round-trips, resampler continuity, latency and contention counters.
- Desktop: preset migration, validation, bundles, config round-trips, path safety.

Run `cargo test --workspace`.

---

## Roadmap

See `docs/IMPROVEMENT-PLAN.md` and `docs/decisions/`. Optional future work includes MIDI control, per-parameter automation, A/B compare and latency compensation.

---

## License

No license file yet; until one is added, all rights are reserved by the repository owner.
