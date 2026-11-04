# CLM web app

The `web` node (`specs/web/SPEC.md`): a Cloudflare Worker at `https://clm-app.dev` that shows
the spec graph, spec branches, implementation progress and the backlog of the CLM repos a
signed-in person can read, and approves spec branches by appending records to that
person's web ledger.

- `crates/clm-web`: the Worker, in Rust (`workers-rs`), built for `wasm32-unknown-unknown`.
- `web/`: the browser app (TypeScript, Vite), `wrangler.toml`, and this file.

## Develop

`web/.npmrc` sets `ignore-scripts=true`, so no dependency's install script runs. Add
dependencies with `npm install --ignore-scripts <package>` and commit `package-lock.json`.

```sh
cd web
npm ci                 # install exactly what the lockfile says
npm run dev            # the app on http://localhost:5173 (no Worker behind /api)
npm run test:unit      # Vitest + jsdom
npm test               # unit tests, then the Playwright tests against `vite preview`
npm run build:worker   # the Rust Worker, with worker-build pinned to the `worker` crate
npm run deploy:check   # `wrangler deploy --dry-run`: builds both and checks the config
```

The Playwright tests need its Chromium and the system libraries it uses. Install them once
per machine (it needs sudo for the libraries; CI does this itself):

```sh
cd web && npm ci && npx playwright install --with-deps chromium
```

`cargo test -p clm-web` tests the Worker against fakes of GitHub and Cloudflare.

## Deploy

The release workflow (`.github/workflows/release.yml`, job `web`) runs `npm ci
--ignore-scripts` and `npx wrangler deploy` in `web/` on each release, with the
`CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` repository secrets. `wrangler.toml`'s build
command builds the app into `web/dist` and the Worker into `crates/clm-web/build`.

## What the setup made

Done once on 2026-10-22. Nothing here needs doing again unless something is lost.

- **Domain.** `clm-app.dev`, registered through Cloudflare Registrar on the operator's
  account. `wrangler.toml` makes it the Worker's custom domain, so Cloudflare creates the DNS
  record and certificate on deploy.
- **Worker.** `clm`, holding the secrets below.
- **KV namespace.** `clm-cache`, id `9fa493aaa0a54d219b2dfebb5f681b9e`, bound as `CLM`. To
  create one again: `npx wrangler kv namespace create clm-cache`, then put its id in
  `wrangler.toml`. Everything in it is a cache derived from GitHub and can be deleted.
- **GitHub App.** `clm-spec-review` ("CLM Spec Review", App ID 5433936), owned by `clunny`.
  People install it from https://github.com/apps/clm-spec-review/installations/new. To
  create it again, post this manifest through GitHub's
  [manifest flow](https://docs.github.com/en/apps/sharing-github-apps/registering-a-github-app-from-a-manifest)
  and exchange the code with `POST /app-manifests/{code}/conversions`:

  ```json
  {
    "name": "CLM Spec Review",
    "url": "https://clm-app.dev",
    "hook_attributes": { "url": "https://clm-app.dev/webhook", "active": true },
    "callback_urls": ["https://clm-app.dev/auth/callback"],
    "public": true,
    "default_permissions": { "contents": "write", "pull_requests": "read", "metadata": "read" },
    "default_events": ["push", "pull_request"]
  }
  ```

- **Worker secrets.** Set with `npx wrangler secret put <NAME>` in `web/` (or all at once
  with `npx wrangler secret bulk`), never printed or written to disk:
  - `GITHUB_APP_ID`: the App ID.
  - `GITHUB_APP_PRIVATE_KEY`: the App's private key as PKCS#8 PEM, which WebCrypto can
    import. GitHub gives PKCS#1; convert it with
    `openssl pkcs8 -topk8 -nocrypt -in app.pem | npx wrangler secret put GITHUB_APP_PRIVATE_KEY`.
  - `GITHUB_CLIENT_ID` and `GITHUB_CLIENT_SECRET`: the App's OAuth credentials.
  - `GITHUB_WEBHOOK_SECRET`: the App's webhook secret.
  - `COOKIE_KEY`: 32 random bytes, base64, which encrypt the session cookie:
    `openssl rand -base64 32 | npx wrangler secret put COOKIE_KEY`. Changing it signs
    everyone out.
- **Repository secrets** for the release workflow:
  `gh secret set CLOUDFLARE_API_TOKEN` (a token from the "Edit Cloudflare Workers" template,
  limited to the operator's account and the `clm-app.dev` zone) and
  `gh secret set CLOUDFLARE_ACCOUNT_ID`.

## Rotate a credential

Generate a new private key or client secret in the App's settings
(https://github.com/settings/apps/clm-spec-review), `npx wrangler secret put` it as above,
then delete the old one in the App's settings. A new webhook secret goes into the App's
settings and `GITHUB_WEBHOOK_SECRET` together.
