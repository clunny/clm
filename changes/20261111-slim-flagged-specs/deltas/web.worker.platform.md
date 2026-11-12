---
component: web.worker.platform
base: { design: "e30a972e8c4dd93b" }
---

## MODIFIED Design
**The fake** (`nested-splits`). The fake GitHub and platform live in `src/fake.rs` behind the `fake` feature. The crates only the fake uses (`futures`' executor and the like) are optional dependencies enabled by that feature alone, so the Worker's wasm build compiles none of them. The fake calls nothing of `clm-web` and writes no checkout. `src/fake.rs` counts toward this component's code: `check/node-size` reads the files under its `paths`, whatever their features.

**One way down.** Imports run one way, as `web.worker`'s Design says: what a module here needs from a route comes in as an argument. Code both halves need goes here; code only one needs stays in it.

**Caches and endpoints.** `access::permission` asks `GET /repos/{o}/{r}/collaborators/{login}/permission` with the installation's token, a 404 being `none`, and keeps the answer 5 minutes per repo and login. `live` keeps the live state 60 seconds.

**A fresh read replaces the plain entry** (`fresh-replaces-plain`). Storing what a `fresh` read fetches under the plain key too, with two `cache_put` calls and no new `Platform` method, means the next read without `fresh` in that data centre (the 60-second refresh, a navigation, another tab) never goes back to the minute-old value, so an approval the app's reload showed doesn't vanish from a review's header. Two races are accepted: two fetches that finish together may leave the older value under the plain key for up to 60 seconds, as two tabs fetching at once may; and the Cache API is per data centre, so a reader in another data centre keeps its own minute-old copy until it expires or a notice's refresh replaces it.

**Assets as responses** (`asset-responses`). `fetch_asset` takes the whole request and returns the whole answer so that HEAD, `If-None-Match` (a 304) and `Range` (a 206) reach the `ASSETS` binding and their answers reach the browser. The fake answers a path it holds the same whatever the method, since dropping a HEAD's body and answering conditional or range requests are the binding's. The platform answers no path specially.

**Reads without the cache** (`object-reads`). The `Replica` object (`web.worker.replica`) must never reach KV or the Cache API (`web.worker/cache-is-disposable`), hence the uncached forms.
