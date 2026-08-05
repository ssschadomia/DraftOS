# 0005 — The install engine: plan/execute split with a dry-run safety gate

- **Status:** accepted
- **Date:** 2026-08-05

## Context

The installer must perform irreversible, root-level operations — partitioning,
`mkfs`, LUKS, `pacstrap`, bootloader install, user creation. That is dangerous to
write, dangerous to test, and must never run by accident on a normal system. It
also must stay decoupled from the GUI (ADR 0004: UI ≠ engine).

## Decision

A separate crate **`crates/draftos-install`** (library + `draftos-install` CLI)
with three cleanly separated concerns:

1. **`config`** — the `InstallRequest` contract (serde JSON). Passwords use a
   `Secret` type that redacts in `Debug`/logs but still serializes, so nothing
   leaks.
2. **Pure planning** — `plan(&InstallRequest) -> Vec<Step>` is a deterministic,
   side-effect-free function. A `Step` is data (`Run`/`WriteFile`, phase, title,
   `destructive` flag). The entire install can therefore be built, inspected and
   **unit-tested without touching a disk**.
3. **Execution** — runs a plan, or **dry-runs it (the default)**. Real execution
   is **refused unless we're in a DraftOS live environment** (`/run/archiso`) or
   explicitly forced with `DRAFTOS_INSTALL_FORCE=1` (VM only). This makes an
   accidental wipe of a running system impossible.

The GUI serializes its `InstallConfig` to an `InstallRequest`
(`InstallConfig::to_request`) and, on the live ISO, invokes `draftos-install`
via pkexec, parsing its `PROGRESS` lines. The Install screen already renders the
real planned steps.

Target layout: UEFI/GPT, FAT32 ESP at `/boot`, btrfs root with subvolumes
(`@`, `@home`, `@snapshots`, `@log`, `@cache`), optional LUKS2, systemd-boot.

## Consequences

- The risky surface is small, isolated and dry-runnable everywhere; the pure
  planner carries thorough unit tests (secret-leak, ordering, encryption, manual
  target, destructive flags).
- Standard `linux` kernel is supported now; **CachyOS kernel** and the
  **Alongside/Reinstall** install types are explicitly rejected until implemented,
  rather than mis-installing.
- End-to-end execution can only be validated in a VM booted from the live ISO —
  which is the next milestone.
