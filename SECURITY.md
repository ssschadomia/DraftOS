# Security Policy

DraftOS is an operating system, so we take security reports seriously — both in
our own code and in how we assemble upstream components.

## Reporting a vulnerability

**Please do not report security issues in public GitHub issues.**

Instead, report privately via one of:

- GitHub's **[Private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing/privately-reporting-a-security-vulnerability)**
  (Security → Report a vulnerability on this repository), or
- Email **security@draftos.org** (update this to a working address before launch).

Please include:

- A description of the issue and its impact.
- Steps to reproduce, or a proof of concept.
- The affected edition/component and version or commit.

We aim to acknowledge reports within a few days and to keep you updated as we
investigate. We'll credit you in the advisory unless you prefer to stay anonymous.

## Scope

- **In scope:** DraftOS's own code — the `draftos*` tools, the installer, the
  libcosmic apps, build recipes and scripts in this repository.
- **Upstream components** (Arch, COSMIC, the kernel, etc.) should be reported to
  their respective projects; we'll help coordinate where a DraftOS default makes
  an upstream issue materially worse.

## Supported versions

DraftOS is pre-release. Until a stable release is tagged, only the latest `main`
is supported for security fixes.
