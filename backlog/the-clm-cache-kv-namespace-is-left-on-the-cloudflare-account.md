---
title: "The clm-cache KV namespace is left on the Cloudflare account with nothing bound to it"
kind: human-task
components: [web]
---

`web/wrangler.toml` no longer binds a KV namespace and `web/README.md` no longer lists one under "What the setup made", since the Worker reads none (`web.worker/cache-is-disposable`). The namespace itself, `clm-cache` (id `9fa493aaa0a54d219b2dfebb5f681b9e`), is still on the operator's Cloudflare account, holding only a disposable cache derived from GitHub.

Once a deploy without the binding has gone out, delete it: `npx wrangler kv namespace delete --namespace-id 9fa493aaa0a54d219b2dfebb5f681b9e`. Nothing in the build or a deploy waits on this.
