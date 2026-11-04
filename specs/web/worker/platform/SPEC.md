---
title: Worker platform
status: active
owners: ["@clunny"]
depends_on: [model, orchestrator, ledger, gate]
paths: ["crates/clm-web-platform/**"]
successor: ""
serves: [capability/follow-from-browser]
---

## Contract
Crate `clm-web-platform`, the layer `web.worker`'s routes stand on, under both `clm-web` and `clm-web-review`. Built for `wasm32-unknown-unknown` and natively for tests, with no `worker` dependency: `clm-web`'s Worker adapter (`worker.rs`) implements its traits. Its modules:
- `platform`: the `Platform` trait and its parts (assets, KV, the Cache API, GitHub, `Live`, logging, the clock, secrets, randomness) and `Auth`;
- `http`: `Request` and `Response`, its opaque upgrade variant included, and percent coding;
- `access`: `Access` and `check`, the read check every repo route makes, and `permission`, a login's permission on a repo asked with the installation's token (`GET /repos/{o}/{r}/collaborators/{login}/permission`, a 404 being `none`) and kept 5 minutes per repo and login;
- `gh`: the GitHub REST and GraphQL client;
- `files`: tree listings and batched GraphQL fetches of files and objects;
- `keys`: tree listings as `Listing`, `code_key` and `fingerprint`;
- `install`: installation IDs and tokens, `unix_time` and `rfc3339`;
- `live`: the live state and its 60-second cache, with `fresh` (`web.worker`'s **Live state**);
- `ledger`: the ledger branches read and folded (`web.worker`'s **Ledgers**), a branch's tip (`tip`), and the web-ledger append: `web_writer`, `record` (a record by `<login>/web`, now, with a random ID) and `append`, which commits a record as the last line of its writer's `records.jsonl` on `clm/ledger/<login>/web` with four Git Data API writes (blob, tree, commit, and the ref created or moved forward only) and nothing else;
- `fake`, behind the cargo feature `fake`: the fake GitHub and platform the Worker's tests drive, which `clm-web` and `clm-web-review` turn on in their dev-dependencies.

It answers no route of its own, and decides nothing a route answers.

## Design
**The fake** (`nested-splits`). The fake GitHub and platform live in `src/fake.rs` behind the `fake` feature. The crates only the fake uses (`futures`' executor and the like) are optional dependencies enabled by that feature alone, so the Worker's wasm build compiles none of them. The fake calls nothing of `clm-web` and writes no checkout. `src/fake.rs` counts toward this component's code: `check/node-size` reads the files under its `paths`, whatever their features.

**One way down.** Nothing here imports `clm-web` or `clm-web-review`: what a module needs from a route comes in as an argument. Code both halves need goes here; code only one needs stays in it.

**A fresh read replaces the plain entry** (`fresh-replaces-plain`). `live::cached` stores what it fetches for an `Access` carrying a `fresh` delivery under the delivery's key, `live:<o>/<r>#<d>:<what>`, and under the plain key `live:<o>/<r>:<what>` too, with two `cache_put` calls and no new `Platform` method. So the next read without `fresh` in that data centre (the 60-second refresh, a navigation, another tab) answers from the newer value and never goes back to the minute-old one, so an approval the app's reload showed doesn't vanish from a review's header. A value read from the delivery's key is stored nowhere else, since the fetch that put it there stored both. Two fetches that finish together may leave the older value under the plain key for up to 60 seconds, as two tabs fetching at once may; that is accepted. The Cache API is per data centre, so a reader in another data centre keeps its own minute-old copy until it expires or a notice's refresh replaces it; that is accepted too.

**Assets as responses** (`asset-responses`). `Assets` has one primitive, `fetch_asset(&self, req: &Request) -> Response`, which hands the incoming request to the `ASSETS` binding and returns its answer whole, so HEAD, `If-None-Match` (a 304) and `Range` (a 206) reach the binding and their answers reach the browser. `asset(path)` is a provided method over it, which `invited.rs` keeps calling unchanged: it fetches a `Request` with method `GET`, that `path`, and an empty query, headers, body and origin, and gives the body's text only for a 200 whose body is UTF-8. So the Worker adapter (`crates/clm-web/src/worker.rs`'s `Cloudflare`) and the fake each implement one method. Adding the required method breaks the adapter, which is `web.worker`'s code, so this component's build rewrites the adapter's `Assets` implementation as `fetch_asset` and names that fix in its pull request (`path-ownership`), as `web/app-at-app-path`'s build names the fixtures it moves. The adapter passes the request's method, path, query and headers to the binding, answers 503 without the binding and 502 when the fetch fails, converts the binding's answer into an `http::Response` (status, every header, the body's bytes), and keeps its per-isolate cache, now of whole responses, only for 200 answers to a `GET` of a `/_clm/*` path, which change only with a deploy. The fake's `assets` is a `RefCell<BTreeMap<String, (String, Vec<u8>)>>`, a content type and bytes per path, so binary fixtures (a font, an image) are allowed; it answers a path it holds with 200, a lower-case `content-type` header and the bytes, whatever the method (dropping a HEAD's body and answering conditional or range requests are the binding's, which the fake doesn't imitate), and a path it lacks with a bare 404 and no body, as the binding does under `"none"`. The platform answers no path specially. Changing the map's value type moves the fixtures of `clm-web`'s tests that insert into it (`tests/sign_in.rs`, `tests/push.rs`): this build makes those fixes and names them in its pull request (`path-ownership`), and leaves the whole workspace compiling and its tests passing, the `clm-web` wasm build included, since `cargo test -p clm-web-platform` alone doesn't build them.

**Reads without the cache** (`object-reads`). The `Replica` object (`web.worker.replica`) calls this crate's GitHub reads and the web-ledger append, but must never reach KV or the Cache API (`web.worker/cache-is-disposable`): it keeps what it read in its own memory and storage. So each function it calls has a form bounded by only the traits it uses (`GitHub`, `Clock`, `Secrets`, `Live`, `Log`), which reads GitHub each time it is called; the form the Worker calls, which keeps its answer in the Cache API as today, is a wrapper that adds `Cache` to the bounds and calls it. Today's set is `gh::get`, `gh::get_ok` and `gh::graphql`, `files::listing` and `files::objects`, `install::installation_id` and `install::token`, `access::check` and `access::permission`, and `ledger::own`, `ledger::tip`, `ledger::record` and `ledger::append`; a function the object comes to call joins it. `Platform` stays the bundle the Worker's routes take.

## Requirements

### Requirement: shared-layer
`clm-web-platform` SHALL hold the Worker's platform traits, HTTP types, access and permission checks, GitHub client, file fetches, tree keys, installation tokens, live state, ledger reading and web-ledger append, as the modules `platform`, `http`, `access`, `gh`, `files`, `keys`, `install`, `live` and `ledger`, and `clm-web` and `clm-web-review` SHALL use them from it, holding no module of those names of their own.

#### Scenario: one copy
- GIVEN the workspace
- WHEN a test in `clm-web-platform` lists `crates/clm-web/src`, `crates/clm-web-review/src` and `crates/clm-web-platform/src`, a directory that doesn't exist counting as holding nothing
- THEN `crates/clm-web-platform/src` holds `platform.rs`, `http.rs`, `access.rs`, `gh.rs`, `files.rs`, `keys.rs`, `install.rs`, `live.rs` and `ledger.rs`, and neither other directory holds a file of those names

#### Scenario: the append
- GIVEN the fake GitHub, a repo whose `clm/ledger/alice/web` holds one record, and an `Access` to it
- WHEN `ledger::append` commits a record by `alice/web`
- THEN the fake saw exactly four writes, a blob, a tree, a commit whose parent is the old tip, and a ref update with `force` false, and the branch's `records.jsonl` holds the old line then the record's

#### Serves
- architecture/reuse-first
- architecture/per-writer-ledgers

### Requirement: code-key-doc-only
`keys::code_key` SHALL leave out of a commit's code key exactly the entries whose path `clm_ledger::doc_only` (`ledger/doc-only`) answers true for, by calling it, and `crates/clm-web-platform/src/keys.rs` SHALL hold no list of doc-only paths of its own, so a change to `ledger`'s list reaches the Worker's code keys with no edit to this component.

#### Scenario: doc-only paths from the ledger
- GIVEN a tree listing of `blob` entries (any mode and sha) at `docs/x.md`, `docs/guide/intro.md`, `backlog/y.md`, `CLAUDE.md`, `web/README.md`, `crates/a/src/lib.rs` and `docsy/z.md`
- WHEN `keys::code_key` runs on it
- THEN it equals `keys::code_key` over only `docs/guide/intro.md`, `web/README.md`, `crates/a/src/lib.rs` and `docsy/z.md`, the entries `clm_ledger::doc_only` answers false for, and differs from the key of the listing with any one of those four removed

#### Scenario: no copy of the list
- GIVEN the text of `crates/clm-web-platform/src/keys.rs`, read by a test outside it with `include_str!`
- WHEN the tests run
- THEN the test fails, naming each it finds, when the text holds any of the string literals `"specs/"`, `"changes/"`, `"backlog/"`, `"docs/"`, `"docs/guide/"`, `"ARCHITECTURE.md"`, `"CAPABILITIES.md"`, `"README.md"` or `"CLAUDE.md"`

#### Serves
- architecture/reuse-first
- architecture/no-silent-drift

### Requirement: code-key-as-ledger
`keys::code_key` SHALL give, for a tree listing, the key `clm_ledger::code_key` gives for the same tree, leaving out exactly the paths `ledger`'s `doc_only` names: a tree that differs from another only in `docs/x.md` gets the same key, and one that differs only in `docs/guide/intro.md` or only in `docs/spec-format.md` gets a different key.

#### Serves
- architecture/reuse-first
- architecture/no-silent-drift

#### Needs
- ledger/doc-only

#### Scenario: the guide's files are code
- GIVEN a repository whose commits `a`, `b` changing only `docs/x.md`, `c` changing only `docs/guide/start.md` and `d` changing only `docs/spec-format.md`, each listed as the Trees API lists it
- WHEN `keys::code_key` runs on each listing
- THEN each key equals `clm_ledger::code_key` of the same commit, `a` and `b` share one, and `c` and `d` each differ from `a`'s

### Requirement: fresh-replaces-plain
When `live::cached` fetches a live-state item for an `Access` that carries a `fresh` delivery, it SHALL store the value under the plain key `live:<o>/<r>:<what>` as well as under the delivery's key, each for 60 seconds, so a later read of that item without `fresh` answers from it; a value it reads from the delivery's key SHALL be stored under no other key.

#### Serves
- capability/follow-from-browser

#### Scenario: the next plain read sees it
- GIVEN the fake platform, `o/a`'s ledgers stored under the plain key 10 seconds ago without `clm/ledger/alice/web`, and that branch created since with one record
- WHEN `ledger::fold` reads with an `Access` to `o/a` carrying `fresh` `d1`, and then with one carrying no `fresh`
- THEN both folds hold `alice/web`'s record, and the second read made no request to GitHub

#### Scenario: a cached fresh read
- GIVEN the fake platform and an item already stored under `fresh` `d1`'s key
- WHEN `live::cached` reads it again with `fresh` `d1`
- THEN it makes no request to GitHub and stores nothing

### Requirement: asset-responses
`platform::Assets` SHALL have the required method `fetch_asset(&self, req: &Request) -> Response`, which answers the request from the assets with their status, headers and bytes as they give them, and `asset(path) -> Option<String>` SHALL become a provided method over it that gives `Some(text)` only for a 200 whose body is UTF-8. The fake SHALL serve `fetch_asset` from its assets map, which holds a content type and bytes per path: a path the map holds answers 200 with that `Content-Type` and those bytes, and a path it lacks answers 404 with no body. The Worker adapter SHALL implement `fetch_asset` alone, passing the request's method, path, query and headers to the `ASSETS` binding, answering 503 without the binding and 502 when the fetch fails, and caching per isolate only `/_clm/*` paths.

#### Serves
- architecture/invite-only-web

#### Scenario: a binary asset
- GIVEN the fake platform whose assets map holds `/fonts/inter.woff2` as `font/woff2` with bytes that aren't UTF-8
- WHEN `fetch_asset` answers a GET of `/fonts/inter.woff2`
- THEN the response is 200 with `Content-Type` `font/woff2` and exactly those bytes, and `asset("/fonts/inter.woff2")` is `None`

#### Scenario: a missing path
- GIVEN the fake platform whose assets map lacks `/guide/nothing-here`
- WHEN `fetch_asset` answers a GET of `/guide/nothing-here`
- THEN the response is 404 with an empty body, and `asset("/guide/nothing-here")` is `None`

#### Scenario: text through the provided method
- GIVEN the fake platform with its default assets, `/_clm/invited.json` as `application/json` holding `["clunny"]`
- WHEN `asset("/_clm/invited.json")` runs
- THEN it is `Some` of `["clunny"]`

### Requirement: object-reads
Every function of `clm-web-platform` that `clm-web-replica` calls SHALL have a form whose bounds name none of `Kv`, `Cache` and `Assets` and that reads GitHub, or the clock and secrets, each time it is called, keeping nothing; where the Worker's form of the same function keeps its answer in the Cache API, it SHALL call that form rather than repeat it.

#### Scenario: a platform without storage
- GIVEN a test platform that implements only `GitHub`, `Clock`, `Secrets`, `Live` and `Log`, over the fake GitHub
- WHEN the uncached forms of `install::token`, `access::check`, `access::permission`, `files::listing`, `files::objects`, `ledger::own`, `ledger::tip` and `ledger::append` are each called twice with the same arguments
- THEN the test compiles, each call answers as the Worker's form does over the fake platform, and the fake GitHub saw each request twice

#### Scenario: the Worker's form still caches
- GIVEN the fake platform
- WHEN `install::token` is called twice for one installation
- THEN the fake GitHub saw one token request

#### Serves
- architecture/git-is-truth
- architecture/reuse-first
