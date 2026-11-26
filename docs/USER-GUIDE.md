# AURIVO user guide

## Getting started

1. Connect a microphone and headphones or speakers.
2. Press **Start**, or press Space when the window has focus.
3. Speak normally. The **Level** bar and spectrum should move.
4. Adjust **Gain dB** until the peak readout sits below about −6 dB with your
   loudest speaking voice. If **CLIP** lights up, reduce gain and press
   **Reset clip**.

If **Start** fails, the app will show a toast. The most common causes are a
device that does not support 32-bit float audio, or a device that is already
exclusively claimed by another application.

## The sidebar

- **Transport** — start/stop, device selection, input gain, level, peak,
  latency and clip flag.
- **Spectrum** — live FFT with a logarithmic frequency axis; the red line is
  peak hold.
- **Output** — monitoring gain and mute (M).
- **Configuration** — save/load the full session into the app config directory.
- **Session state** — dump/apply raw `AppState` JSON for support and sharing.
- **Audio health** — overruns/underruns and non-blocking lock skips.
- **Effects (enable / bypass)** — quick bypass toggles.
- **Effect chain** — reorder with the arrows or by dragging a row; the × button
  was removed because effects cannot be re-added at runtime.
- **Recorder** — record the processed signal, save a WAV and play it back.
- **Presets** — name, save, load, delete, install factory presets, export and
  import bundles.
- **Automation** — add an LFO to input gain, output gain or pitch.
- **Controllers** — map a MIDI-style channel/controller pair to a parameter and
  test it with the value slider.

## Effects

| Section | What it does |
|---------|--------------|
| Parametric EQ | Five bands by default; peaking, shelves and low/high-pass; each band can be disabled |
| Pitch | Granular shift from −24 to +24 semitones |
| Delay / Echo | 20–1200 ms, feedback, wet mix and damping |
| Reverb | Room size, damping, wet and dry levels |
| Distortion | Soft clip, hard clip or fuzz; drive, tone, mix, downsampling and bit depth |
| Modulation | Chorus, flanger, phaser or vibrato with rate and depth |
| Vocal FX | Ring mod, robotizer, vowel formant or tremolo |

Every parameter is applied live. Bypass toggles do not delete settings; turning
an effect back on restores the previous values.

## Recording

Press **Record** (or R) to capture the processed mono signal at the input rate.
**Save WAV** writes the take to the path shown in the sidebar. **Play** adapts
the file to the output device's rate and channel count, so a 44.1 kHz mono
recording plays correctly on a 48 kHz stereo device.

Pressing **Stop** in the transport also stops recording and playback.

## Presets

Saved presets contain all parameters, chain order, bypass states, EQ bands,
description and tags. Factory presets include Warm Vocal, Radio, Shred, Clean,
Space, Robot, Whisper and Bass. **Export** writes a JSON bundle; **Import** adds
presets without overwriting existing names.

## Automation and controllers

Automation routes add an LFO offset to a base parameter value. Removing a route
or pressing **Clear all** restores the base value. Controllers scale an incoming
0..1 value into a parameter range; one controller can drive several targets.

## Keyboard shortcuts

| Key | Action |
|-----|--------|
| Space | Start the engine |
| Shift+Space | Stop the engine |
| R | Toggle recording |
| M | Toggle output mute |

Shortcuts are ignored while a text field, slider or select has focus.

## Troubleshooting

- **No spectrum movement** — check that the input device is selected and that
  the input gain is not fully down.
- **Audio glitches** — watch overruns/underruns and lock skips; reduce system
  load or increase the OS device buffer.
- **Unexpected pitch** — check that input and output rates match; resampling is
  reported in the stream info line.
- **Preset will not load** — run `aurivo-cli preset validate <file>` and check
  the reported problems.
- **Config missing after restart** — configuration is stored in the platform
  app config directory, not next to the executable; do not delete that folder.
