# Security notes

AURIVO writes user files and receives untrusted JSON from presets and session
dumps. This document records the controls in place.

## Path handling

Every path supplied by the UI goes through `apps/desktop/src-tauri/src/paths.rs`:

- absolute paths are rejected,
- parent-directory traversal (`..`) is rejected,
- only normal path components are accepted,
- resolved paths must remain under the app config directory.

File names are further sanitised by `safe_file_name` before use. In addition,
`presets::safe_name` limits preset file names to ASCII alphanumerics, dash and
underscore.

## Atomic writes

Config and preset writes use a temporary file in the same directory followed by
`fs::rename`. A crash during a write leaves the previous file intact. Temporary
files are removed if the rename fails.

## Input validation

- `aurivo_dsp::validate` clamps every parameter that can arrive from JSON
  before it reaches DSP math.
- `ProcessingPipeline::process` replaces non-finite samples with silence.
- Effect modes are clamped to known ranges in the engine setters.
- EQ band frequencies, Q and gains are checked by `presets::validate`.

## WebView

`withGlobalTauri` is enabled because the UI has no npm package. The Content
Security Policy is currently disabled (`csp: null`) to allow the inline
stylesheet and script. The UI never evaluates remote content, and dynamic
option lists are built with `document.createElement` and `textContent` rather
than `innerHTML`.

## Reporting a security issue

Please report privately to the repository owner with reproduction steps. Do not
open a public issue for an unpatched vulnerability.
