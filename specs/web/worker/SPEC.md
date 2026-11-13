---
title: Worker
status: active
owners: ["@clunny"]
depends_on: [model, check, gate, context, orchestrator, ledger, view, config, web.live, effort, replica, web.worker.platform, web.worker.review, web.worker.replica]
paths: ["crates/clm-web/**"]
successor: ""
size_budget: { spec_tokens: 16000, code_tokens: 72000 }
serves: [capability/follow-from-browser]
---

## Contract
The Cloudflare Worker behind `web`: it is deployed with the browser app's static assets under `/app/`, on the app's routes of `clm-app.dev`, while the public site (`web.site`) is a Worker of its own at the root (`web/site-served-directly`, `site-worker-of-its-own`), and answers what `web/wrangler.toml`'s `run_worker_first` routes to it: `/app`, which it redirects to `/app/` (`reserved-paths`), `/auth/*` (GitHub sign-in, which returns to `/app/`), `/api/*` (JSON for the app: the user's repos, a repo's snapshot, its live state, a spec branch's review, approve, comments and change requests on a spec branch, proposals for the backlog, what each capability and each architectural constraint costs and a decision on one, the live socket and push subscriptions), `/webhook`, and `/_clm/*`, which it answers 404 so the invited list is never served raw (`reserved-paths`). The app calls nothing else, and every response shape it reads is this component's contract: changing one is a change here that `web` builds against. The Worker alone holds a user's token, passing it to the repo's `Replica` object only for a decision's, a proposal's or a withdrawal's append, which keeps it only for that request; it writes to the repository only to append an `approved`, `subscribed`, `unsubscribed`, `proposed`, `withdrawn`, `commented`, `changes_requested` or `decided` record to the user's web ledger (`per-writer-ledgers`, `push-subscriptions-in-ledger`, `web-proposals-as-ledger-records`, `web-review-feedback-as-ledger-records`), and keeps only state that can be deleted at any time (`web-derived-cache`): caches, each repo's replica, which its `Replica` object (`web.worker.replica`) holds, derived from git and rebuilt from it when lost (`web-repo-replica`), and the open pages' sockets, which its `Live` objects (`web.live`) hold and which store nothing (`web-live-channel`). Its webhook passes each repo's events to the repo's `Replica` object, which recomputes the views, tells the `Live` object, and sends Web Push notifications to the subscriptions in the repo's ledgers. The views are computed by `replica` with the `model`, `check`, `orchestrator`, `view`, `ledger` and `effort` crates compiled to wasm, so the Worker shows what `clm` would report and decides nothing.

Three children answer parts of it beneath this component's router, session and access check: `web.worker.review` the spec-branch list, a spec branch's review, approve, comments and change requests; `web.worker.replica` the views and the writes that need them; and `web.worker.platform` the layer every route stands on. None changes a route or a response shape: what the app reads stays this component's contract.

## Design
Crate `crates/clm-web`, in Rust with `workers-rs`, built for `wasm32-unknown-unknown` (`web-reuses-rust-in-wasm`). Worker-only code sits behind `cfg(target_arch = "wasm32")`, so `cargo test` covers the rest on the host. It is deployed with the app by `web`'s `wrangler.toml` (`ci`'s release workflow). The invited-users list it enforces is `web`'s (`web/invited-users`): `web`'s build writes it from `specs/web/SPEC.md` to `_clm/invited.json`, and the Worker reads that asset. Three children's crates sit beneath it (`nested-splits`): `clm-web-platform` (`web.worker.platform`), the layer every route stands on; `clm-web-review` (`web.worker.review`), a spec branch's review, its approve, comments and change requests, and the spec-branch list; and `clm-web-replica` (`web.worker.replica`), the repo's `Replica` object, which computes the views. This crate keeps the router (`lib.rs`), the session, the view routes and the writes that go through the object, and calls them; imports run one way, from here to the three, and from the review and replica crates to the platform crate.

**Budget** (`web-on-free-plan`, `web-repo-replica`). It runs on Cloudflare's Free plan: at most 10 ms of CPU and 50 subrequests per Worker request. So a Worker request computes no view: it checks the session and access, then answers a view the repo's `Replica` object (`web.worker.replica`) computed, or passes a write on to it. The object does the work, within a Durable Object's 30 s of CPU per request or alarm and the account's daily Durable Object allowance. KV holds nothing the views read; the Cache API holds the memoised repo lists and access checks, and the views for 30 seconds (below).

**Sign-in** (`web-github-app`). One public GitHub App, owned by the operator, that any org or user can install on chosen repos. People sign in through the App's user authorization flow. The user-to-server token and its refresh token live only in an AES-GCM encrypted, `HttpOnly`, `Secure`, `SameSite=Lax` cookie keyed by a Worker secret. There is no server-side session store. The App asks for repository `contents: write` (the ledger append), `pull_requests: read` (a branch's pull request: a user's token reads a private repo's pull requests only when the App may), `metadata: read`, and subscribes to `push`.

**Repos and isolation.** The repo list is every repo of the user's App installations (`GET /user/installations/{id}/repositories`) whose default branch has `specs/SPEC.md`, memoised for 5 minutes per token hash in the Cache API, so navigating doesn't repeat its GitHub requests; a newly installed repo can take that long to appear. An empty list isn't memoised, so someone who has just installed the App sees their repo at once. Repos from many organisations share one cache, so isolation is the main security boundary. Every API request for a repo first checks, with the user's own token, that they can read it (`GET /repos/{o}/{r}`, memoised for 5 minutes per token hash in the Cache API), and a user who can't read it gets 404 even when its data is cached. Cache keys always include `<owner>/<repo>`, and no response mixes repos.

**Views** (`views-from-replica`). `/snapshot`, `/implementation`, `/capabilities` and `/architecture` answer, after the access check, the view of the same name that the repo's `Replica` object answers (`GET /view/<name>`), reached through a `Replicas` platform trait, which tests fake. Their shapes are this component's contract, and `replica` computes them: the snapshot (`sha`, `coverage`, `sizes`, the `spec` model, components, changes, backlog, `resolved`, decisions, violations), the implementation response (`sha`, `now`, `expiry_hours`, `coverage`, `flight`, `blockers`, `pages`, `activity`, `taken` and `proposals`, to each of which the Worker adds `mine`), the capabilities report (`report`, `signed`) and the architecture report (`architecture`, `signed`, `indexing`). The Worker keeps each answer in the Cache API for 30 seconds under `view:<o>/<r>:<name>`, so an open tab's polls rarely reach the object; a request with a valid `fresh` skips that cache and asks the object with `fresh=1`, which brings itself up to the tip first, and stores what it answers. While the object is still preparing the repo, the route answers its 503 `{"preparing": <share>}` as given, uncached. When the object can't be reached, the route answers 503 `{"error": "unavailable"}`. The implementation's `proposals` are the view's own (`replica/open-proposals`): the Worker adds `mine` for the requester to the view it holds or was just given, so one cached view serves every reader, and reads no ledger and no other view for them. A view the object stored before it carried `proposals` answers `[]` until the object's next recompute. The views are up to a minute behind the latest push, which the object batches (`web-repo-replica`).

