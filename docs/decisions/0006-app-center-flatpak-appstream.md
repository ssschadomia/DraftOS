# 0006 — The App Center: Flatpak + Flathub via AppStream

- **Status:** accepted
- **Date:** 2026-08-07

## Context

DraftOS needs a graphical software store — the "App Center" — with the polish and
richness of the Microsoft Store: a browsable catalog with categories, featured
apps, screenshots, rich descriptions, ratings, versions/changelogs, and one-click
install/update/manage. The Desktop edition is mutable Arch, so the theoretical
package sources are pacman (official repos), the AUR, and Flatpak (Flathub).

The store-grade experience the product wants depends on **metadata** that native
Arch packages simply do not carry: pacman packages have no screenshots, no curated
descriptions, no category art, no developer/branding fields. Building a
screenshot-driven storefront on pacman metadata is not possible.

## Decision

The App Center's **primary (v1) backend is Flatpak + Flathub**, driven by the
local **AppStream** catalog:

- **Metadata** comes from the on-disk AppStream catalog that Flatpak already keeps
  in sync (`.../flatpak/appstream/<remote>/<arch>/active/appstream.xml[.gz]`):
  names, summaries, full descriptions, categories, keywords, screenshots,
  versions/changelogs, OARS content ratings, developer, license, homepage.
- **Icons** come from the catalog's local icon cache (`icons/128x128/<id>.png`) —
  no network needed to render the storefront.
- **Actions** (install/remove/update/launch) shell out to the `flatpak` CLI,
  installing to the **user** installation (`--user`) so no root prompt is needed
  for everyday use. This mirrors the engine pattern (ADR 0005): the UI never links
  the package machinery, it drives a well-defined external tool.
- **Screenshots** are fetched lazily over the network (Flathub CDN URLs from the
  catalog) and cached under `~/.cache/draftos-store/`.

This is the same architecture GNOME Software and the COSMIC Store use, and the
Flatpak-first storefront is what Pop!_OS and Bazzite ship.

## Consequences

- The DraftOS image must ship `flatpak` and pre-configure the **Flathub** remote
  (and run an initial `--appstream` sync) so the store has a catalog on first boot.
- A pluggable `Source` seam is kept so a **native pacman/AUR** source can be added
  later as a second catalog tab — but without the rich storefront treatment, since
  the metadata isn't there.
- The catalog parse (~48 MB XML, ~4.7k apps) runs once in a background task at
  startup; the UI shows a loading state until it completes.
- Filtered remotes (e.g. Fedora's FOSS-only Flathub filter used on the dev host)
  limit what is installable but not what the parser can read — development against
  real data works on the Fedora host.
