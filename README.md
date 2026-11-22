# AURIVO

Real-time voice processing for the desktop — a low-latency effects chain you can drive live from a Tauri app. Speak into your mic, hear the processed signal on your output device, and shape it with dynamics, tone, modulation, pitch, vocal character, and recording.

AURIVO is a Rust workspace: a pure-DSP core, a cpal-based audio engine, and a thin Tauri shell with a single-page UI.

---

## Features

| Area | What you get |
|------|----------------|
| **Device I/O** | Enumerate input/output devices, start/stop a duplex stream, pick devices by name or fall back to defaults |
| **Dynamics** | Input gain, noise gate, compressor |
| **Tone** | High-pass filter, multiband parametric EQ (peaking / low shelf / high shelf / low-pass / high-pass) |
| **Time-based** | Feedback delay with damping, Freeverb-style reverb |
| **Distortion** | tanh soft clip, hard clip, fuzz, bitcrush + downsample |
| **Modulation** | Chorus, flanger, phaser, vibrato |
| **Pitch** | Granular pitch shifter (±24 semitones) |
| **Vocal FX** | Ring modulator, robotizer, vowel formant bank (A/E/I/O/U morph), tremolo |
| **Visualizer** | FFT spectrum display, output gain, mute |
| **Recording** | Record the processed signal to WAV, play it back |
| **Presets** | Save/load/list/delete JSON presets (input gain + vocal settings) |
| **Metering** | Peak and RMS polled from the UI (~10 Hz) |

Signal flow: `input gain → gate → compressor → high-pass → delay → reverb → distortion → modulation → EQ → pitch → vocal FX → ring buffer → output gain/mute → speakers`, with the meter and spectrum analyzer tapping the processed signal.

---

## Workspace layout

```
vyra/
├── Cargo.toml                     # workspace root (resolver 2)
├── README.md
├── docs/
│   └── IMPROVEMENT-PLAN.md        # roadmap for future work
├── crates/
│   ├── aurivo-core/               # errors, AudioConfig, DeviceInfo, LevelMeter
│   ├── aurivo-dsp/                # pure DSP, no I/O
│   │   ├── src/                   # gain, meter, gate, compressor, filter,
│   │   │                          #   pipeline, vocal, delay, reverb,
│   │   │                          #   distortion, modulation, eq, pitch, analysis
│   │   └── tests/basic.rs         # unit tests for gain/meter/pipeline
│   └── aurivo-audio-engine/       # cpal streams, ring buffer, recorder, API
└── apps/
    └── desktop/
        ├── dist/index.html        # UI (vanilla JS + Tauri invoke)
        └── src-tauri/             # Tauri commands, JSON presets
```

### Crate responsibilities

- **`aurivo-core`** — Shared types: `AurivoError` / `Result`, `AudioConfig`, `DeviceInfo`, `LevelMeter`. No audio logic.
- **`aurivo-dsp`** — Zero-dependency DSP. Every processor implements `AudioProcessor { process, bypass, set_bypass }` and is composable via `ProcessingPipeline`. Shared real-time parameters use `Arc<AtomicU32>` or `Arc<Mutex<…>>` so the UI can update without blocking the audio thread (`try_lock` in the callback).
- **`aurivo-audio-engine`** — Owns the host, streams, pipeline, meter, spectrum analyzer, recorder, and output routing. `AudioEngine::start` builds matched input/output streams, pumps processed audio through a `ringbuf` heap ring, and exposes a sync API for Tauri.
- **`aurivo-desktop`** — Tauri 2 binary with thin `#[tauri::command]` wrappers over `AudioEngine` plus a `presets` module. Frontend is a single `index.html` served from `dist/`.

---

## Requirements

- **Rust** stable (edition 2021) with Cargo workspace support
- **Platform audio** via [cpal](https://crates.io/crates/cpal) (WASAPI / CoreAudio / ALSA as appropriate for your OS)
- **Tauri 2** prerequisites for desktop builds ([Tauri prerequisites](https://v2.tauri.app/start/prerequisites/))
- An F32-capable input and output device (AURIVO rejects non-F32 default configs with a `Stream` error)

---

## Build & run

From the repository root:

```bash
# Library / DSP checks
cargo build -p aurivo-dsp -p aurivo-audio-engine

# Desktop app (dev)
cargo run -p aurivo-desktop
```

The window is configured in `apps/desktop/src-tauri/tauri.conf.json` (1100×720, frontend from `../dist`).

---

## Using the app

1. **Start** — pick input/output (or leave defaults), press **Start**. Status becomes *Running*.
2. **Gain** — drag *Gain dB* (−24…+24); applied live through shared atomics.
3. **Levels / spectrum** — the green bar tracks RMS, the canvas draws the live FFT, both updating continuously.
4. **Effects** — each section (delay, reverb, distortion, modulation, EQ, pitch, vocal FX) has live sliders; changes invoke the matching Tauri command immediately.
5. **Output** — set *Out gain* or check *Mute* to control monitoring.
6. **Recorder** — record the processed signal, then save it as WAV or play it back.
7. **Presets** — name a preset and save input gain + vocal settings; load, apply, or delete from the list.
8. **Stop** — tears down the audio streams.

---

## Architecture notes

- **Real-time safety** — DSP runs in the cpal input callback. Parameter updates never block on a full `Mutex` for the pipeline: the callback uses `try_lock` and skips a block if contended; gain and output level use relaxed atomics.
- **Duplex sync** — A heap ring buffer (`HeapRb<f32>`, ~4 × device period) decouples input production from output consumption; short reads are zero-filled.
- **Shared parameters** — Effects expose a `::shared()` constructor returning the processor plus a thread-safe handle (`Arc<AtomicU32>` or `Arc<Mutex<Params>>`); `update` / `snapshot` are the only UI-facing mutators.
- **Errors** — Engine methods return `aurivo_core::Result`; Tauri commands map to `Result<_, String>` for the frontend.
- **UI** — No bundler: plain HTML/JS in `dist/index.html`, `window.__TAURI__.core.invoke`, periodic `setInterval` for metering/presets and `requestAnimationFrame` for the spectrum.

---

## Testing

DSP unit tests live in `crates/aurivo-dsp/tests/basic.rs` (gain, meter, pipeline). Run them with:

```bash
cargo test -p aurivo-dsp
```

---

## Roadmap

All nine original feature branches are merged into `main`. See [`docs/IMPROVEMENT-PLAN.md`](docs/IMPROVEMENT-PLAN.md) for the next wave: effect-slot registry, full-state presets, audio correctness (resampling, stereo, clip/overrun detection), UI redesign, DSP tests, CI, and packaging.

---

## License

Add a `LICENSE` file when you are ready to publish; until then all rights reserved by the repository owner.
