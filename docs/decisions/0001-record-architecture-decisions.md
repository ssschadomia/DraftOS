# 0001 — Record architecture decisions

- **Status:** accepted
- **Date:** 2026-08-04

## Context

The previous iteration of DraftOS accumulated several parallel narrative
documents (CONTEXT, STATUS, PROGRESS, ROADMAP) that overlapped and drifted out of
sync. The *why* behind key choices was buried in prose and hard to find or update.

## Decision

We keep an **Architecture Decision Record (ADR)** log under `docs/decisions/`.
Each significant, hard-to-reverse decision gets one numbered file:

- Filename: `NNNN-short-title.md` (zero-padded, kebab-case).
- Sections: Context · Decision · Consequences, plus a Status/Date header.
- Statuses: `proposed` → `accepted` → optionally `superseded by NNNN`.

Superseded ADRs are kept, not deleted — the history is the point.

## Consequences

- The reasoning behind a choice lives next to the choice and survives refactors.
- New contributors can read the log top-to-bottom to understand how we got here.
- Day-to-day status lives in git history and area READMEs, not in prose files
  that drift.
