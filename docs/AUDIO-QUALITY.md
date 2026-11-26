# Audio quality notes

This document records the deliberate choices made in the audio path and what
they mean for perceived quality. It is a companion to `ARCHITECTURE.md`, which
describes the structure.

## Latency budget

| Stage | Typical contribution |
|-------|----------------------|
| Device input buffer | OS dependent, often 5–20 ms |
| Downmix + chain | < 1 ms for the default twelve effects |
| Ring buffer | 0–20 ms depending on buffer occupancy (shown in the UI) |
| Resampler | < 1 ms |
| Device output buffer | OS dependent, often 5–20 ms |

The UI's **Latency** readout shows only buffering inside AURIVO. The **Peak**
and clip flag show headroom before the limiter and output gain.

The ring capacity is four seconds at the input rate. That prevents dropouts on
busy systems but means the steady-state buffered latency can grow if the output
callback cannot keep up; watch **overruns/underruns** and the buffered value.

## Signal path decisions

### Mono processing

The chain handles one mono stream. Stereo input is averaged with
`downmix_into`, which preserves level for correlated signals and reduces level
for wide stereo. This is intentional: voice effects behave predictably on one
stream and latency stays low. Per-channel processing is a future feature.

### Sample rate reconciliation

Devices that do not run at 48 kHz are resampled with a linear interpolator on
the mono stream. Linear interpolation is transparent for modest ratios
(44.1 ↔ 48 kHz) but does not include an anti-alias filter for extreme
downsampling. Prefer device rates near 48 kHz when possible.

### Parameter smoothing

Input gain uses `SmoothedGain`, a one-pole ramp with a 5 ms coefficient. Fast
slider movements therefore produce a short glide instead of zipper noise. The
effect's own `process` methods use `RtParam` snapshots; a parameter may be one
block late under UI contention, but no clicks are introduced by locking.

### Limiter

The final chain stage is a feed-forward peak limiter with a -3 dB threshold,
2 ms attack, 120 ms release and a 0.98 ceiling. It catches resonant peaks from
EQ and distortion but is not a look-ahead brick-wall limiter. If you need
strict broadcast ceilings, add an external limiter after AURIVO.

### Denormals

Feedback state is flushed below `1e-37`, safely inside the subnormal range of
f32. This keeps feedback loops fast without discarding audible low-level
signals.

## Known artefacts

- **Pitch shifting** uses overlapping grains with Hann windows and linear
  interpolation. Large semitone shifts can sound granular; this is expected.
- **Formant mode** glides biquad coefficients with an 8% per-block step. Very
  fast vowel sweeps therefore trail slightly.
- **Vibrato** forces wet mix to 1.0 because the effect is implemented as a
  modulated delay rather than a dry/wet blend.
- **Reverb** uses a fixed allpass coefficient and `wet * tail * 2.0` gain; the
  limiter is the final safeguard against clipping.
- **Bitcrush** quantises after drive drive and before the tone filter. Very low
  bit depths are intentionally harsh.

## Measuring a problem

1. Note input/output rates, channels and buffered latency from the UI.
2. Check clip and peak; reduce input gain or enable the limiter if they are hot.
3. Check overrun/underrun counters; increase device buffer size in the OS if they grow.
4. Check `skips` counters; a high FX count means the UI held a lock too long.
5. Dump state and inspect it with `aurivo-cli state check`.
6. For preset problems, use `aurivo-cli preset validate`.

## Guidelines for new effects

- Never allocate in `process`.
- Never hold a `Mutex` guard across a whole block; poll with `try_lock`.
- Clamp all parameters before division or exponentiation.
- Flush denormals in feedback state.
- Provide `reset()` so state can be cleared when the stream restarts.
- Add a waveform test that checks the effect's intended shape, not just finiteness.