**Webhook.** `/webhook` verifies the signature, then passes each event that names a repository, `push`, `pull_request`, `installation` and `installation_repositories`, with its delivery and installation, to the named repos' `Replica` objects (`POST /event`), and answers. It computes nothing and writes no KV; the objects fetch, apply, recompute, notify the open pages and send notifications (`web.worker.replica`). A failed post is logged and doesn't change the webhook's answer.

**Spec branches and reviews.** `web.worker.review` answers the spec-branch list, a branch's review and its writes from `web.worker.platform`'s live state, kept in the Cache API for 60 seconds, and `fresh` works for them.

**Writes through the object** (`writes-to-object`). A proposal, a withdrawal and a decision pass through the Worker in one post to the repo's `Replica` object, `POST /write`, reached through the `Replicas` trait the views use, so each costs the Worker its access check and that post and reads no view, ledger or spec file. The body is the requester's login and token, which the object keeps only for the request, with `"kind"` and the write's fields: `"propose"` with `title`, `body` and `nodes` as the body check left them; `"withdraw"` with `id`; `"decide"` with `id` (`capability/<slug>` or `architecture/<slug>`, the slug percent-decoded), `version`, `value` and `decision`. The route answers the object's status and body as it gave them, so its refusals, its 503 `{"preparing": <share>}` and its 502 reach the app unchanged, and answers 503 `{"error": "unavailable"}` when the object can't be reached, as the view routes do. Nothing else is told of the write: GitHub's `push` webhook for the append reaches an object that has already applied it. Since these routes' scenarios name what is written and answered, `clm-web`'s tests drive them through a real `clm_web_replica::Object` over the fake GitHub, with `clm-web-replica` and `clm-replica` (its `Store`) as dev-dependencies, and only `writes-to-object`'s own scenarios script the object.

**Proposals** (`web-proposals-as-ledger-records`, `proposals`). `POST /api/repos/{o}/{r}/proposals` takes an object with `/title`, `/body` and `/nodes`: `title` a string of 1 to 120 characters once trimmed, without a line break; `body` a string of at most 8,000 characters; `/nodes` a list of at most 10 component IDs the replica's graph holds. The title and body are stored trimmed; the body may be empty; the components are deduplicated and may be empty. The slug is the title in kebab case as `clm defect` slugs one (`plugin.hooks/defect-backlog`), in `replica`'s `validate`, since `clm-hooks` isn't built for wasm: lowercased, split on anything but ASCII letters and digits, the words joined with `-` while the result stays within 64 bytes (a first word longer than that cut to 64), with no fallback name. The Worker computes no slug: its body check refuses, as 422 `slug`, a title that lowercases to no ASCII letter or digit, the one whose slug would be empty. The body is checked first, before any request to GitHub, as Approve's is; then the access check; then the request goes to the repo's `Replica` object (`POST /write`, `writes-through-object`), which checks the slug against its state brought up to the tip and appends. The 422 and 409 answers are `proposals`'s: a taken slug is one the backlog has, `resolved` holds or an open proposal the implementation response would list has (an unwithdrawn `proposed` record whose slug is neither filed nor resolved), so the person changes the title. A slug `resolved` holds is spent for good, since `clm rebase` never files it, and an unwithdrawn record whose item was filed or resolved takes its slug only through the backlog or `resolved`, never as a proposal of its own. A valid proposal appends the `proposed` record, with `slug`, `title`, `body` and `/nodes`, with the four writes Approve makes, answering 200 with `{"id"}`. `DELETE /api/repos/{o}/{r}/proposals/{id}` goes to the object in the same way, which decides from the requester's own `clm/ledger/<login>/web` read at its tip, as the push routes do, never from the minute-old fold, so it withdraws only what that ledger holds: other people's proposals and one already withdrawn answer 404. The implementation response's `proposals` are the implementation view's, which leaves out those `clm rebase` files, by the rule it files by (`replica/open-proposals`), each with `mine` added, whether the writer is `<login>/web`, the requester's. The object applies the record and recomputes before it answers, so the app's reload after the write (`fresh`) shows it. The Worker files nothing: an item reaches `main` through `clm rebase` on the next `impl/*` branch and that branch's land.

**Decide** (`decide`, `architecture-decide`, decision `effort-records`). The body is checked before any request to GitHub; then the access check; then the request goes to the repo's `Replica` object (`POST /write`), which brings itself up to the tip, so the version the person saw is compared with the entry's version at the tip now, as approve compares the fingerprint, and answers `decide`'s refusals from `replica/validate-writes`. A valid decision appends through `clm-web-platform`'s `ledger::append` with the requester's token, settling the flags `clm_effort::unsettled` names over the ledgers the object holds, which include every append it has seen. The `signed` refusal exists because a signed repository counts only records signed with a key the Worker doesn't hold (`signed-repository`). The capabilities and architecture views have no history: commits, the backlog items' lives and the sizes need git, so those numbers are absent and the app says to run `clm capabilities <slug>` for them.

**Live pushes** (`web-live-channel`). `GET /api/repos/{o}/{r}/live`, with or without `Upgrade: websocket`, first answers 403 unless the request's `Origin` header equals the app's own origin exactly (`Request::origin`), as the routes that write do, in `api`'s same check before the session's, matched on the route's parts; a missing `Origin` is refused: a WebSocket handshake isn't bound by CORS and browsers always send `Origin` on one, so this, not `SameSite=Lax`, keeps another site from opening the socket with the person's cookie. It then makes the session and access checks every API route makes (401 without a valid session, 404 without access), then forwards the request through the `LIVE` Durable Object binding to the repo's `Live` object (`web.live`), `id_from_name("<o>/<r>")`, and returns its 101, with a renewed session cookie when the route would add one; without the upgrade header it answers 426, and without the binding 503. The core reaches the object through a `Live` platform trait (forward a request, post a notice, post a send), which tests fake, and `http::Response` gains an opaque upgrade variant that only `worker.rs`'s adapter fills in with the object's socket. The notices themselves come from the repo's `Replica` object after each recompute (`web.worker.replica/alarm-batches`), so a page refreshes once the views it will read have changed. A ledger append is a push, so an approval, a check or a land made anywhere reaches the open pages.

