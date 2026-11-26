# Getting support

If AURIVO is not behaving as expected, the following information makes it much
easier to diagnose the problem without access to your machine.

## Before you ask

1. Restart the app and reproduce the problem once more.
2. Make sure the input and output devices in the sidebar are the ones you expect.
3. Check **Audio health** for overruns, underruns and lock skips.
4. Check the clip flag and peak readout; reduce input gain if they are hot.

## What to attach

- The **stream info** line (input rate, output rate, channel count).
- A screenshot of the sidebar, including audio health and latency.
- A **session state** dump from the Session state panel.
- The preset file, if the problem only happens with one preset.
- OS version and audio device names.

Never attach a recording that contains private conversation unless you are
comfortable sharing it; a WAV header from `aurivo-cli wav info` is usually
enough.

## Useful CLI checks

```bash
aurivo-cli state check state.json
aurivo-cli preset validate preset.json
aurivo-cli wav info recording.wav
aurivo-cli effects
```

Include the output of the relevant command in your report.

## Known limitations

AURIVO currently processes one mono stream, uses linear sample-rate
conversion, and has a feed-forward limiter. These are documented in
`docs/AUDIO-QUALITY.md`; behaviour caused by them is expected rather than a
bug.

## Response expectations

Issues are triaged in the order received. Include a clear title and reproduction
steps. Security reports should follow `docs/SECURITY.md` instead of a public
issue.
