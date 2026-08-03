# Credits & third-party notices

DraftOS's own code — the `draftos*` tools, `draftbox`, `draftos-link`, the
libcosmic apps, and the recipes and scripts in this repository — is
**© 2026 DraftOS contributors, GPL-3.0-or-later** (see [LICENSE](LICENSE)).

DraftOS is an **Arch-based** distribution assembled from a great deal of upstream
free software. Those components keep **their own licenses and copyrights** — we do
not relicense or claim them. Trademarks belong to their respective owners; names
below are used descriptively (nominative), and **no endorsement or affiliation is
implied**.

## Core upstreams

| Component | Role in DraftOS | License |
|---|---|---|
| **Arch Linux** | base system & packages | packages under their own licenses; "Arch" used descriptively — DraftOS is not official Arch |
| **COSMIC** (System76) | desktop environment we extend | GPL-3.0 / MPL-2.0 (per component) |
| **libcosmic / iced** | toolkit for our first-party apps | MPL-2.0 / MIT |
| **arkdep / Arkane Linux** | atomic engine, image build, repo & keyring (Immutable edition) | GPL-3.0 |
| **CachyOS** | kernel + optimized repos (Desktop edition) | kernel GPL-2.0; packages under their own licenses |
| **archiso** | live ISO build tooling | GPL-3.0 |
| **KDE Connect** | protocol reference for the phone ecosystem | GPL-2.0-or-later / GPL-3.0 |
| **scrcpy** (Genymobile) | phone screen mirroring/control | Apache-2.0 |
| **Flatpak** | app delivery engine behind DraftOS Store | LGPL-2.1+ |
| **distrobox** | dev containers behind draftbox | GPL-3.0 |
| **systemd, dracut, btrfs-progs, PipeWire, NetworkManager, …** | plumbing | respective (LGPL/GPL/MIT) |

"Inspired by **Bazzite / SteamOS**" (Immutable edition) and "inspired by
**CachyOS**" (Desktop edition) describe design influence only.

## Obligations we honor

- Keep upstream license texts and copyright notices intact in the shipped image.
- For GPL/LGPL components we redistribute, corresponding source is available from
  the upstream repositories; DraftOS adds no proprietary modifications to them.
- Our own additions are GPL-3.0-or-later, with source in this repository.