A refresh a notice prompts must not be answered from a cache older than the notice. The view routes take `?fresh=<delivery>` as **Views** says. The routes `web.worker.review` and `web.worker.platform` answer from the live state, the spec-branch list, a spec branch's review and `GET .../push`, take it: when the value is 1 to 64 characters of `[0-9a-f-]`, every live-state item they read is cached under a key that adds it (`live:<o>/<r>#<delivery>:<what>`), carried to `live::cached` with the request's `Access`, so the first request after a notice fetches it from GitHub again and later requests for that notice share it for 60 seconds; any other value is ignored. Two tabs in one data centre that ask at the same instant may both fetch, which is accepted. `fresh` lets an invited person make the Worker and the object fetch more than once a minute; only invited people can sign in (`invite-only-web`).

**Notifications** (`push-subscriptions-in-ledger`). `GET /api/push/key` answers `{"key": <VAPID_PUBLIC_KEY>}`, or 404 when that secret isn't set, which the app reads as notifications being unavailable. A person's *web subscriptions* are the `subscriptions` of `clm_ledger::fold_records` run over their own web ledger branch alone, `clm/ledger/<login>/web` (`web-subscriptions-by-branch`): the branch names the person, the record's `writer` is not consulted, and only that branch's `unsubscribed` records end them, so a record someone commits to a branch of their own neither counts as that person's nor ends theirs. The Worker's web ledger branches are those named exactly `clm/ledger/<login>/web` with `<login>` a name `access::valid_name` accepts (the owner-name rule; a name it admits that no login has is asked once and refused by the permission check); a branch named otherwise under `clm/ledger/` holds nobody's web subscriptions. The ledger reader keeps the per-branch sources it fetches (`ledger.rs`'s `Sources`, cached with the live state), and the switch's `GET` and the `Replica` object's notifications fold the web branches from them one at a time, so nothing is fetched twice; `GET /api/repos/{o}/{r}/push?endpoint=<url>` answers `{"subscribed": <bool>}`: whether the user's web subscriptions, folded from their own branch's `records.jsonl` among the live state's cached ledger sources, hold one for the endpoint. `POST /api/repos/{o}/{r}/push` takes the browser's `PushSubscription.toJSON()`, `{"endpoint", "keys": {"p256dh", "auth"}}`, and gets 422 unless the endpoint is an `https` URL on a known push service (host `fcm.googleapis.com`, or ending in `.push.services.mozilla.com`, `.push.apple.com` or `.notify.windows.com`), `p256dh` decodes to a 65-byte uncompressed point and `auth` to 16 bytes, so the Worker never sends to an address an invited person made up. `POST` and `DELETE` decide what to write from the user's own `clm/ledger/<login>/web` at its tip, which they read as Approve does, not from the minute-old fold, so a quick off after an on is never lost. `POST` answers 200 without writing when that ledger already holds a live subscription for the endpoint, and otherwise appends a `subscribed` record with `endpoint`, `p256dh` and `auth` with the four writes Approve makes. `DELETE` takes `{"endpoint"}` and appends one `unsubscribed` record, naming the `subscribed` record's `id`, for each live subscription in that ledger for the endpoint, writing nothing when there is none. Each append is a push, so its notice brings the app's view of the switch up to date.

The notifications a push or a pull request calls for (`waiting_notice`, `review_notice`) are sent by the repo's `Replica` object after it recomputes (`web.worker.replica/push-waiting`, `push-review`), with the recipients, permission checks and limits they had when the webhook sent them.

