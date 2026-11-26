# FAQ

## Why does AURIVO process only mono?

Voice effects are most predictable on a single stream. Stereo input is
downmixed explicitly and the processed mono signal is duplicated to every
output channel. This avoids channel-dependent timing and keeps the ring buffer
simple. Per-channel processing is on the roadmap.

## Why does Start fail with "device does not support f32"?

AURIVO requests 32-bit float streams from `cpal`. Most modern drivers expose
F32, but some interfaces only offer 16 or 24 bit formats. Try a different
device or change the OS audio format settings.

## What do overruns and underruns mean?

- **Overrun** — the input callback produced more samples than the ring could
  hold; audio was dropped.
- **Underrun** — the output callback needed samples but the ring was empty;
  silence was substituted.

Both usually mean the OS audio callback is being starved. Close other
real-time applications, increase the device buffer size, or reduce CPU load.

## What do the "skips" counters mean?

They count times the audio callback found a parameter lock held by the UI and
reused the last known good values instead of waiting. A few skips are normal
while dragging sliders. Sustained high counts indicate the UI is doing heavy
work on a shared lock.

## Why is the limiter always on?

It is the final safety stage that catches resonant peaks from EQ and
distortion. It can be bypassed from the Effects chain panel if you want a fully
transparent signal path, but then you are responsible for headroom.

## Where are presets and config stored?

In the platform's application config directory, not next to the executable.
The Configuration panel writes `config.json` there, and the preset directory is
resolved relative to it. This keeps user data separate from program files.

## Can I share a preset with someone else?

Yes. Use **Export** in the Presets panel to write a JSON bundle, then send that
file. The recipient can **Import** it without overwriting existing presets of
the same name.

## Why did my preset lose its bypass states?

Older presets did not store them. Loading such a preset now migrates it by
filling in the default chain and all-enabled map. Saving again writes the full
state.

## What does automation actually modulate?

Input gain (dB), output gain (dB) and pitch (semitones). The base value is
captured when the route is added; the LFO adds an offset around it. Clearing
routes restores the base values.

## Can a controller drive more than one parameter?

Yes. Add multiple bindings with the same channel and controller. Each can have
its own target and range.

## Does recording include the limiter?

Yes. Recording taps the processed mono signal after the whole chain, so the
WAV matches what you hear through the output (before output gain and mute).

## Why does my 44.1 kHz WAV play at the right speed?

Playback adapts the file's sample rate and channel count to the output device
before streaming. The CLI command `aurivo-cli wav info <file>` shows the
source format.

## How do I report an audio bug?

Include the stream info line, audio health counters, a state dump, OS and device
names, and the exact steps to reproduce. Attach a preset if one is involved.
