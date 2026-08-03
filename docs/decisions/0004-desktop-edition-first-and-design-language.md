# 0004 — Desktop edition first; Rust + libcosmic for all UI

- **Status:** accepted
- **Date:** 2026-08-04

## Context

DraftOS defines two editions ([ADR 0003](0003-two-editions-shared-layer.md)), but
effort should converge before it spreads. The user experience is the product's
whole point: DraftOS aims for macOS-level polish and coherence while keeping
Linux's openness. That bar is set by the first-run software users actually touch —
the installer, the initial-setup wizard, and the Hello/Welcome app.

## Decision

1. **Desktop edition is the current focus.** We build out core infrastructure for
   the mutable Arch + CachyOS edition first; the Immutable edition follows once the
   shared UX layer is solid. (The GUI apps are largely edition-agnostic, so this
   work benefits both.)
2. **All user-facing UI is Rust + libcosmic.** No GTK, no other toolkit. This keeps
   one toolchain, matches the COSMIC desktop natively (real glass, consistent
   theming), and lets apps share a design language and helper crate.
3. **One design language, macOS-grade.** Every app follows the same conventions:
   generous spacing, a clear type hierarchy, card/list containers, a single accent,
   and COSMIC's native glass. Shared UI helpers are extracted into a crate
   (`crates/draftos-ui`) once a second app needs them — not before (avoid premature
   abstraction). The Hello app (`apps/welcome`) is the living reference.
4. **libcosmic is pinned** to one revision in the workspace so every app tracks the
   same API (see root `Cargo.toml`).

## Consequences

- Near-term build order: Hello (design reference) → initial-setup wizard →
  installer → the rest, all as libcosmic apps in `apps/`.
- The Immutable edition's image recipe is deferred (still available as reference in
  the prior iteration) until the Desktop UX foundation is in place.
- Choosing libcosmic ties us to its release cadence; mitigated by pinning and by
  extending rather than forking ([ADR 0002](0002-desktop-is-cosmic.md)).
