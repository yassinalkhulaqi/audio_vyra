# Architecture

This document explains how AURIVO moves audio and state through the system. It
is written for contributors who need to change more than one crate.

## 1. Process model

AURIVO is a single Tauri process with two relevant execution contexts:

- **UI thread** — WebView JavaScript, Tauri commands, filesystem I/O.
- **Audio callbacks** — cpal input and output threads, plus optional playback.

The engine object itself is held in a `Mutex` on the UI side. Commands lock it
briefly; the audio callbacks never touch that mutex. Instead they capture
`Arc` handles to the chain, meter, analyzer, recorder and atomic parameters.

## 2. Audio path

```text
microphone (interleaved, F32)
  -> downmix_into()             mono scratch buffer
  -> EffectChain::process()     try_lock on the chain
  -> LevelMeterProcessor        atomics
  -> SpectrumAnalyzer::push()   try_lock on published bins
  -> RecorderState::push()      try_lock
  -> HeapRb<f32>                mono samples
  -> output callback            pop mono frames
  -> LinearResampler            only when rates differ
  -> expand_into()              output channel count
  -> output gain/mute           relaxed atomics
  -> speakers (interleaved, F32)
```

Keeping the ring buffer mono is important. Earlier versions expanded to the
input channel count before the ring and consumed using the output channel
count, which produced time stretching whenever the devices disagreed.

### Resampling

`LinearResampler` keeps a fractional read position and unconsumed source
samples between callbacks. It never clamps the position when a source block is
short, so its output is identical whether audio arrives as one block or many.
Anti-alias filtering on extreme downsampling is not implemented; document any
new limitations here.

### Latency

A mono sample counter is incremented by the input callback and decremented by
the output callback. `LatencyTracker` converts the residual count to
milliseconds and publishes it atomically. This measures buffering inside the
app only, not device or OS latency.

## 3. Effect chain

`EffectChain` keeps two parallel vectors: `EffectSlot` metadata (name and
enabled flag) and the `ProcessingPipeline` of boxed processors. Operations that
change order move entries in both vectors together, otherwise bypass state
would attach to the wrong effect.

`AudioProcessor` requires `name`, `process`, `bypass` and `set_bypass`. All
implementations live in `aurivo-dsp/src/pipeline.rs`. Effects that cache their
own sample rate are constructed when the engine starts.

The limiter is the final stage and clamps the master output to 0.98.

## 4. Parameters

Three patterns exist, in decreasing order of preference:

1. `Arc<AtomicU32>` storing `f32::to_bits()` — gain, pitch, output gain.
2. `RtParam<T>` — a UI-owned `Arc<Mutex<T>>` plus an audio-owned cached copy
   refreshed with `try_lock`. Used by delay, reverb, distortion, modulation,
   vocal and limiter-class parameters.
3. `Arc<Mutex<T>>` with `try_lock` at use sites — EQ coefficients and the
   spectrum output buffer.

Every effect accepts untrusted values from JSON. `aurivo_dsp::validate`
provides clamps for dB, ratio, milliseconds, frequency, unit values and modes.
`ProcessingPipeline::process` replaces non-finite samples with silence before
running the chain.

## 5. State and persistence

`AppState` is the single serialisable snapshot of user-controlled state. It
contains all effect parameters, chain order, bypass states and EQ bands.

- `AudioEngine::state()` captures live values.
- `AudioEngine::apply_state()` restores order, bypass, parameters and the
  exact number of EQ bands.
- Presets extend `AppState` with a flat `params` map for backwards
  compatibility, plus description, tags, version and timestamp.
- Config stores the full `AppState` plus device names in the platform config
  directory.

Writes to both config and presets use a temporary file and `fs::rename`.

## 6. Path safety

All user-entered paths are resolved relative to the app config directory by
`apps/desktop/src-tauri/src/paths.rs`. Absolute paths, parent traversal and
invalid components are rejected before any filesystem call. File names are
additionally sanitised.

## 7. Frontend

`apps/desktop/dist` is deliberately build-free:

- `index.html` declares controls with stable IDs.
- `styles.css` holds the dark theme and responsive layout.
- `app.js` defines one `boot()` function wired on `DOMContentLoaded`.

`withGlobalTauri` is enabled, so `window.__TAURI__.core.invoke` is available.
Tauri expects camelCase argument keys for Rust parameters written in
snake_case. Polling intervals drive meters, health counters and recorder state;
the spectrum uses `requestAnimationFrame`.

## 8. Testing strategy

- Unit tests beside DSP modules for pure math.
- Integration tests under `crates/*/tests/` for effect invariants, property
  checks, chain/state behaviour, resampler continuity and WAV round-trips.
- Desktop persistence modules are tested through a small harness that compiles
  `presets.rs`, `config.rs` and `paths.rs` without the Tauri runtime.

## 9. Deliberate non-goals

- No plugin host (CLAP/VST) in this phase.
- No MIDI or automation yet.
- No look-ahead limiter; the current limiter is feed-forward with a ceiling.
