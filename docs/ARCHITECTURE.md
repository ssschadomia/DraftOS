# Architecture

This document is the map of the repository and the principles that keep it clean.
Decisions with lasting consequences are recorded as ADRs under
[decisions/](decisions/); this file describes the *structure* those decisions produce.

## Principles

1. **Source and filesystem-image are separate.** We never edit programs in place
   inside a rootfs overlay tree. Source lives in `crates/`, `apps/`, `cli/`;
   the build **stages** compiled binaries and assets into overlays. This was the
   main source of debt in the previous iteration and is deliberately avoided.
2. **One Rust workspace.** Every Rust component shares one dependency graph and
   lockfile (`Cargo.toml`). Shared logic goes in `crates/draftos-common`.
3. **One shared layer, many editions.** Everything both editions have in common
   (branding, tools, desktop config) is defined once and staged into each
   edition's build. Editions differ only in their base and mutability model.
4. **Extend COSMIC, don't fork it** ([ADR 0002](decisions/0002-desktop-is-cosmic.md)).
   We own the experience via config/theme/brand + first-party apps.
5. **Decisions are written down.** Non-obvious choices get an ADR so the *why*
   survives. Docs stay compact — no parallel status files that drift apart.

## Repository map

| Path | Contains | Staged into image? |
|---|---|---|
| `brand/` | logos, wordmark, palette, wallpapers, `os-release` template | yes (via overlay) |
| `desktop/cosmic/` | COSMIC RON presets (panel, dock, theme) → `/usr/share/cosmic` | yes |
| `crates/` | shared Rust libs + non-GUI components (`draftos-common`, `draftos-link`) | binaries staged |
| `apps/` | first-party libcosmic GUI apps (store, welcome, control, companion) | binaries + `.desktop` staged |
| `cli/` | `draftos`, `draftbox` (Rust) | binaries staged |
| `overlay/` | static rootfs files common to both editions (systemd units, xdg, etc.) | yes |
| `editions/immutable/` | arkdep image recipe + live ISO (Bazzite-style) | — |
| `editions/desktop/` | archiso + CachyOS repos (CachyOS-style) | — |
| `build/` | build scripts, `Containerfile`s, staging logic, `ci/` | — |
| `docs/` | this file + `decisions/` (ADRs) | — |

## Build model (target shape)

```
   source (crates/ apps/ cli/)        static assets (brand/ desktop/ overlay/)
              │                                     │
              ▼  cargo build --release              │
        compiled binaries  ─────────┐               │
                                     ▼               ▼
                          build/stage-layer.sh  →  staging overlay
                                     │
                 ┌───────────────────┴───────────────────┐
                 ▼                                         ▼
        editions/immutable  (arkdep)            editions/desktop  (archiso)
                 │                                         │
                 ▼                                         ▼
          draftos-*.tar.zst                         draftos-*.iso
```

The shared layer is assembled once and consumed by both edition builds, so a tool
or asset is authored in exactly one place.

## Status of each area

Tracked in [decisions/](decisions/) and each area's own `README.md`. Top-level
project status lives in this repo's git history and the README badge line — we do
not keep separate CONTEXT/STATUS/PROGRESS files (they drifted last time).
