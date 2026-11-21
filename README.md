# AURIVO

Real-time voice processing for the desktop — a low-latency effects chain you can drive live from a Tauri app. Speak into your mic, hear the processed signal on your output device, and tweak gain, dynamics, tone, and vocal character while you talk.

AURIVO is a Rust workspace: a pure-DSP core, a cpal-based audio engine, and a thin Tauri shell with a single-page UI.

---

## Features (on `main`)

| Area | What you get |
|------|----------------|
| **Device I/O** | Enumerate input/output devices, start/stop a duplex stream, pick devices by name or fall back to defaults |
| **Signal chain** | Input gain → noise gate → compressor → 80 Hz high-pass → vocal FX, all sample-accurate in the input callback |
| **Level metering** | Peak and RMS polled from the UI (~10 Hz) |
| **Vocal FX** | Ring modulator, robotizer, vowel formant bank (A/E/I/O/U morph), tremolo — continuous mix, carrier, depth |
| **Latency path** | Input callback → DSP → lock-free ring buffer → output callback (F32 only; non-F32 devices are rejected with a clear error) |

### In flight on feature branches

These are polished and ready to merge; each adds DSP + engine wiring + Tauri commands + UI:

| Branch | Adds |
|--------|------|
| `feature/delay-echo` | Feedback delay with damping, wet/dry, time, feedback |
| `feature/reverb` | Freeverb-style reverb (room size, damping, wet/dry, width) |
| `feature/distortion` | tanh / hard clip / fuzz / bitcrush + downsample & bit depth |
| `feature/modulation` | Chorus, flanger, phaser, vibrato |
| `feature/parametric-eq` | Multiband peaking EQ with full band editor UI |
| `feature/pitch-shift` | Granular pitch shifter (semitones, grain size, mix) |
| `feature/visualizer-output` | FFT spectrum analyzer, output gain, mute |
| `feature/recording` | WAV record/playback of the processed signal |
| `feature/presets-config` | JSON presets: save/load/list/delete, full param apply |

Merge them in any order that fits your milestone plan; each branch is based on current `main`.

---

## Workspace layout

```
vyra/
├── Cargo.toml                     # workspace root (resolver 2)
├── crates/
│   ├── aurivo-core/               # errors, AudioConfig, DeviceInfo, LevelMeter
│   ├── aurivo-dsp/                # pure DSP (no I/O): gain, gate, compressor,
│   │   │                          #   biquad, meter, vocal, pipeline
│   │   └── tests/basic.rs         # unit tests for gain/meter/pipeline
│   └── aurivo-audio-engine/       # cpal duplex stream, shared params,
│                                  #   ring buffer, AudioEngine API
└── apps/
    └── desktop/
        ├── dist/index.html        # UI (vanilla JS + Tauri invoke)
        └── src-tauri/             # Tauri commands, presets (on feature branch)
```

### Crate responsibilities

- **`aurivo-core`** — Shared types: `AurivoError` / `Result`, `AudioConfig`, `DeviceInfo`, `LevelMeter`. No audio logic.
- **`aurivo-dsp`** — Zero-dependency DSP. Every processor implements `AudioProcessor { process, bypass, set_bypass }` and is composable via `ProcessingPipeline`. Shared real-time parameters use `Arc<AtomicU32>` or `Arc<Mutex<…>>` so the UI can update without blocking the audio thread (`try_lock` in the callback).
- **`aurivo-audio-engine`** — Owns the host, streams, pipeline, meter, and (on branches) recorder. `AudioEngine::start` builds matched input/output streams, pumps processed audio through a `ringbuf` heap ring, and exposes a small sync API for Tauri.
- **`aurivo-desktop`** — Tauri 2 binary. Thin `#[tauri::command]` wrappers over `AudioEngine` (+ `presets` module on the presets branch). Frontend is a single `index.html` served from `dist/`.

---

## Default signal chain

Built in `AudioEngine::new` (48 kHz construction-time defaults; actual rate follows the input device on `start`):

```
mic → Gain (shared dB) → NoiseGate (-40 dB, 5 ms / 50 ms)
    → Compressor (-18 dB, 3:1, 10 / 100 ms, +2 dB makeup)
    → Biquad high-pass 80 Hz → Vocal FX → ring buffer → speakers
         ↘ LevelMeter (UI poll)
```

Feature branches insert their processors into this pipeline at construction and expose matching Tauri commands + sliders.

---

## Requirements

- **Rust** stable (edition 2021) with Cargo workspace support
- **Platform audio** via [cpal](https://crates.io/crates/cpal) (WASAPI / CoreAudio / ALSA as appropriate for your OS)
- **Tauri 2** prerequisites for desktop builds ([Tauri prerequisites](https://v2.tauri.app/start/prerequisites/))
- An F32-capable input and output device (AURIVO rejects non-F32 default configs with `Stream` error)

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
2. **Gain** — drag *Gain dB* (−24…+24); applied live to the shared gain atomics.
3. **Levels** — green bar tracks RMS; numeric readout updates continuously.
4. **Vocal FX** — choose mode (Ring mod / Robot / Vowel formant / Tremolo), then shape carrier, vowel morph, depth, and wet mix. Changes invoke `set_vocal` immediately.
5. **Stop** — tears down both streams.

On feature branches, additional sections appear (Recorder, Presets, EQ editor, spectrum canvas, etc.) with the same pattern: slider → `invoke` → engine.

---

## Architecture notes

- **Real-time safety** — DSP runs in the cpal input callback. Parameter updates from the UI never block on a full `Mutex` for the pipeline: the callback uses `try_lock` and skips a block if contended; gain uses relaxed atomics.
- **Duplex sync** — A heap ring buffer (`HeapRb<f32>`, ~4 × device period) decouples input production from output consumption; short reads are zero-filled.
- **Shared vocal state** — `Vocal::shared` returns the processor plus an `Arc<Mutex<VocalParams>>` handle; `Vocal::update` / `Vocal::snapshot` are the only UI-facing mutators.
- **Errors** — Engine methods return `aurivo_core::Result`; Tauri commands map to `Result<(), String>` for the frontend.
- **UI** — No bundler: plain HTML/JS in `dist/index.html`, `window.__TAURI__.core.invoke`, periodic `setInterval` for metering/presets.

---

## Testing

DSP unit tests live in `crates/aurivo-dsp/tests/basic.rs` (gain, meter, pipeline). Run them with:

```bash
cargo test -p aurivo-dsp
```

---

## Roadmap

1. **Merge feature branches** into `main` as milestones land (delay, reverb, distortion, modulation, EQ, pitch, visualizer, recording, presets).
2. **Unify UI** — one cohesive control surface as branches combine (shared layout, consistent param formatting).
3. **Latency & device robustness** — optional buffer-size negotiation, channel mapping, resampling if in/out rates diverge.
4. **Preset packs** — ship factory JSON presets alongside user config.

---

## License

Add a `LICENSE` file when you are ready to publish; until then all rights reserved by the repository owner.
