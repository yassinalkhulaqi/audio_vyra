# ADR-002: Mono ring buffer with explicit channel handling

- Status: Accepted
- Date: 2026-11-26

## Context

Input and output devices may disagree on channel count. The original design
expanded mono audio to the input channel count before writing to the ring and
consumed using the output channel count, which changed playback speed when the
counts differed.

## Decision

The DSP chain processes one mono stream. The ring stores mono samples.
Resampling happens on the mono stream, and output expansion uses the actual
output device channel count.

## Consequences

- Channel mismatches no longer change timing.
- Spatial information from stereo input is intentionally discarded (documented in the UI and README).
- Per-channel processing would require a larger refactor and is deferred.
