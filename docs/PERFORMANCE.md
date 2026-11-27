# Performance notes

AURIVO targets a stable 48 kHz stereo duplex stream with a callback budget well
below 10 ms. This document records where time is spent and how to investigate
regressions.

## Callback budget

The input callback performs, in order:

1. downmix (one pass over the input frames),
2. the twelve-stage effect chain,
3. meter update,
4. FFT accumulation (once per 1024 samples),
5. recorder copy (only while recording),
6. ring buffer write.

The output callback pops mono samples, optionally resamples, expands to the
output channel count, and applies gain/mute. No callback allocates after stream
start.

## What is expensive

| Operation | Relative cost | Notes |
|-----------|---------------|-------|
| Reverb | high | 8 combs + 4 allpasses per sample |
| Parametric EQ | high | five biquads per sample |
| Granular pitch | medium | window math and two grain reads |
| FFT | medium | 1024-point FFT once per 1024 samples |
| Gain, gate, compressor | low | a few multiplies and one `powf` per block |
| Limiter | low | envelope follower and clamp |

## Instrumentation

Use the UI's **Audio health** panel:

- overruns/underruns — ring buffer pressure,
- `skips fx` — chain lock contention,
- `skips fft` — spectrum output lock contention,
- `skips rec` — recorder lock contention (only while recording),
- `skips rate` — resampler lock contention (only when rates differ).

`aurivo-cli state check` and `aurivo-cli effects` help reproduce a configuration
without the GUI.

## Rules for contributors

- Preallocate buffers in `start()` and capture them in callbacks.
- Do not call `format!`, `clone()` or `collect()` in `process`.
- Poll shared parameters with `try_lock` through `RtParam`.
- Keep denormal flushing in feedback paths.
- Profile with a release build; debug builds are 10–30× slower in DSP loops.

## Regression checklist

1. Run `cargo test --workspace --release`.
2. Start with default devices and 48 kHz.
3. Enable every effect and watch peak/CPU for five minutes.
4. Record for one minute and verify no dropped samples.
5. Toggle every bypass while recording; there should be no clicks.
6. Change sample rate via the OS and verify the UI reports resampling.
