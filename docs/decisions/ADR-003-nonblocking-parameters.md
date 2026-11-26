# ADR-003: Non-blocking parameter snapshots instead of blocking locks

- Status: Accepted
- Date: 2026-11-26

## Context

Effects previously called `Mutex::lock()` in the audio callback while the UI
updated the same mutex. Under contention the audio thread could block for an
unbounded time, causing glitches.

## Decision

Introduce `RtParam<T>`: the UI owns an `Arc<Mutex<T>>`; the audio thread owns a
cached copy refreshed with `try_lock`. Chain and analyzer locks are also
non-blocking. Skipped stages are counted and shown in the UI.

## Consequences

- Parameter changes can be applied one audio block late; this is inaudible.
- Audio latency stays bounded under UI contention.
- Telemetry makes regressions visible instead of silently dropping work.
