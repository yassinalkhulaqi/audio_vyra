# ADR-001: Keep the DSP crate dependency-free

- Status: Accepted
- Date: 2026-11-26

## Context

The audio effects are mathematically small but performance critical. Pulling in
a DSP framework would simplify some filters but would also add transitive
dependencies, longer compile times and potential allocation in callbacks.

## Decision

`aurivo-dsp` depends only on `std`. FFT, biquads, envelopes and granular
buffers are implemented locally.

## Consequences

- Positive: predictable build, no hidden allocation, easy to reason about real-time safety.
- Negative: more code to maintain; correctness must be proven by tests.
- Revisit if a well-audited, allocation-free crate covers a substantial subset.