The Worker reads its secrets as `GITHUB_APP_ID`, `GITHUB_APP_PRIVATE_KEY` (a PKCS#8 PEM, which WebCrypto can import), `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`, `GITHUB_WEBHOOK_SECRET` and `COOKIE_KEY` (32 random bytes, base64), and, when the operator has set them, `VAPID_PUBLIC_KEY` and `VAPID_PRIVATE_KEY` (`web.live`'s Design); without them it sends no notifications. CI deploys on release (`ci`).

**GitHub App.** The App is `clm-spec-review` ("CLM Spec Review", App ID 5433936), owned by `clunny`. The app links to `https://github.com/apps/clm-spec-review/installations/new` so people can install it. Its permissions are `contents: write`, `pull_requests: read` and `metadata: read`, and it subscribes to the `push` and `pull_request` events. Only the operator changes them, in the App's settings, and every installation must then accept the change. The manifest to create it from, if it ever has to be created again:

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

**Moving the routes.** The view routes and the webhook move to the `Replica` object only once it is built: `replica-class-export` needs `web.worker.replica/object-class`, which defines the class, and every requirement that moves a route or the webhook onto the object (`views-from-replica`, `capabilities`, `architecture`, `fresh-live-state`, `repo-access`, `webhook-refresh`) needs the object's `object-class`, `applies-events`, `first-build` and `alarm-batches` (`needs-alone-order`). Within this component the build orders them itself: `replica-class-export` and `views-from-replica` first, as the others answer through them. The capability tests under `capabilities/web/` drive the Worker as deployed, so the build that moves the routes also adds the `REPLICA` binding (`className: 'Replica'`, on SQLite) to their Miniflare harness and the GitHub calls the object makes to their fake, reading past `preparing` as the app does: test-fixture fixes the removal of the Worker's own views forces, named in its pull request (`path-ownership`).

**Assets adapter** (`asset-adapter`). The session cookie and the user's credentials stay with the Worker, so they never reach the binding. A `/_clm/*` answer changes only with a deploy, which starts new isolates, so the adapter keeps it for the isolate's life; the site's answers change without one and are never kept. `worker.rs` is built only for `wasm32`, so what the adapter decides about an asset request (what reaches the binding, the 503 and 502 answers, what it keeps) is code built for the host too, over a binding that `worker.rs` supplies from `ASSETS` and tests fake; `worker.rs` keeps only the binding's calls.

**Size** (`slim-components`). This component's budget is 72,000 code tokens and 16,000 spec tokens, above the defaults, because what stays is the Worker's one shell: the router every route passes through, the session and sign-in, the view routes and their cache, proposals, decide, the webhook's dispatch and the live socket, and the Worker adapter, with the tests that drive them through `handle`.

## Requirements

### Requirement: sign-in
The Worker SHALL sign people in through the GitHub App's user authorization flow, and SHALL keep their tokens only in an encrypted `HttpOnly` cookie, never in KV, the Cache API or any other store.

#### Scenario: callback
- GIVEN a GitHub that completes the authorization with a code
- WHEN the browser reaches the callback
- THEN the response sets an `HttpOnly` cookie that doesn't contain the token in clear, and nothing written to KV or the Cache API contains the token

#### Scenario: tampered cookie
- GIVEN a session cookie with one byte changed
- WHEN any API request carries it
- THEN the Worker answers 401

### Requirement: repo-access
The Worker SHALL answer a request for a repo's data only after confirming, with the requester's own token, that they can read the repo, and SHALL answer 404 otherwise, whether or not the data is cached or held by the repo's `Replica` object.

#### Needs
- web.worker.replica/object-class
- web.worker.replica/applies-events
- web.worker.replica/first-build
- web.worker.replica/alarm-batches

#### Scenario: held repo, no access
- GIVEN `o/private`'s views in its `Replica` object and in the Cache API, and a user whose token gets 404 from `GET /repos/o/private`
- WHEN that user requests `o/private`'s snapshot, review or implementation state
- THEN the Worker answers 404 each time, no body contains anything from `o/private`, and the object was asked nothing

#### Scenario: another organisation
- GIVEN views of `a/x` and `b/y` held, and a user who can read only `a/x`
- WHEN the user requests `b/y` by name
- THEN the Worker answers 404

### Requirement: repo-list
The Worker SHALL list the repos of the user's App installations whose default branch contains `specs/SPEC.md`, and no others, and SHALL answer a user's further listings for 5 minutes from the Cache API, keyed by a hash of their token, without asking GitHub, unless the list was empty.

#### Scenario: non-CLM repo
- GIVEN installations covering `o/a`, which has `specs/SPEC.md`, and `o/b`, which doesn't
- WHEN the user lists repos
- THEN the list is `o/a`

#### Scenario: listed again
- GIVEN a user who listed repos a minute ago
- WHEN they list repos again
- THEN the list is the same and the Worker makes no GitHub request

#### Scenario: another user
- GIVEN user A listed repos a minute ago
- WHEN user B, with a different token, lists repos
- THEN B's list comes from GitHub with B's token, not from A's

#### Scenario: empty list
- GIVEN a user whose listing a minute ago was empty, and who has since installed the App on a CLM repo
- WHEN they list repos again
- THEN the list comes from GitHub and holds the repo

### Requirement: webhook-refresh
The Worker SHALL reject webhooks whose signature doesn't verify, and SHALL pass each verified `push`, `pull_request`, `installation` and `installation_repositories` event to the `Replica` object of each repo it names, computing no view and writing no KV.

#### Needs
- web.worker.replica/object-class
- web.worker.replica/applies-events
- web.worker.replica/first-build
- web.worker.replica/alarm-batches
- web.worker.replica/push-waiting
- web.worker.replica/push-review
- web.worker.replica/uninstall-deletes

#### Scenario: push to main
- GIVEN a `Replica` platform the test fakes
- WHEN a verified push webhook with delivery `d1` moves `o/a`'s `main` to `222`
- THEN the object named `o/a` received the event with delivery `d1` and the installation's id, and the Worker wrote no KV value

#### Scenario: forged webhook
- GIVEN a push webhook with a wrong signature
- WHEN it reaches the Worker
- THEN the Worker answers 401 and no object received anything

### Requirement: cache-is-disposable
Every view SHALL show the same data after KV, the Cache API and every `Replica` object's storage are emptied, once rebuilt from GitHub, and nothing outside this component and its children `web.worker.platform`, `web.worker.review` and `web.worker.replica`, which reach them only through `clm-web-platform`'s `Platform` traits or the object's own storage, SHALL read them. Of the workspace's crates, only `clm-web`, `web.live`'s `clm-web-live` and `clm-web-replica` SHALL depend on `worker`, and only `clm-web` SHALL use its KV and Cache API bindings: `clm-web-live` holds the `Live` object and `clm-web-replica` the `Replica` object, which need `worker`'s Durable Object types and nothing of the caches.

#### Scenario: emptied cache
- GIVEN a repo whose snapshot was served once with a complete index
- WHEN KV, the Cache API and its `Replica` object's storage are emptied and the snapshot is requested until its index completes
- THEN the response equals the first one, but for `computed_at`

#### Scenario: the worker crate's dependants
- GIVEN the workspace's crate manifests and the sources of `clm-web-live` and `clm-web-replica`
- WHEN each `Cargo.toml` is read for a `worker` dependency and those sources for `kv`, `KvStore` or `Cache`
- THEN no crate but `clm-web`, `clm-web-live` and `clm-web-replica` depends on `worker`, and none of those sources names the caches

#### Serves
- architecture/git-is-truth
- architecture/web-optional

### Requirement: pr-routes-removed
The Worker SHALL answer 404 to `GET /api/repos/{o}/{r}/pulls`, `GET /api/repos/{o}/{r}/pulls/{n}/review` and `POST /api/repos/{o}/{r}/pulls/{n}/approve`, making no write to the repository for the last.

#### Serves
- architecture/no-dead-code
- architecture/spec-is-current

#### Needs
- web.views/spec-branch-review
- web/approve-by-branch

#### Scenario: old routes
- GIVEN a signed-in user who can read `o/a`, and an open pull request #7 whose head is the spec branch `spec/x`
- WHEN they request `o/a`'s `pulls`, PR 7's review, and then post PR 7's approve
- THEN each answer is 404, and no ledger ref or other ref of `o/a` changes

### Requirement: live-socket
`GET /api/repos/{o}/{r}/live` SHALL answer 403, before any other check, unless the request's `Origin` header equals the app's own origin. With that origin and `Upgrade: websocket` it SHALL answer 401 without a valid session and 404 when the user can't read the repo, as every API route does, and otherwise SHALL forward the request to the `Live` object named `<o>/<r>` and return its answer. Without the upgrade header it SHALL answer 426, and without the `LIVE` binding 503.

#### Scenario: forwarded
- GIVEN a signed-in user who can read `o/a`, and a `Live` platform the test fakes
- WHEN they request `/api/repos/o/a/live` with `Upgrade: websocket`
- THEN the fake received the request for the object named `o/a`, and the user got its 101

#### Scenario: another origin
- GIVEN a signed-in user who can read `o/a`, and a `Live` platform the test fakes
- WHEN they request `/api/repos/o/a/live` with `Upgrade: websocket` and `Origin: https://evil.example`, and then with no `Origin` header
- THEN both answers are 403 and the fake received nothing

#### Scenario: no access
- GIVEN a signed-in user whose token gets 404 from `GET /repos/o/private`
- WHEN they request `/api/repos/o/private/live` with `Upgrade: websocket`
- THEN the answer is 404 and the fake received nothing

#### Needs
- web.live/hub

### Requirement: fresh-live-state
The snapshot, implementation, capabilities and architecture routes, given `?fresh=<d>` where `<d>` is 1 to 64 characters of `[0-9a-f-]`, SHALL skip the Worker's cache of the view and ask the repo's `Replica` object with `fresh=1`. The spec-branch list, spec branch review and push state routes, given such a `fresh`, SHALL answer from live state, the ledgers included, fetched anew from GitHub the first time `<d>` is asked for, and from the Cache API for 60 seconds after. All SHALL ignore any other `fresh` value. The value is a notice's delivery or one the app made for the reload after its own write (`web/fresh-after-write`); the Worker treats both alike.

#### Needs
- web.worker.replica/object-class
- web.worker.replica/applies-events
- web.worker.replica/first-build
- web.worker.replica/alarm-batches

#### Scenario: a notice's refresh
- GIVEN `o/a`'s implementation view cached by the Worker 10 seconds ago, and a `Replica` platform the test fakes that now answers a newer view
- WHEN the implementation is requested with `?fresh=d1`, then without `fresh`
- THEN both answers are the newer view, and the object was asked once, with `fresh=1`

#### Scenario: an approval just made
- GIVEN `o/a`'s ledgers cached 10 seconds ago, and an `approved` record for `spec/x`'s fingerprint appended to `clm/ledger/alice/web` since
- WHEN `spec/x`'s review is requested with `?fresh=7c9e6679-7425-40de-944b-e07fc1f90ae7`
- THEN the answer lists the approval by `alice/web`

### Requirement: push-subscriptions
`GET /api/push/key` SHALL answer the `VAPID_PUBLIC_KEY` secret, or 404 when it isn't set. `POST /api/repos/{o}/{r}/push` with a body `{"endpoint", "keys": {"p256dh", "auth"}}` SHALL append a `subscribed` record with those three to the user's web ledger, unless that ledger, read at its tip, already holds a live subscription for the endpoint, and SHALL answer 422, writing nothing, unless the endpoint is an `https` URL on a known push service, `p256dh` a 65-byte uncompressed point and `auth` 16 bytes. `DELETE` on that route with `{"endpoint"}` SHALL append an `unsubscribed` record for each live subscription in the user's web ledger, read at its tip, for that endpoint. `GET` on it with `?endpoint=` SHALL answer whether the user's web subscriptions, the live subscriptions of their own branch `clm/ledger/<login>/web` folded alone (`web-subscriptions-by-branch`), hold one for the endpoint, whatever the records of other branches say. Each SHALL make the access check every API route makes.

#### Scenario: subscribe once
- GIVEN a signed-in user `alice` who can read `o/a`, and no subscription in `clm/ledger/alice/web`
- WHEN she posts a subscription for `https://fcm.googleapis.com/fcm/send/1` twice within a second
- THEN `clm/ledger/alice/web` holds exactly one `subscribed` record, for that endpoint

#### Scenario: off right after on
- GIVEN `alice` posted a subscription for that endpoint a second ago, as record `s1`
- WHEN she deletes that endpoint's subscription
- THEN her web ledger gains an `unsubscribed` record naming `s1`

#### Scenario: not a subscription
- GIVEN a signed-in user who can read `o/a`
- WHEN they post a subscription whose endpoint is `https://example.com/s/1`, and one on `fcm.googleapis.com` whose `auth` decodes to 15 bytes
- THEN both answers are 422 and nothing is written

#### Scenario: ended from another branch
- GIVEN `clm/ledger/alice/web` holding `subscribed` `s1` for an endpoint, and `clm/ledger/mallory/web` holding `unsubscribed` naming `s1`
- WHEN `alice` asks `GET .../push?endpoint=` for that endpoint
- THEN the answer is `{"subscribed": true}`

#### Needs
- ledger/subscription-records

### Requirement: live-class-export
`clm-web` SHALL depend on `clm-web-live` with its `durable` feature in its `wasm32` target dependencies and re-export `Live` (`pub use clm_web_live::Live`), so the Worker's module exports the Durable Object class `web/wrangler.toml` binds, and the wasm build of `clm-web` SHALL still succeed.

#### Scenario: the class is exported
- GIVEN `crates/clm-web/Cargo.toml` and `crates/clm-web/src/lib.rs`
- WHEN a test reads them
- THEN the `wasm32` target dependencies name `clm-web-live` with the feature `durable`, and `lib.rs` holds `pub use clm_web_live::Live`

#### Scenario: the Worker builds
- GIVEN the workspace
- WHEN `cargo build -p clm-web --target wasm32-unknown-unknown` runs
- THEN it succeeds

#### Needs
- web.live/builds-for-wasm

### Requirement: proposals
`POST /api/repos/{o}/{r}/proposals` with `/title`, `/body` and `/nodes` SHALL, through `writes-to-object`, append a `proposed` record with the title, its kebab-case slug, the body and the components to `clm/ledger/<login>/web` through the Git Data API with the requester's own token, making no other write to the repository, and answer 200 with the record's `id`. It SHALL answer 422 with `{"error": <field>}`, writing nothing, for a title that is empty or over 120 characters once trimmed or holds a line break, a body over 8,000 characters, a component the replica's graph doesn't hold, or a slug that is empty, and 409 with `{"error": "taken"}`, writing nothing, when the slug is taken: the replica's backlog holds an item of that slug, its `resolved` holds it, or its fold holds an open proposal with it. `DELETE /api/repos/{o}/{r}/proposals/{id}` SHALL, through `writes-to-object`, append a `withdrawn` record naming `{id}` when the requester's own web ledger, read at its tip, holds a `proposed` record of that `id` no `withdrawn` record in it names, and answer 404, writing nothing, for any other `id`. The implementation response SHALL carry `proposals`: the implementation view's (`replica/open-proposals`), in its order, each with `mine` added, whether the writer is the requester's `<login>/web`. Each route SHALL make the access check every API route makes.

#### Scenario: proposed
- GIVEN a signed-in user `alice` who can read `o/a`, whose graph holds component `a`
- WHEN she posts `/title` `"Export CSV"`, `/body` `"Let me export."` and `/nodes` `["a"]`
- THEN the writes made are exactly a blob, a tree, a commit and a ref update on `clm/ledger/alice/web`, with her token, the new `records.jsonl` ends with a `proposed` line with slug `export-csv`, that title, body and components, and the answer carries its `id`

#### Scenario: refused
- GIVEN the same, where the replica's tip holds `backlog/export-csv.md`
- WHEN she posts the same, then `/title` `"Dark mode"`, `/body` `""` and `/nodes` `["nope"]`
- THEN the first answers 409 and the second 422, and nothing is written

#### Scenario: taken by a change
- GIVEN the same, where the replica's `resolved` holds `export-csv`
- WHEN she posts `/title` `"Export CSV"`, `/body` `""` and `/nodes` `[]`
- THEN the answer is 409 with `error` `taken`, and nothing is written

#### Scenario: the slug
- GIVEN the same
- WHEN she posts `/title` `"  Dark mode: on/off!  "`, a title of 70 `a`s, a title of 62 `a`s, a space and `bb`, then `/title` `"!!!"`, each with `/body` `""` and `/nodes` `[]`
- THEN the first three append `proposed` lines with slugs `dark-mode-on-off`, 64 `a`s and 62 `a`s, and the last answers 422 with `error` `slug`, asking neither GitHub nor the object

#### Scenario: withdrawn
- GIVEN `clm/ledger/alice/web` holding `proposed` `p1` and `clm/ledger/bob/web` holding `proposed` `p2`
- WHEN `alice` deletes `p2`, then `p1`, then `p1` again
- THEN the first answers 404 writing nothing, the second appends `withdrawn` naming `p1` to her ledger, and the third answers 404 writing nothing

#### Scenario: open proposals
- GIVEN an implementation view whose `proposals` are `p1` by `alice/web` and `p2` by `bob/web`
- WHEN the implementation response is built for `alice`
- THEN `proposals` lists `p1` with `mine` true, then `p2` with `mine` false, each with the view's other fields as given

#### Serves
- capability/propose-intent
- capability/follow-from-browser
- architecture/per-writer-ledgers

#### Needs
- ledger/proposal-records
- replica/open-proposals
- replica/validate-writes
- web.worker.replica/writes-through-object

### Requirement: capabilities
`GET /api/repos/<o>/<r>/capabilities` SHALL answer the repo's `Replica` object's capabilities view, `{"report", "signed"}` (`replica/capabilities-report`), as `views-from-replica` answers a view.

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser

#### Needs
- web.worker.replica/object-class
- web.worker.replica/applies-events
- web.worker.replica/first-build
- web.worker.replica/alarm-batches
- replica/capabilities-report

#### Scenario: the object's report
- GIVEN a `Replica` platform the test fakes that answers a capabilities view
- WHEN the app requests the capabilities
- THEN the answer is that view

### Requirement: decide
`POST /api/repos/<o>/<r>/capabilities/<slug>/decide` SHALL, through `writes-to-object`, append a `decided` record for `capability/<slug>`, with the body's `value` and `decision` and `settles` the flags `clm_effort::unsettled` names over the default branch's tip and the ledgers the object holds, to `clm/ledger/<login>/web` through the Git Data API with the requester's own token, making no other write, and SHALL answer 200 with the record. It SHALL answer 422 and write nothing for a body without a string `version` of 8 lowercase hex digits, a `value` of `high`, `medium` or `low` and a `decision` of `keep`, `cut`, `redesign` or `accept` (checked before any request to GitHub), for a tip whose specs don't load, for a `<slug>`, percent-decoded, that `CAPABILITIES.md` doesn't hold at the tip, and, with the body `{"error": "signed", "command": "clm capabilities decide <slug> --value <value> --decision <decision>"}`, for a repository whose default branch's tip holds `docs/allowed_signers`; it SHALL answer 409 and write nothing when `version` isn't the capability's version now. The 422 cases are checked before the 409 one. When the object can't read the tip, or the append fails, the route SHALL answer 502 and write nothing more.

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser
- architecture/per-writer-ledgers
- architecture/humans-review-intent

#### Needs
- ledger/decision-records
- replica/validate-writes
- web.worker.replica/writes-through-object

#### Scenario: decide
- GIVEN a capability `x` at version `v` with one unsettled flag `f`
- WHEN the user decides it with `{"version": "v", "value": "low", "decision": "cut"}`
- THEN the writes made are exactly a blob, a tree, a commit and a ref update on `clm/ledger/<login>/web`, with the user's token, the new `records.jsonl` ends with a `decided` line for `capability/x` by `<login>/web` with value `low`, decision `cut` and settles `[f]`, and the answer is that record

#### Scenario: reworded since
- GIVEN the page of `x` was drawn at version `v`, and `x` has since been reworded
- WHEN the user decides it naming `v`
- THEN the Worker answers 409 and makes no write

#### Scenario: bad body
- WHEN the user decides `x` with no body, a `version` that isn't 8 lowercase hex digits, a `value` of `huge` or a `decision` of `maybe`
- THEN the Worker answers 422 each time, makes no request to GitHub or the object and no write

#### Scenario: signed
- GIVEN a default branch whose tip holds `docs/allowed_signers`, and a stale `version`
- WHEN the user decides `x` with `{"version": <stale>, "value": "high", "decision": "keep"}`
- THEN the Worker answers 422, not 409, with `error` `signed` and `command` `clm capabilities decide x --value high --decision keep`, and makes no write

#### Scenario: unknown capability
- WHEN the user decides `nope`, which `CAPABILITIES.md` doesn't hold, and in a second run decides `x` on a tip whose `specs/c/SPEC.md` doesn't parse
- THEN the Worker answers 422 both times and makes no write

#### Scenario: GitHub fails
- GIVEN a GitHub that fails the read of the default branch's ref
- WHEN the user decides `x`
- THEN the Worker answers 502 and no write is made

### Requirement: sign-in-opens-app
When sign-in completes for an invited login, the callback SHALL send the browser to `/app/`.

#### Scenario: callback redirect
- GIVEN the invited list holds `clunny` and GitHub completes the authorization for `clunny`
- WHEN the browser reaches the callback
- THEN the response sets the session cookie and redirects to `/app/`

#### Needs
- web/app-at-app-path

### Requirement: architecture
`GET /api/repos/<o>/<r>/architecture` SHALL answer the repo's `Replica` object's architecture view, `{"architecture", "signed", "indexing"}` (`replica/architecture-report`), as `views-from-replica` answers a view.

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser

#### Needs
- web.worker.replica/object-class
- web.worker.replica/applies-events
- web.worker.replica/first-build
- web.worker.replica/alarm-batches
- replica/architecture-report

#### Scenario: the object's report
- GIVEN a `Replica` platform the test fakes that answers an architecture view with `indexing` true
- WHEN the app requests the architecture
- THEN the answer is that view

### Requirement: architecture-decide
`POST /api/repos/<o>/<r>/architecture/<slug>/decide` SHALL do for `architecture/<slug>` what `decide` does for a capability: append a `decided` record for it, with the body's `value` and `decision` and `settles` the flags `clm_effort::unsettled` names for it, to `clm/ledger/<login>/web` with the requester's own token and no other write, and answer 200 with the record; answer 422, writing nothing, for the bodies `decide` refuses, a tip whose specs don't load, a `<slug>`, percent-decoded, that `ARCHITECTURE.md` doesn't hold at the tip, and, with the body `{"error": "signed", "command": "clm architecture decide <slug> --value <value> --decision <decision>"}`, a repository whose default branch's tip holds `docs/allowed_signers`; answer 409, writing nothing, when `version` isn't the entry's version now, the 422 cases checked first and in `decide`'s order; and answer 502 as `decide` does.

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser
- architecture/per-writer-ledgers
- architecture/humans-review-intent

#### Needs
- ledger/decision-records
- effort/unsettled-flags
- replica/validate-writes
- web.worker.replica/writes-through-object

#### Scenario: decide
- GIVEN an architectural constraint `e` at version `v` with one unsettled flag `f`
- WHEN the user decides it with `{"version": "v", "value": "medium", "decision": "redesign"}`
- THEN the writes made are exactly a blob, a tree, a commit and a ref update on `clm/ledger/<login>/web`, the new `records.jsonl` ends with a `decided` line for `architecture/e` with value `medium`, decision `redesign` and settles `[f]`, and the answer is that record

#### Scenario: not an entry
- WHEN the user decides `nope`, which `ARCHITECTURE.md` doesn't hold, or `e` at a version it no longer has
- THEN the Worker answers 422 for the first and 409 for the second, and makes no write

#### Scenario: signed
- GIVEN a default branch whose tip holds `docs/allowed_signers`
- WHEN the user decides `e` with `{"version": "v", "value": "high", "decision": "keep"}`
- THEN the Worker answers 422 with `error` `signed` and `command` `clm architecture decide e --value high --decision keep`, and makes no write

### Requirement: views-from-replica
The snapshot, implementation, capabilities and architecture routes SHALL, after the access check, answer the view of that name that the repo's `Replica` object answers, computing none of it, and SHALL keep each answer in the Cache API for 30 seconds per repo and view, answering from there without asking the object. While the object answers 503 `{"preparing": <share>}`, the route SHALL answer the same, uncached. When the object can't be reached, the route SHALL answer 503 `{"error": "unavailable"}`. The implementation route SHALL add `mine` to each of the view's `proposals`, as `proposals` describes, answering `proposals` `[]` for a view that carries none, and SHALL ask for it neither another view nor, beyond the access check, GitHub.

#### Scenario: polls within 30 seconds
- GIVEN a signed-in user who can read `o/a`, and a `Replica` platform the test fakes
- WHEN the snapshot is requested three times within 30 seconds
- THEN the three answers are the object's snapshot view, and the object was asked once

#### Scenario: preparing
- GIVEN a fake object that answers 503 `{"preparing": 0.4}`
- WHEN the snapshot is requested twice
- THEN both answers are 503 `{"preparing": 0.4}`, and the object was asked twice

#### Scenario: object down
- GIVEN a fake object that fails every request
- WHEN the capabilities are requested
- THEN the answer is 503 `{"error": "unavailable"}`

#### Scenario: proposals for each reader
- GIVEN `alice` and `bob`, who can read `o/a`, and a fake object whose implementation view carries a proposal by `alice/web`
- WHEN each requests the implementation within 30 seconds
- THEN `alice`'s answer gives it `mine` true and `bob`'s `mine` false, the object was asked once, for the implementation view, and no ledger was read from GitHub

#### Scenario: a view without proposals
- GIVEN a fake object whose implementation view carries no `proposals`
- WHEN the implementation is requested
- THEN the answer is that view with `proposals` `[]`

#### Serves
- capability/follow-from-browser
- capability/see-project-state

#### Needs
- replica/batched-recompute
- replica/open-proposals
- web.worker.replica/object-class
- web.worker.replica/applies-events
- web.worker.replica/first-build
- web.worker.replica/alarm-batches

### Requirement: replica-class-export
`clm-web` SHALL depend on `clm-web-replica` in its `wasm32` target dependencies and re-export `Replica` (`pub use clm_web_replica::Replica`), so the Worker's module exports the Durable Object class `web/wrangler.toml` binds, and the wasm build of `clm-web` SHALL still succeed.

#### Needs
- web.worker.replica/object-class

#### Scenario: the class is exported
- GIVEN `crates/clm-web/Cargo.toml` and `crates/clm-web/src/lib.rs`
- WHEN a test reads them
- THEN the `wasm32` target dependencies name `clm-web-replica`, and `lib.rs` holds `pub use clm_web_replica::Replica`

#### Scenario: the Worker builds
- GIVEN the workspace
- WHEN `cargo build -p clm-web --target wasm32-unknown-unknown` runs
- THEN it succeeds

#### Serves
- capability/follow-from-browser

### Requirement: no-scheduled-event
The Worker SHALL handle no scheduled event: `crates/clm-web/src/worker.rs` SHALL hold no `#[event(scheduled)]` handler and import neither `ScheduledEvent` nor `ScheduleContext`, since no Cron Trigger calls it once `web/wrangler.toml` clears the trigger, and the wasm build of `clm-web` SHALL still succeed.

#### Needs
- web/no-cron-trigger

#### Scenario: no handler
- GIVEN `crates/clm-web/src/worker.rs`
- WHEN a test reads it
- THEN it holds none of `event(scheduled)`, `ScheduledEvent` and `ScheduleContext`

#### Scenario: the Worker builds
- GIVEN the workspace
- WHEN `cargo build -p clm-web --target wasm32-unknown-unknown` runs
- THEN it succeeds

#### Serves
- capability/follow-from-browser

### Requirement: writes-to-object
The proposal, withdrawal and decide routes SHALL, after their body check and the access check, make one post to the repo's `Replica` object's `/write`, and answer the status and body it answers, as it gave them, reading no view, no ledger and no spec file, and making no other request to the object or, beyond the access check, to GitHub. The post's body SHALL be the requester's `login` and `token` with: `kind` `propose` and the trimmed `title` and `body` and the deduplicated `nodes`; `kind` `withdraw` and the `id`; or `kind` `decide`, the `id` `capability/<slug>` or `architecture/<slug>` with the slug percent-decoded, and the body's `version`, `value` and `decision`. When the object can't be reached, the route SHALL answer 503 `{"error": "unavailable"}`. A failed body check or access check SHALL post nothing.

#### Scenario: a proposal passed on
- GIVEN `alice`, signed in with token `u`, who can read `o/a`, and a fake object that answers `/write` with 409 `{"error": "taken"}`
- WHEN she posts `/title` `" Export CSV "`, `/body` `""` and `/nodes` `["a", "a"]`
- THEN the answer is 409 `{"error": "taken"}`, the object was asked once, on `/write`, with `login` `alice`, `token` `u`, `kind` `propose`, `title` `Export CSV` and `nodes` `["a"]`, and GitHub was asked nothing beyond the access check

#### Scenario: a decision passed on
- GIVEN the same, and a fake object that answers `/write` with 200 and a body `b`
- WHEN she posts `{"version": "0123abcd", "value": "low", "decision": "cut"}` to `.../architecture/my%20e/decide`
- THEN the answer is 200 with body `b`, and the one post names `kind` `decide`, `id` `architecture/my e`, that `version`, `value` and `decision`

#### Scenario: a withdrawal passed on
- GIVEN the same, and a fake object that answers `/write` with 404
- WHEN she deletes `.../proposals/p1`
- THEN the answer is 404, and the one post names `kind` `withdraw` and `id` `p1`

#### Scenario: preparing
- GIVEN a fake object that answers `/write` with 503 `{"preparing": 0.4}`
- WHEN she posts a valid proposal
- THEN the answer is 503 `{"preparing": 0.4}`

#### Scenario: object down
- GIVEN a fake object that fails every request
- WHEN she posts a valid proposal, a valid decision and a withdrawal
- THEN each answer is 503 `{"error": "unavailable"}`, and nothing is written

#### Scenario: no access
- GIVEN `alice`, who can't read `o/private`
- WHEN she posts a valid proposal to it
- THEN the answer is 404, and the object was asked nothing

#### Needs
- web.worker.replica/writes-through-object

#### Serves
- capability/propose-intent
- capability/review-capability-cost
- capability/follow-from-browser

### Requirement: reserved-paths
The Worker SHALL redirect a request for `/app`, with a session or without, to `/app/`, and SHALL answer a request for any path its routes don't answer, `/_clm/*` among them, with a bare 404, reading no session and no asset, so the invited list is never served raw and the Worker holds no page of the site.

#### Needs
- web/site-served-directly

#### Scenario: bare app path
- GIVEN a request for `/app`, with a valid session and then with none
- WHEN the Worker answers
- THEN each is a redirect to `/app/`

#### Scenario: the invited list
- GIVEN the fake's assets holding `/_clm/invited.json`
- WHEN `/_clm/invited.json` is requested with a valid session of `clunny` and then with none
- THEN each answer is 404 with an empty body

#### Scenario: a path the assets should have answered
- GIVEN the fake's assets holding a page at `/guide/quickstart`
- WHEN a request for `/guide/quickstart` reaches the Worker with a valid session of `clunny` and then with none
- THEN each answer is 404 with an empty body, holding none of the page's text and no sign-in page

#### Serves
- architecture/invite-only-web

### Requirement: not-invited-page
The "not invited" page the callback answers to a login that isn't invited SHALL say that the web app is in early access, that CLM works fully without it by following the quickstart, linking to `/guide/quickstart`, and that the web app is an optional extra for convenience; it SHALL link to Request early access at `https://github.com/clunny/clm/issues/new?template=early-access.yml`, the one constant in the Worker that names it, and to the homepage at `/`. It SHALL still answer 403, say "not invited", and set no session cookie (`web/invited-users`).

#### Scenario: someone not invited
- GIVEN the invited list holds `clunny` and GitHub completes the authorization for `someone-else`
- WHEN the browser reaches the callback
- THEN the response is 403, sets no session cookie, and its HTML links to `https://github.com/clunny/clm/issues/new?template=early-access.yml`, `/guide/quickstart` and `/`, and says the web app is optional and in early access

#### Serves
- capability/learn-clm

### Requirement: asset-adapter
The Worker adapter's `fetch_asset` SHALL pass the request's method, path, query and headers, but for `cookie` and `authorization`, to the `ASSETS` binding, and answer the binding's status, every header and the body's bytes. It SHALL answer 503 when there is no `ASSETS` binding, and 502 when the fetch fails or the body can't be read. It SHALL keep, for the isolate's life, only 200 answers to a `GET` with no query of a `/_clm/*` path, and answer a later such request for the same path from what it kept without asking the binding.

#### Scenario: passed through
- GIVEN a fake `ASSETS` binding that answers 304 with an `etag` header and no body
- WHEN the adapter answers a `HEAD` of `/fonts/inter.woff2` with query `v=1` and the headers `if-none-match`, `cookie` and `authorization`
- THEN the binding was asked once, with method `HEAD`, path `/fonts/inter.woff2`, query `v=1` and the header `if-none-match` but neither `cookie` nor `authorization`, and the answer is 304 with the binding's headers and an empty body

#### Scenario: no binding, or a failed answer
- GIVEN no `ASSETS` binding, then a fake binding whose fetch fails, then one whose answer's body can't be read
- WHEN the adapter answers a `GET` of `/app/index.html` with each
- THEN the first answer is 503, and the second and third 502

#### Scenario: kept per isolate
- GIVEN a fake `ASSETS` binding that answers `/_clm/invited.json` and `/app/index.html` with 200 and bytes, and `/_clm/missing.json` with 404
- WHEN the adapter answers two `GET`s of each path, and one of `/_clm/invited.json` with query `x=1`
- THEN the binding was asked once for `/_clm/invited.json` without a query, once with it, and twice each for `/app/index.html` and `/_clm/missing.json`, and each answer carries the binding's status and exactly its bytes

#### Serves
- architecture/invite-only-web
