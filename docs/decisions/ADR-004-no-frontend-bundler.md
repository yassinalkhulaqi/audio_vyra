# ADR-004: No build step for the frontend

- Status: Accepted
- Date: 2026-11-26

## Context

A bundler would allow TypeScript and component frameworks but would add a
Node toolchain, a build step in CI, and a second dependency graph.

## Decision

Serve plain `index.html`, `styles.css` and `app.js` from `dist/`. Enable
`withGlobalTauri` so no npm package is required.

## Consequences

- The app can be built with Cargo alone.
- State management is procedural and duplicated in places; a future framework migration should start by introducing a typed API wrapper.
- All UI errors must be surfaced through `toast()` because there is no compiler checking JS.
