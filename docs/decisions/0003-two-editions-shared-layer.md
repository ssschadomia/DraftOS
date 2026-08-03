# 0003 — Two editions over one shared layer

- **Status:** accepted
- **Date:** 2026-08-04

## Context

DraftOS targets two audiences with different needs: classic desktops/laptops that
want performance and full mutability, and users who want an atomic, self-healing
system. Both should feel like the *same* OS — same brand, same tools, same COSMIC
desktop — differing only in their base and update model.

## Decision

DraftOS is **one shared layer** staged onto **two edition bases**:

| Edition | Base | Mutability | Build tooling | Inspired by |
|---|---|---|---|---|
| **Desktop** | Arch + CachyOS kernel/repos | mutable (pacman + snapper) | archiso | CachyOS |
| **Immutable** | arkdep image (A/B, btrfs) | immutable | arkdep + archiso live ISO | Bazzite / SteamOS |

- The **shared layer** = branding + COSMIC presets + our tools/apps + common
  overlay files. It is defined once and staged into each edition's build.
- The **`draftos` CLI is edition-aware**: it drives `arkdep` on Immutable and
  `pacman` + `snapper` on Desktop, exposing the same verbs to the user.
- GUI apps are edition-agnostic.

## Consequences

- A tool or asset is authored in exactly one place and shipped to both editions.
- Editions can be built and released independently but stay visually identical.
- The Immutable edition is the more complex build (arkdep image + a live ISO that
  installs it); the Desktop edition is a more conventional archiso install.
- We do **not** literally fork CachyOS — "in the style of CachyOS" means a tuned,
  performance-oriented mutable Arch base using CachyOS's kernel and repos.
