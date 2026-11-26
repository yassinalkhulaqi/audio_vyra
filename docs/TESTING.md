# Testing AURIVO

The test suite is split by the kind of guarantee it provides.

## Layers

| Layer | Location | What it proves |
|-------|----------|----------------|
| DSP unit | `crates/aurivo-dsp/src/**` | Individual math helpers (validate, denormal, limiter, automation, control) |
| DSP integration | `crates/aurivo-dsp/tests/basic.rs`, `effects.rs` | Each processor produces the expected shape of output |
| DSP waveform | `crates/aurivo-dsp/tests/waveforms.rs` | Frequency response, delay timing, reverb tail, pitch direction |
| DSP property | `crates/aurivo-dsp/tests/property.rs`, `rt_safety.rs`, `correctness.rs`, `bypass.rs` | Silence stays silent, finite output under hostile input/params, bypass transparency, non-blocking parameter snapshots |
| Engine integration | `crates/aurivo-audio-engine/tests/*.rs` | Chain/state round-trips, WAV files, resampler continuity, latency counters, automation, control mapping |
| Desktop persistence | `apps/desktop/src-tauri/src/{config,presets,paths}.rs` `#[cfg(test)]` modules | Migration, validation, atomic writes, path safety |

The desktop persistence modules are also compiled by a small harness that
includes them as plain modules, so they can be tested without building the full
Tauri runtime on machines with limited disk space.

## Commands

```bash
cargo test --workspace
cargo test -p aurivo-dsp -- --test-threads=1
cargo test -p aurivo-audio-engine waveforms
```

Run DSP tests single-threaded when debugging shared-handle behaviour to make
failures deterministic.

## Invariants worth protecting

1. **Silence** — a silent input buffer must produce a silent output buffer for
   the whole chain.
2. **Finite output** — non-finite input or parameters must not leak into
   feedback buffers. `ProcessingPipeline::process` sanitises samples.
3. **Bypass** — setting every processor to bypass must be bit-exact
   passthrough.
4. **State round-trip** — `state()` → `apply_state()` → `state()` must preserve
   chain order, bypass states, EQ band count and all parameter values.
5. **Non-blocking audio** — parameter locks use `try_lock`; contention is
   counted rather than waited on.
6. **Atomic persistence** — a failed write must leave the previous file intact
   and no `.tmp` file behind.

## Adding a test

- Put pure DSP tests beside the module with `#[cfg(test)]`.
- Put cross-module behaviour in `crates/aurivo-dsp/tests/`.
- Put engine/state tests in `crates/aurivo-audio-engine/tests/`.
- Add a state round-trip assertion whenever `AppState` gains a field.
- Use deterministic noise (`Rng` in `property.rs`) instead of `rand`.
- Keep audio buffers short: most tests run in milliseconds; the longest
  waveform tests use one second of 48 kHz audio.

## Known limitations

- No dummy cpal host is used; stream start/stop is not exercised in CI.
- FFT magnitude normalisation is internal and not calibrated to dBFS.
- The limiter is feed-forward without look-ahead; tests assert the ceiling,
  not perceptual transparency.
