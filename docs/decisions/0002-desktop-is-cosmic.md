# 0002 — The desktop is COSMIC, extended not forked

- **Status:** accepted
- **Date:** 2026-08-04 (carried forward from the prior iteration, re-affirmed)

## Context

DraftOS needs a cohesive, distinctive desktop experience — real transparency /
"glass", a consistent visual rhythm, and the best ergonomics of macOS — without
committing to maintaining a compositor fork forever.

- GNOME/Mutter cannot deliver real per-window blur/transparency (a proven ceiling)
  and actively punishes customization.
- **COSMIC** (System76, Rust) is modular, customizable and distro-agnostic *by
  design*, and its compositor supports real glass out of the box via a theme
  toggle (`frosted_windows`), not a source patch.

## Decision

DraftOS's desktop is **COSMIC**. We **extend** it, we do **not fork** it:

- Own the experience through **configuration, theming, branding and wallpaper**
  (RON presets shipped as system defaults under `/usr/share/cosmic`).
- Add **first-party libcosmic apps** (store, welcome, control, companion) rather
  than patching upstream components.
- Use COSMIC's supported customization points where needed; do **not** modify
  `cosmic-comp` sources.

"Glass" is therefore a configuration concern (`frosted_windows: true`), never a
compositor fork.

## Consequences

- We track upstream COSMIC releases as packages instead of rebasing a fork.
- Our differentiation lives in `desktop/cosmic/` (presets) and `apps/` (libcosmic).
- If a needed capability is genuinely impossible without a fork, that is a new ADR
  decision, made explicitly — not the default path.
