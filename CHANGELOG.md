# Changelog

All notable changes to this project are documented here. The format follows
Keep a Changelog and the project adheres to Semantic Versioning from v0.1.0.

## [Unreleased]

### Fixed

- Tauri UI no longer fails at startup because the global API is enabled and
  initialization waits for `DOMContentLoaded`.
- Command argument keys use the camelCase names Tauri expects.
- `get_state`/`set_state` now serialise and restore chain order, bypass states
  and the exact EQ band list.
- `stop()` tears down playback and recording in addition to the duplex streams.
- Reordering effects no longer desynchronises bypass flags.
- `AudioProcessor::bypass()` reports the real state for every effect.
- Resampling happens on the mono stream before channel expansion.
- WAV playback adapts the file's sample rate and channel count.
- Denormal flushing no longer discards audible low-level audio.
- Compressor and gate time constants and ratios are sanitised.
- Formant filters glide between coefficients instead of resetting each block.
- Pitch shifting uses linear interpolation between grain samples.
- User-supplied file paths are validated against the app config directory.
- Preset and config writes are atomic (temporary file + rename).

### Added

- `RtParam` lock-free parameter snapshots and `try_lock` everywhere in the
  audio path, with contention counters surfaced in the UI.
- Peak limiter as the final chain stage.
- Latency telemetry (buffered milliseconds) and peak level readout.
- Drag-to-reorder effect chain and keyboard shortcuts (Space, R, M).
- Toast notifications, focus styles and reduced-motion support.
- Versioned preset bundles with import/export, validation and eight factory
  presets.
- Property-style DSP tests, WAV round-trip tests and state-matrix tests.
- Architecture decision records and contributor documentation.

### Changed

- Frontend split into `index.html`, `styles.css` and `app.js`.
- Effect chain contains twelve stages including the limiter.
- README rewritten to describe the current system.

## [0.1.0] - initial release

- Duplex audio engine with eleven effect stages.
- Tauri 2 desktop shell with vanilla JS frontend.
- JSON presets and session state.
