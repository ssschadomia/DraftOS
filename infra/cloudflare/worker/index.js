// DraftOS CDN worker — serves an R2 bucket over HTTP.
//
// Features beyond a plain public bucket:
//   - HTTP Range requests (resumable ISO downloads)
//   - Long, immutable caching for artifacts; short caching for repo indexes
//   - Auto-generated directory listings for prefixes ending in "/"
//
// Bound to the bucket via wrangler.toml ([[r2_buckets]] binding = "BUCKET").

export default {
  async fetch(request, env) {
    if (request.method !== "GET" && request.method !== "HEAD") {
      return new Response("Method Not Allowed", { status: 405, headers: { allow: "GET, HEAD" } });
    }

    const url = new URL(request.url);
    const key = decodeURIComponent(url.pathname.replace(/^\/+/, ""));

    // Directory listing for "" or "prefix/".
    if (key === "" || key.endsWith("/")) {
      return listing(env, url, key);
    }

    const range = parseRange(request.headers.get("range"));
    const object = await env.BUCKET.get(key, range ? { range } : undefined);

    if (!object) {
      // If it's actually a directory, redirect to the trailing-slash form.
      const probe = await env.BUCKET.list({ prefix: key + "/", delimiter: "/", limit: 1 });
      if (probe.objects.length || probe.delimitedPrefixes.length) {
        return Response.redirect(url.origin + "/" + key + "/", 301);
      }
      return new Response("Not found", { status: 404 });
    }

    const headers = new Headers();
    object.writeHttpMetadata(headers);
    headers.set("etag", object.httpEtag);
    headers.set("accept-ranges", "bytes");
    headers.set("cache-control", cacheControl(key));

    const body = request.method === "HEAD" ? null : object.body;
    if (range && object.range) {
      const start = object.range.offset;
      const end = start + object.range.length - 1;
      headers.set("content-range", `bytes ${start}-${end}/${object.size}`);
      headers.set("content-length", String(object.range.length));
      return new Response(body, { status: 206, headers });
    }
    return new Response(body, { headers });
  },
};

// Cache immutable artifacts hard; keep mutable repo metadata fresh.
function cacheControl(key) {
  if (/\.(db|db\.tar\.gz|files\.tar\.gz)$/.test(key) || key.endsWith("/summary")) {
    return "public, max-age=60, must-revalidate";
  }
  if (/\.(iso|pkg\.tar\.zst|tar\.zst|sha256|sig)$/.test(key)) {
    return "public, max-age=31536000, immutable";
  }
  return "public, max-age=300";
}

// Parse a single-range "bytes=start-end" header into an R2 range option.
function parseRange(header) {
  if (!header) return null;
  const m = /^bytes=(\d*)-(\d*)$/.exec(header.trim());
  if (!m) return null;
  const [, s, e] = m;
  if (s === "" && e === "") return null;
  if (s === "") return { suffix: Number(e) };
  const offset = Number(s);
  return e === "" ? { offset } : { offset, length: Number(e) - offset + 1 };
}

// Minimal HTML index for a prefix.
async function listing(env, url, prefix) {
  const list = await env.BUCKET.list({ prefix, delimiter: "/" });
  const dirs = list.delimitedPrefixes.map((p) => p.slice(prefix.length)).sort();
  const files = list.objects
    .filter((o) => o.key !== prefix)
    .map((o) => ({ name: o.key.slice(prefix.length), size: o.size }))
    .sort((a, b) => a.name.localeCompare(b.name));

  const rows = [];
  if (prefix) rows.push(`<li><a href="../">../</a></li>`);
  for (const d of dirs) rows.push(`<li><a href="${esc(d)}">${esc(d)}</a></li>`);
  for (const f of files) rows.push(`<li><a href="${esc(f.name)}">${esc(f.name)}</a> <span>${human(f.size)}</span></li>`);

  const html = `<!doctype html><meta charset="utf-8">
<title>${esc(env.SITE_NAME || "DraftOS")} — /${esc(prefix)}</title>
<style>body{font:15px/1.6 system-ui,sans-serif;max-width:820px;margin:3rem auto;padding:0 1rem}
h1{font-size:1.1rem}ul{list-style:none;padding:0}li{display:flex;justify-content:space-between;border-bottom:1px solid #eee;padding:.3rem 0}
span{color:#888}a{text-decoration:none}</style>
<h1>Index of /${esc(prefix)}</h1><ul>${rows.join("")}</ul>`;
  return new Response(html, {
    status: 200,
    headers: { "content-type": "text/html; charset=utf-8", "cache-control": "public, max-age=60" },
  });
}

function esc(s) {
  return s.replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
}

function human(n) {
  const u = ["B", "KB", "MB", "GB", "TB"];
  let i = 0;
  while (n >= 1024 && i < u.length - 1) { n /= 1024; i++; }
  return `${n.toFixed(i ? 1 : 0)} ${u[i]}`;
}
