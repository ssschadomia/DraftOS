# infra — hosting & deployment

How DraftOS is served to users. The project is designed to run on Cloudflare's
free/generous tiers so it costs nothing to keep alive (no always-on VPS):

| What | Where | Service |
|---|---|---|
| ISO images, package repo, Flatpak repo, image tarballs | `cloudflare/` | **R2** (object storage, no egress fees) |
| Project website & docs (later) | — | **Pages** (static hosting) |
| Optional nicer serving / auto-index for the CDN | `cloudflare/worker/` | **Workers** |

Build artifacts are published as **GitHub Release assets** by CI and mirrored to
**R2** so downloads and repositories are served from Cloudflare's CDN.

## Cloudflare R2

See [cloudflare/R2.md](cloudflare/R2.md) for the bucket layout and the full setup
runbook (create the bucket, tokens, public access / custom domain, and how to
upload). The pieces:

- `cloudflare/rclone.conf.example` — rclone remote for R2's S3 API.
- `cloudflare/upload.sh` — sync a local `dist/` tree to the bucket.
- `cloudflare/wrangler.toml` + `cloudflare/worker/index.js` — optional Worker that
  serves the bucket with range support, cache headers, and directory indexes.
- `.github/workflows/publish-r2.yml` — mirrors release assets to R2 automatically.

> These are prepared **deploy-ready**. Creating the Cloudflare account, the bucket,
> and the API tokens are account-level steps you do once in the dashboard — the
> runbook lists them exactly.
