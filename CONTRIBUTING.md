# Contributing to DraftOS

Thanks for your interest in DraftOS! This document explains how to build the
project, the conventions we follow, and how to get a change merged.

By participating you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

## Project layout

DraftOS is a single Cargo workspace plus per-edition build definitions. See
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full map. In short:

- `crates/`, `apps/`, `cli/` — Rust source (one workspace).
- `desktop/`, `brand/`, `overlay/` — static assets staged into images.
- `editions/` — per-edition build definitions.
- `infra/` — hosting/deployment (Cloudflare R2, etc.).
- `docs/decisions/` — Architecture Decision Records (ADRs).

## Building

You need a recent stable Rust toolchain (`rustup` recommended) and a C linker
(`gcc`). Then:

```bash
cargo build              # build everything
cargo test               # run the test suite
cargo clippy --workspace --all-targets -- -D warnings   # lint (must be clean)
```

All code must build and pass `clippy -D warnings` before it can be merged.

### Previewing the GUI apps

The libcosmic apps render as normal Wayland/X11 clients. `tools/shot.sh`
screenshots an app window for review (see its header for usage). Many apps expose
a dev hook to jump straight to a screen, e.g. `DRAFTOS_WELCOME_PAGE=1` or
`DRAFTOS_INSTALLER_STEP=6`.

## Making changes

1. **Branch** off `main` (never commit directly to `main`).
2. Keep changes **focused** — one logical change per pull request.
3. **Match the surrounding code**: naming, comment density, and idiom.
4. If your change makes a non-obvious, hard-to-reverse decision, add an **ADR**
   under `docs/decisions/` (`NNNN-short-title.md`).
5. Add or update **tests** for behavior changes. Pure logic (parsing, detection)
   should be unit-tested; keep it separate from UI so it stays testable.

## Commits & pull requests

- Write clear, imperative commit subjects scoped by area, e.g.
  `apps/installer: add Time zone step`.
- Explain the *why* in the body when it isn't obvious.
- Ensure `cargo build`, `cargo test`, and `cargo clippy -D warnings` all pass.
- Open a PR against `main` and fill in the template.

## Reporting bugs & ideas

Use the issue templates (Bug report / Feature request). For anything
security-sensitive, follow [SECURITY.md](SECURITY.md) instead of opening a public
issue.

## License

DraftOS is **GPL-3.0-or-later**. By contributing, you agree that your
contributions are licensed under the same terms.
