# AURIVO — Improvement Plan v2

Status: all 9 original feature branches merged into `main`, followed by the
audio-correctness, real-time-safety, DSP-correctness, test-expansion, UI-split,
UI-controls, presets-v3 and config-security workstreams. This plan is retained
for historical context; the current system is described in
[`ARCHITECTURE.md`](ARCHITECTURE.md) and the decision records in `decisions/`.

Remaining optional work from the original plan: MIDI integration, per-parameter
automation, A/B compare, latency compensation and installer packaging.

Guiding principles:
- Keep the DSP core dependency-free and real-time safe.
- One source of truth for the effect chain and its parameters.
- Small, mergeable steps; no big-bang rewrites.

---

## Phase 0 — Housekeeping (30 min, do first)

| Task | Why |
|------|-----|
| Update `README.md`: move the 9 branches from "In flight" to "Included" | Docs are now wrong |
| Add per-effect bypass toggles in the UI | Every DSP type already supports `set_bypass`, nothing exposes it |
| Remove unused imports (`AudioConfig`, `DeviceInfo`) from engine | Clean compile warnings |
| Add `AGENTS.md` with build/test commands | Future contributors (and agents) need them |

---

## Phase 1 — Architecture (1–2 days, highest long-term value)

### 1.1 Replace the monolithic `AudioEngine`
`crates/aurivo-audio-engine/src/lib.rs` is now 320 lines with 15 effect fields and 38 forwarding methods. Refactor to:

```rust
pub struct EffectSlot {
    name: &'static str,
    processor: Box<dyn AudioProcessor>,   // add remove/bypass/reorder to the trait
    params: EffectParams,                 // enum of shared handles
    enabled: Arc<AtomicBool>,
}
pub struct AudioEngine {
    chain: Arc<Mutex<Vec<EffectSlot>>>,
    // ...devices, meter, recorder, analyzer
}
```

- Extend `AudioProcessor` with `name()`, `set_enabled()`, `snapshot_params()`.
- Add chain operations: `add`, `remove`, `move`, `set_enabled`, `reorder`.
- Engine becomes ~150 lines; adding a new effect touches one module, not 5 files.

### 1.2 One shared state snapshot
Define a serializable `AppState` containing all effect params + chain order. Tauri `get_state` / `set_state` replace the 38 individual commands. This also fixes the current gap where presets only save gain + vocal.

### 1.3 Config file on disk
`~/.config/aurivo/config.json` for device choice, chain, and last state; loaded on startup.

---

## Phase 2 — Audio correctness (2–3 days)

| Issue today | Fix |
|-------------|-----|
| Input and output rates can differ; ring buffer just copies samples | Add linear or windowed-sinc resampler in the output callback |
| Chain treats interleaved multi-channel as mono | Process per-channel or downmix explicitly; expose channel count in UI |
| Ring buffer overflow silently drops audio | Track `overruns`/`underruns` atomics, show them in the UI |
| No clip indication | Add peak-hold clip flag to the meter; red bar at ≥ 0 dBFS |
| Denormals can slow DSP | `FTZ/DAZ` on x86 or add tiny DC offset in hot loops |
| `stop()` doesn't stop recording/playback | Make `stop()` tear down all three streams consistently |

---

## Phase 3 — UI/UX (2–3 days)

- **Layout:** left sidebar (devices, transport, levels, presets) + right panel with collapsible effect cards; the current single 450-line page stacks 10 sections vertically.
- **Bypass per card** with an on/off switch; dragging cards reorders the DSP chain (ties into 1.1).
- **Presets v2:** save/load *all* params + chain order + bypass states; factory presets shipped as JSON.
- **Meters:** history sparkline for RMS/peak, spectrum with log-frequency axis and peak hold.
- **Feedback:** toast notifications for save/load/errors instead of `alert()`; show device sample rate and measured latency.
- **Keyboard:** Space = start/stop, R = record, M = mute.
- **Accessibility:** labels tied to inputs, focus outlines, reduced-motion friendly.

---

## Phase 4 — Tests & CI (1 day)

Earlier constraint said no tests, but for long-term health this is the highest-value insurance:

- Unit tests for every DSP module (gain accuracy, gate envelope, compressor ratio, biquad magnitude response, delay decay, reverb tail, EQ band shape, pitch shift, FFT bins, recorder WAV round-trip).
- Property-style tests: silence in → silence out; NaN/inf in → finite out; no panics on extreme params.
- Engine integration test with a dummy cpal host if feasible.
- CI: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `cargo build --release`.

---

## Phase 5 — Features (optional, by demand)

- **Automation:** per-param LFO with rate/depth and BPM sync.
- **MIDI:** `midir` for expression pedals / footswitches (bypass, tap tempo).
- **Latency compensation** for recording (align recorded file to monitored signal).
- **A/B compare:** snapshot two states, instant switch with crossfade.
- **Plugin host:** CLAP support would be a large undertaking; evaluate only after phases 1–4.

---

## Phase 6 — Packaging & release (1 day)

- Set `bundle.active = true` in `tauri.conf.json`; produce MSI/NSIS, .deb, .dmg.
- Real app icon, about dialog, license file.
- Changelog and semantic versioning from `v0.1.0`.

---

## Recommended order

1. Phase 0 (quick wins) and Phase 4 (tests) first — cheap and they de-risk everything else.
2. Phase 1.1/1.2 — the refactor that makes all later work easy.
3. Phase 2 — audio correctness before UI polish.
4. Phase 3 — new UI on top of the stable `AppState` API.
5. Phases 5–6 as time allows.

Estimated total: ~8–10 focused days. Each phase ends with a mergeable commit; no push unless you ask.
