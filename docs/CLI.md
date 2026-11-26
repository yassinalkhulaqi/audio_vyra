# AURIVO CLI

`aurivo-cli` is a small diagnostic tool for inspecting the JSON and WAV files
produced by the desktop app. It has no audio device dependency and can run on a
machine without a sound card.

## Build

```bash
cargo run -p aurivo-cli -- help
```

## Commands

### Presets

```bash
aurivo-cli preset inspect presets/Warm_Vocal.json
aurivo-cli preset validate presets/Warm_Vocal.json
aurivo-cli preset list presets
```

`inspect` prints the preset name, format version, parameter table and the
effect chain with bypass states. `validate` reports missing names, non-finite
parameters, empty chains and bypass entries that do not match the chain.
`list` scans a directory and marks each JSON file as valid or invalid.

### Session state

```bash
aurivo-cli state check state.json
aurivo-cli state diff before.json after.json
```

`check` reports which `AppState` fields are present. `diff` prints a table of
changed fields, including array and object sizes. This is useful when
comparing state dumps before and after a preset load.

### Recordings

```bash
aurivo-cli wav info recording.wav
```

Prints the container format, channel count, sample rate, bit depth, frame
count and duration. It reads the RIFF chunks directly and does not decode
audio samples.

### Effects

```bash
aurivo-cli effects
```

Lists the default twelve-stage chain in signal-flow order.

## Exit codes

- `0` — command completed.
- `1` — invalid arguments, unreadable file, malformed JSON or an unsupported WAV.

Errors are printed to stderr with an `aurivo:` prefix so they can be piped
separately from formatted output.
