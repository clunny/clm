---
component: web.worker.review
base: { contract: "3b024b91d4ac24a5" }
---

## MODIFIED Contract

Crate `clm-web-review`: a spec branch's review and what is written on one, for `web.worker`'s router, which makes the session and access checks and passes the `Access` they give (`clm_web_platform::access`). Built for `wasm32-unknown-unknown` and natively for tests, with no `worker` dependency. It offers, each answering a `clm_web_platform::http::Response`:
- `branches::list`, the spec-branch list (`GET .../branches`);
- `review::branch`, a spec branch's review (`GET .../branches/{branch}/review`);
- `approve::branch`, approve (`POST .../branches/{branch}/approve`);
- `feedback::comment` and `feedback::request_changes` (`POST .../branches/{branch}/comments` and `.../request-changes`), the first taking the snapshot's requirement lookup from the router;
- the body parsers the router runs before any request to GitHub: `approve::fingerprint`, `feedback::parse_comment` and `feedback::parse_request`;
- `review::signed` and `review::signed_refusal`: whether the default branch's tip holds `docs/allowed_signers`, cached a day by tip, and the 422 a signed repository's write gets.

The routes and response shapes are `web.worker`'s contract. It imports nothing of `clm-web`.
