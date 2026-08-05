# DraftOS

An **Arch-based** Linux distribution with a cohesive, first-party experience built
on the **COSMIC** desktop. DraftOS ships in two editions from one shared layer:

| Edition | Base | Mutability | Inspired by |
|---|---|---|---|
| **Desktop** | Arch + CachyOS kernel/repos | mutable | CachyOS |
| **Immutable** | arkdep image (A/B, btrfs) | immutable | Bazzite / SteamOS |

The desktop is **COSMIC, extended not forked** — we own the experience through
configuration, theming, branding and our own first-party libcosmic apps, without
maintaining a fork of the compositor.

> Status: **early scaffold** — clean rebuild started 2026-08-04. See
> [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the repo map and
> [docs/decisions/](docs/decisions/) for the decision log (ADRs).

## Repository layout

```
brand/        logos, wordmark, palette, wallpapers, os-release
desktop/      COSMIC customization (RON presets → /usr/share/cosmic)
crates/       shared Rust libraries + non-GUI components (draftos-link, …)
apps/         first-party libcosmic GUI apps (store, welcome, control, companion)
cli/          command-line tools (draftos, draftbox)
overlay/      shared rootfs overlay staged into both editions
editions/     per-edition build definitions (immutable/, desktop/)
build/        build scripts, container definitions, CI helpers
docs/         architecture + decision records
```

All Rust code lives in a single Cargo workspace rooted at [Cargo.toml](Cargo.toml).

## Downloads & hosting

Release builds are published as **GitHub Release assets** and mirrored to
**Cloudflare R2** (CDN, no egress fees). The package and Flatpak repositories are
served from the same bucket — see [infra/](infra/) for the layout and setup.

## Contributing & community

Contributions are welcome! Start with [CONTRIBUTING.md](CONTRIBUTING.md), and note
the [Code of Conduct](CODE_OF_CONDUCT.md). Found a vulnerability? Please follow
[SECURITY.md](SECURITY.md) rather than opening a public issue.

## License

DraftOS's own code is **GPL-3.0-or-later** ([LICENSE](LICENSE)). Upstream
components keep their own licenses — see [CREDITS.md](CREDITS.md).
