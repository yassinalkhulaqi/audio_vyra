# Glossary

Terms used throughout the codebase and UI.

## Audio

- **Attack** — time constant that controls how quickly an envelope follower
  responds to an increase in level.
- **Biquad** — second-order infinite impulse response filter used for the
  high-pass stage, EQ bands and formant filters.
- **Bypass** — pass audio through an effect unchanged while keeping its
  parameter values.
- **Clip** — a sample reaching or exceeding full scale (1.0), shown by the CLIP flag.
- **Damping** — attenuation of high frequencies in a delay feedback path.
- **Denormal** — a floating point value smaller than the smallest normal
  number; can slow down DSP loops and is flushed to zero in feedback paths.
- **Downmix** — combine multiple channels into one mono stream.
- **Dry / wet** — the unprocessed and processed portions of a signal.
- **Envelope follower** — circuit (in software) that tracks signal level over
  time, used by the gate, compressor and limiter.
- **Expand** — duplicate a mono stream into multiple interleaved channels.
- **FFT** — Fast Fourier Transform; converts time-domain audio into frequency bins.
- **Formant** — resonant frequency band that characterises a vowel sound.
- **Granular** — synthesis that overlaps short grains of audio read at a
  different rate to shift pitch.
- **Interleaved** — sample layout where channels alternate (L, R, L, R…).
- **LFO** — Low Frequency Oscillator; used for modulation and automation.
- **Makeup gain** — gain applied after compression to restore level.
- **Overrun** — producer wrote more samples than the ring buffer could hold.
- **Peak** — maximum absolute sample value in a block.
- **Release** — time constant that controls how quickly an envelope returns
  after a decrease in level.
- **RMS** — root mean square level, an approximation of perceived loudness.
- **Sample rate** — number of samples per second, typically 48000 Hz.
- **Underrun** — consumer needed samples but the ring buffer was empty.

## Architecture

- **AppState** — serialisable snapshot of everything the user can change.
- **AudioProcessor** — trait implemented by every DSP effect.
- **EffectChain** — ordered collection of named effect slots and processors.
- **EffectSlot** — name plus enabled flag for one stage of the chain.
- **Engine** — `AudioEngine`, owner of streams, buffers and shared state.
- **RtParam** — UI-owned shared parameter with an audio-side cached copy.
- **StreamFormat** — sample rate and channel count of a device.

## Desktop

- **Command** — Tauri bridge from JavaScript to a Rust function.
- **CSP** — Content Security Policy; currently disabled for the local UI.
- **invoke** — JavaScript function used to call a Tauri command.
- **Preset bundle** — JSON file containing multiple presets for export/import.
- **Toast** — short non-modal notification used for command results.
- **Tauri command argument** — camelCase in JavaScript, snake_case in Rust by default.

## Testing

- **Property test** — checks invariants that must hold for all inputs.
- **Round-trip test** — serialise then deserialise and compare the result.
- **Waveform test** — checks the shape or timing of an effect's output.
- **Harness** — small crate that includes desktop persistence modules so they
  can be tested without the Tauri runtime.
