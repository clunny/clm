---
component: web.worker
base: { frontmatter: "fa10593588e5f57d", contract: "f855e1c013ebb892", design: "e9e8b809ad420ef6" }
---

## MODIFIED Contract
The Cloudflare Worker behind `web`: it is deployed with the browser app's static assets under `/app/`, on the app's routes of `clm-app.dev`, while the public site (`web.site`) is a Worker of its own at the root (`web/site-served-directly`, `site-worker-of-its-own`). It answers what `web/wrangler.toml`'s `run_worker_first` routes to it:
- `/app`, which it redirects to `/app/`, and any path no route answers, `/_clm/*` among them, with a bare 404, so the invited list is never served raw (`reserved-paths`);
- `GET /auth/login` and `GET /auth/callback`, GitHub sign-in, which returns to `/app/` with the session in the encrypted `HttpOnly` cookie `__Host-clm`, the flow's `state` kept meanwhile in `__Host-clm-state` (`sign-in`, `invited-sign-in`, `sign-in-opens-app`, `not-invited-page`);
- `POST /webhook`, GitHub's events (`webhook-refresh`);
- `/api/*`, JSON for the app, every request behind one guard: the app's own `Origin` for any request that isn't a `GET`, and a session (`api-guard`). `GET /api/push/key` (`push-subscriptions`) and `GET /api/repos`, the user's repos (`repo-list`); and under `/api/repos/{o}/{r}/`, each after the access check (`repo-access`): `snapshot` (`snapshot-names-its-repo`), `implementation`, `capabilities` and `architecture` (`views-from-replica`, `capabilities`, `architecture`), `live`, the live socket (`live-socket`), `push`, by `GET`, `POST` and `DELETE` (`push-subscriptions`), `branches` and, for a spec branch, `branches/{branch}/review`, `approve`, `comments` and `request-changes` (`branch-routes`), `proposals` by `POST` and `proposals/{id}` by `DELETE` (`proposals`), `backlog/{slug}/answer` (`answers`), and `capabilities/{slug}/decide` and `architecture/{slug}/decide` (`decide`, `architecture-decide`).

The app calls nothing else, and every response shape it reads is this component's contract: changing one is a change here that `web` builds against. The snapshot's backlog items carry `body`, `priority` and `choices` (`replica/snapshot-backlog-text`), the implementation response carries `answers` (`answers`), and each capability and architectural constraint in the reports carries its entry's `text` (`effort/entry-text`). The Worker alone holds a user's token, passing it to the repo's `Replica` object only for a decision's, a proposal's, a withdrawal's or an answer's append, which keeps it only for that request; it writes to the repository only to append an `approved`, `subscribed`, `unsubscribed`, `proposed`, `withdrawn`, `answered`, `commented`, `changes_requested` or `decided` record to the user's web ledger (`per-writer-ledgers`, `push-subscriptions-in-ledger`, `web-proposals-as-ledger-records`, `web-answers-as-ledger-records`, `web-review-feedback-as-ledger-records`), and keeps only state that can be deleted at any time (`web-derived-cache`): what it keeps in the Cache API, each repo's replica, which its `Replica` object (`web.worker.replica`) holds, derived from git and rebuilt from it when lost (`web-repo-replica`), and the open pages' sockets, which its `Live` objects (`web.live`) hold and which store nothing (`web-live-channel`). Its webhook passes each repo's events to the repo's `Replica` object, which recomputes the views, tells the `Live` object, and sends Web Push notifications to the subscriptions in the repo's ledgers. The views are computed by `replica` with the `model`, `check`, `orchestrator`, `view`, `ledger` and `effort` crates compiled to wasm, so the Worker shows what `clm` would report and decides nothing.

Three children answer parts of it beneath this component's router, session and access check: `web.worker.review` the spec-branch list, a spec branch's review, approve, comments and change requests; `web.worker.replica` the views and the writes that need them; and `web.worker.platform` the layer every route stands on. None changes a route or a response shape: what the app reads stays this component's contract.

## MODIFIED Design
Crate `crates/clm-web`, in Rust with `workers-rs`, built for `wasm32-unknown-unknown` (`web-reuses-rust-in-wasm`). Worker-only code sits behind `cfg(target_arch = "wasm32")`, so the host tests cover the rest. It is deployed with the app by `web`'s `wrangler.toml` (`ci`'s release workflow). The invited-users list it enforces is `web`'s (`web/invited-users`): `web`'s build writes it from `specs/web/SPEC.md` to `_clm/invited.json`, and the Worker reads that asset. Three children's crates sit beneath it (`nested-splits`): `clm-web-platform` (`web.worker.platform`), the layer every route stands on; `clm-web-review` (`web.worker.review`), a spec branch's review, its approve, comments and change requests, and the spec-branch list; and `clm-web-replica` (`web.worker.replica`), the repo's `Replica` object, which computes the views. This crate keeps the router (`lib.rs`), the session, the view routes and the writes that go through the object, and calls them; imports run one way, from here to the three, and from the review and replica crates to the platform crate.

**Budget** (`web-on-free-plan`, `web-repo-replica`). It runs on Cloudflare's Free plan: at most 10 ms of CPU and 50 subrequests per Worker request. So a Worker request computes no view: it checks the session and access, then answers a view the repo's `Replica` object (`web.worker.replica`) computed, or passes a write on to it. The object does the work, within a Durable Object's 30 s of CPU per request or alarm and the account's daily Durable Object allowance. The Worker uses no KV namespace; the Cache API holds the memoised repo lists and access checks, and the views for 30 seconds (below).

**Sign-in** (`web-github-app`). One public GitHub App, owned by the operator, that any org or user can install on chosen repos. People sign in through the App's user authorization flow. The user-to-server token and its refresh token live only in an AES-GCM encrypted, `HttpOnly`, `Secure`, `SameSite=Lax` cookie keyed by a Worker secret. There is no server-side session store. A Worker whose cookie key is unusable can seal no session, so its callback refuses rather than set an empty cookie that would send the person round the flow again. The App's permissions (**GitHub App**) are what the routes need: `contents: write` for the ledger append, and `pull_requests: read` because a user's token reads a private repo's pull requests only when the App may.

**Repos and isolation.** The repo list is every repo of the user's App installations (`GET /user/installations/{id}/repositories`) whose default branch has `specs/SPEC.md`, memoised for 5 minutes per token hash in the Cache API, so navigating doesn't repeat its GitHub requests; a newly installed repo can take that long to appear. An empty list isn't memoised, so someone who has just installed the App sees their repo at once. Its page and repo limits keep it within the subrequest budget. Repos from many organisations share one cache, so isolation is the main security boundary. Every API request for a repo first checks, with the user's own token, that they can read it (`web.worker.platform/access-check`), and a user who can't read it gets 404 even when its data is cached. Cache keys always include `<owner>/<repo>`, and no response mixes repos.

**Views** (`views-from-replica`). `/snapshot`, `/implementation`, `/capabilities` and `/architecture` answer, after the access check, the view of the same name that the repo's `Replica` object answers (`GET /view/<name>`), reached through the `Replicas` trait, which tests fake. Their shapes are this component's contract, and `replica` computes them: the snapshot (`sha`, `coverage`, `sizes`, the `spec` model, components, changes, backlog, `resolved`, decisions, violations), to which the Worker adds `repo`, the one fact only it knows (`snapshot-names-its-repo`); the implementation response (`sha`, `now`, `expiry_hours`, `coverage`, `flight`, `blockers`, `pages`, `activity`, `taken` and `proposals`, to each of which the Worker adds `mine`); the capabilities report (`report`, `signed`); and the architecture report (`architecture`, `signed`, `indexing`). The Worker keeps each view in the Cache API for 30 seconds under `view:<o>/<r>:<name>`, so an open tab's polls rarely reach the object, and adds `repo` as it answers, so a cached view and a fresh one carry it alike; a request with a valid `fresh` skips that cache and asks the object with `fresh=1`, which brings itself up to the tip first, and stores what it answers. The Worker adds `mine` for the requester to the implementation view it holds or was just given, so one cached view serves every reader. The views are up to a minute behind the latest push, which the object batches (`web-repo-replica`).

**Webhook.** `/webhook` verifies the signature, then passes each event that names a repository, `push`, `pull_request`, `installation` and `installation_repositories`, with its delivery and installation, to the named repos' `Replica` objects (`POST /event`), and answers. It computes nothing and stores nothing; the objects fetch, apply, recompute, notify the open pages and send notifications (`web.worker.replica`). A failed post is logged.

**Spec branches and reviews.** `web.worker.review` answers the spec-branch list, a branch's review and its writes from `web.worker.platform`'s live state, kept in the Cache API for 60 seconds, and `fresh` works for them. The router takes the branch's name from the path, checks a write's body before the access check, and gives a comment's check the requirement IDs of the snapshot view it would serve (`branch-routes`).

**Writes through the object** (`writes-to-object`). A proposal, a withdrawal, an answer and a decision pass through the Worker in one post to the repo's `Replica` object, `POST /write`, reached through the `Replicas` trait the views use, so each costs the Worker its access check and that post and reads no view, ledger or spec file. The body is the requester's login and token, which the object keeps only for the request, with `"kind"` and the write's fields. The object brings itself up to the tip, validates and appends (`web.worker.replica/writes-through-object`, `web.worker.replica/write-refusals`), and the route answers its status and body as it gave them, so its refusals, its 503 `{"preparing": <share>}` and its 502 reach the app unchanged. Nothing else is told of the write: GitHub's `push` webhook for the append reaches an object that has already applied it. Their tests drive a real `clm_web_replica::Object` over the fake GitHub (`clm-web-replica` and `clm-replica` are dev-dependencies), but for `writes-to-object`'s scripted scenarios.

**Proposals** (`web-proposals-as-ledger-records`, `proposals`). `POST /api/repos/{o}/{r}/proposals` takes an object with `/title`, `/body` and `/nodes`. The title and body are stored trimmed; the body may be empty; the components are deduplicated and may be empty. The slug is `replica`'s, computed by the object (`replica/validate-writes`); the Worker holds no copy of that rule: its body check refuses only a title that lowercases to no ASCII letter or digit, the one whose slug would be empty. The body is checked first, before any request to GitHub, as Approve's is; then the access check; then the object, which checks the components and the slug against its state brought up to the tip and appends. A withdrawal goes to the object the same way. The implementation response's `proposals` are the implementation view's, each with `mine` added. The object applies the record and recomputes before it answers, so the app's reload after the write (`fresh`) shows it. The Worker files nothing: an item reaches `main` through `clm rebase` on the next `impl/*` branch and that branch's land.

**Decide** (`decide`, `architecture-decide`, decision `effort-records`). The body is checked before any request to GitHub; then the access check; then the object, which brings itself up to the tip, so the version the person saw is compared with the entry's version at the tip now, as approve compares the fingerprint, and appends the `decided` record, settling the flags `effort/unsettled-flags` names over the ledgers it holds, which include every append it has seen. A signed repository counts only records signed with a key the Worker doesn't hold (`signed-repository`), so there the object refuses and names the command to run. The capabilities and architecture views have no history: commits, the backlog items' lives and the sizes need git, so those numbers are absent and the app says to run `clm capabilities <slug>` for them.

**Live pushes** (`web-live-channel`). `GET /api/repos/{o}/{r}/live`, with or without `Upgrade: websocket`, first gets the same `Origin` check the routes that write get, before the session's: a WebSocket handshake isn't bound by CORS and browsers always send `Origin` on one, so this, not `SameSite=Lax`, keeps another site from opening the socket with the person's cookie. It then makes the session and access checks every API route makes, then forwards the request through the `LIVE` Durable Object binding to the repo's `Live` object (`web.live`), `id_from_name("<o>/<r>")`, and returns its 101, with a renewed session cookie when the route would add one. The core reaches the object through `web.worker.platform`'s `Live` trait (forward a request, post a notice, post a send), which tests fake; the request it forwards, with its method and `Upgrade` alone, is built in host code, and `web.worker.platform`'s `http::Response` carries an opaque upgrade that only `worker.rs`'s adapter fills in with the object's socket. The notices themselves come from the repo's `Replica` object after each recompute (`web.worker.replica/alarm-batches`), so a page refreshes once the views it will read have changed. A ledger append is a push, so an approval, a check or a land made anywhere reaches the open pages.

A refresh a notice prompts must not be answered from a cache older than the notice. The view routes take `?fresh=<delivery>` as **Views** says; the routes that answer from the live state, the spec-branch list, a spec branch's review and `GET .../push`, carry a valid one to `live::cached` with the request's `Access`, which keeps what it reads under a key naming the delivery (`live:<o>/<r>#<delivery>:<what>`, `web.worker.platform/fresh-replaces-plain`), so the first request after a notice fetches anew from GitHub and later requests for that notice share it for 60 seconds. Two tabs in one data centre that ask at the same instant may both fetch, which is accepted. `fresh` lets an invited person make the Worker and the object fetch more than once a minute; only invited people can sign in (`invite-only-web`).

**Notifications** (`push-subscriptions-in-ledger`). `GET /api/push/key` gives the app the VAPID public key it subscribes with. A person's *web subscriptions* are the `subscriptions` of `clm_ledger::fold_records` run over their own web ledger branch alone, `clm/ledger/<login>/web` (`web-subscriptions-by-branch`): the branch names the person, the record's `writer` is not consulted, and only that branch's `unsubscribed` records end them, so a record someone commits to a branch of their own neither counts as that person's nor ends theirs. The Worker's web ledger branches are those named exactly `clm/ledger/<login>/web` with `<login>` a name `access::valid_name` accepts (a name it admits that no login has is asked once and refused by the permission check); a branch named otherwise under `clm/ledger/` holds nobody's web subscriptions. The ledger reader keeps the per-branch sources it fetches (`ledger.rs`'s `Sources`, cached with the live state), and the switch's `GET` and the `Replica` object's notifications fold the web branches from them one at a time, so nothing is fetched twice. A subscription's endpoint must be on a known push service (host `fcm.googleapis.com`, or ending in `.push.services.mozilla.com`, `.push.apple.com` or `.notify.windows.com`), so the Worker never sends to an address an invited person made up. `POST` and `DELETE` decide what to write from the user's own `clm/ledger/<login>/web` at its tip, read as Approve does, not from the minute-old fold, so a quick off after an on is never lost; each append is a push, so its notice brings the app's view of the switch up to date.

The notifications a push or a pull request calls for (`waiting_notice`, `review_notice`) are sent by the repo's `Replica` object after it recomputes (`web.worker.replica/push-waiting`, `push-review`), with the recipients, permission checks and limits they had when the webhook sent them.

The Worker reads its secrets as `GITHUB_APP_ID`, `GITHUB_APP_PRIVATE_KEY` (a PKCS#8 PEM, which WebCrypto can import), `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET`, `GITHUB_WEBHOOK_SECRET` and `COOKIE_KEY` (32 random bytes, base64), and, when the operator has set them, `VAPID_PUBLIC_KEY` and `VAPID_PRIVATE_KEY` (`web.live`'s Design); without them it sends no notifications. CI deploys on release (`ci`).

**GitHub App.** The App is `clm-spec-review` ("CLM Spec Review", App ID 5433936), owned by `clunny`. The app links to `https://github.com/apps/clm-spec-review/installations/new` so people can install it. Its permissions are `contents: write`, `pull_requests: read` and `metadata: read`, and it subscribes to the `push` and `pull_request` events. Only the operator changes them, in the App's settings, and every installation must then accept the change. The App's settings on GitHub are the record of how to create it again.

**Assets adapter** (`asset-adapter`). The session cookie and the user's credentials stay with the Worker, so they never reach the binding. A `/_clm/*` answer changes only with a deploy, which starts new isolates, so the adapter keeps it for the isolate's life; the site's answers change without one and are never kept. `worker.rs` is built only for `wasm32`, so what the adapter decides about an asset request (what reaches the binding, the 503 and 502 answers, what it keeps) is code built for the host too, over a binding that `worker.rs` supplies from `ASSETS` and tests fake; `worker.rs` keeps only the binding's calls.

**Size** (`slim-components`). This component's budget is 72,000 code tokens and 16,000 spec tokens, above the defaults, because what stays is the Worker's one shell: the router every route passes through, the session and sign-in, the view routes and their cache, proposals, decide, the webhook's dispatch and the live socket, and the Worker adapter, with the tests that drive them through `handle`.

## ADDED Requirements

### Requirement: snapshot-names-its-repo
`GET /api/repos/{o}/{r}/snapshot` SHALL answer, after the access check, a JSON object whose top-level keys are exactly `repo`, `sha`, `coverage`, `sizes`, `spec`, `nodes`, `changes`, `backlog`, `resolved`, `decisions` and `violations`: `repo` a string, the repository's `<owner>/<repo>` as the access check names it, whatever casing the route was asked in, and the others the repo's `Replica` object's snapshot view's (`replica/snapshot-matches-clm`), as `views-from-replica` answers a view. Every snapshot answer SHALL carry `repo`, whether it comes from the Worker's cache or from the object.

#### Scenario: the snapshot names its repo
- GIVEN a signed-in user who can read `o/a`, and a real `Replica` object over the fake GitHub that has prepared `o/a`
- WHEN the snapshot is requested as `/api/repos/o/a/snapshot`, then again within 30 seconds, then as `/api/repos/O/A/snapshot?fresh=d1`
- THEN each answer is 200, its top-level keys are exactly those eleven, and its `repo` is `o/a`, and the object was asked twice

#### Serves
- capability/follow-from-browser
- capability/see-project-state

### Requirement: api-guard
Every `/api/*` request whose method isn't `GET` SHALL answer 403, before the session check and writing nothing, unless its `Origin` header equals the app's own origin; a missing `Origin` is refused. Past that check, every `/api/*` request SHALL answer 401 without a valid session (`sign-in`, `invited-sign-in`), and, with one, 404 for a path or method no route answers, writing nothing.

#### Scenario: another origin's write
- GIVEN a signed-in user who can read `o/a`
- WHEN they post approve on `spec/x` with `Origin: https://evil.example` and then with no `Origin`, and post a proposal and a push subscription with `Origin: https://evil.example`
- THEN every answer is 403, GitHub and the object were asked nothing, and nothing is written

#### Scenario: a path no route answers
- GIVEN a signed-in user who can read `o/a`
- WHEN they request `GET /api/nope`, `GET /api/repos/o/a/nope` and `POST /api/repos/o/a/snapshot` from the app's origin, and `GET /api/nope` with no session
- THEN the first three answer 404 and the last 401, and nothing is written

#### Serves
- architecture/invite-only-web

### Requirement: branch-routes
The spec-branch routes SHALL take the branch's name from the path's segments between `branches/` and the route's last segment, joined with `/` and percent-decoded, so `spec%2Fx` and `spec/x` name one branch. An approve whose body isn't JSON or has no string `fingerprint` SHALL answer an empty 422 and write nothing. The router SHALL check an approve's, a comment's and a change request's body, the last two by the review's body checks (`web.worker.review/review-feedback`, `web.worker.review/request-changes`), whose 422 `{"error": <field>}` it answers, before the access check and any request to GitHub beyond the session's. A comment's target SHALL be looked for among the requirement IDs (`nodes[].requirements[].id`) of the snapshot view `views-from-replica` would answer, and among none when that view can't be had, which the review answers 502. `GET .../branches` SHALL answer the list `web.worker.review/spec-branches` gives, and 502 when it gives none.

#### Scenario: nothing named
- GIVEN a spec branch `spec/x` of a repo the user can read
- WHEN the user approves it with no body, with `not json`, with `{}` and with `{"fingerprint": 3}`; posts a comment to it with no body and with `{"fingerprint": "f", "target": null, "text": " "}`; and requests changes with `{"fingerprint": "f"}`
- THEN each answers 422: the approves with an empty body, the first comment with `{"error": "fingerprint"}`, and the second comment and the change request with `{"error": "text"}`; GitHub saw no request beyond the session's, and nothing is written

#### Scenario: branch routes
- GIVEN `o/a`'s spec branch `spec/x`, an open pull request for it, and a fake object whose snapshot view's graph holds `m/old`; in a second run, nothing cached and an object that is down
- WHEN the user requests `.../branches/spec%2Fx/review`; approves `spec/x` with its fingerprint; comments on `spec/x` targeting `m/old` and then `m/missing`; in the second run comments targeting `m/old`; and requests `.../branches` while GitHub fails `matching-refs/heads/spec/`
- THEN the review is `spec/x`'s; the approve answers 200 with the four writes of an append; the comments answer 200 and 422, then 502; and the list answers 502

#### Needs
- web.worker.review/review-feedback
- web.worker.review/request-changes
- web.worker.review/spec-branches

#### Serves
- capability/approve-intent
- capability/follow-from-browser

## MODIFIED Requirements

### Requirement: sign-in <!-- base: 17761a3841570f7d -->
The Worker SHALL sign people in through the GitHub App's user authorization flow, and SHALL keep their tokens only in the encrypted `HttpOnly` cookie `__Host-clm`, never in the Cache API or any other store. `GET /auth/login` SHALL redirect to `https://github.com/login/oauth/authorize` with the App's client ID, the `redirect_uri` `<origin>/auth/callback` and a random `state`, which it also sets in the `HttpOnly` cookie `__Host-clm-state` for 10 minutes. The callback SHALL answer 400, setting no session cookie, when `code` or `state` is missing, `state` differs from that cookie's, or GitHub refuses the code, and SHALL clear the state cookie in every other answer. When sign-in would complete for an invited login and `COOKIE_KEY` is missing or isn't 32 bytes in base64, the callback SHALL answer 500 with a page saying sign-in isn't available, setting no session cookie. An API request whose session expires within 60 seconds SHALL be answered with tokens renewed through its refresh token, the answer re-setting the session cookie, and SHALL answer 401 when the renewal fails; a token GitHub gave without an expiry is never renewed.

#### Scenario: callback
- GIVEN a GitHub that completes the authorization with a code
- WHEN the browser reaches the callback
- THEN the response sets an `HttpOnly` cookie that doesn't contain the token in clear, and nothing written to the Cache API contains the token

#### Scenario: tampered cookie
- GIVEN a session cookie with one byte changed
- WHEN any API request carries it
- THEN the Worker answers 401

#### Scenario: the flow's edges
- GIVEN the App's client ID `client-id` and a GitHub that completes the authorization for code `c1`
- WHEN the browser requests `/auth/login`; reaches the callback with a forged `state`, with no `code`, and with a code GitHub refuses; and a session that expired is used for the repo list, with a refresh token GitHub renews, then with one GitHub refuses; and a session GitHub gave no expiry is used 30 days on
- THEN the login redirects to GitHub's authorize URL with `client-id`, the callback's `redirect_uri` and a `state` equal to the `__Host-clm-state` cookie it sets; the three callbacks answer 400 and set no session cookie; the first expired session's answer is 200, re-sets `__Host-clm` and its GitHub call carries the renewed token, and the second's is 401; and the last is 200 with the original token

#### Scenario: no usable cookie key
- GIVEN a Worker with no `COOKIE_KEY`, then one whose `COOKIE_KEY` decodes to 16 bytes, and a GitHub that completes the authorization for an invited login
- WHEN the browser reaches the callback
- THEN each answer is 500, is no redirect, sets no `__Host-clm` cookie and clears the state cookie

### Requirement: repo-list <!-- base: b7761292549331a9 -->
`GET /api/repos` SHALL list the repos of the user's App installations whose default branch contains `specs/SPEC.md`, and no others, as a JSON list of `{owner, name, default_branch}` sorted by owner then name, each repo once, `default_branch` `main` when GitHub gives none. It SHALL read at most 25 pages of 100 repositories across the installations and check at most 1,000 repos, leaving out any beyond, SHALL answer 502 when a GitHub request fails, and SHALL answer a user's further listings for 5 minutes from the Cache API, keyed by a hash of their token, without asking GitHub, unless the list was empty.

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

#### Scenario: the list's shape and limits
- GIVEN two installations that both cover `o/b`, with `p/a` and `o/c`, all CLM repos, and `o/c` with no default branch given; then 60 installations of one CLM repo each; then a GitHub that fails the installations' listing
- WHEN the user lists repos each time
- THEN the first list is `o/b`, `o/c` with `default_branch` `main`, then `p/a`, each once; the second answers 200 within 50 GitHub requests; and the third answers 502

### Requirement: webhook-refresh <!-- base: 2b72012d95491a44 -->
The Worker SHALL answer 401 to a webhook whose signature doesn't verify, and to every webhook when `GITHUB_WEBHOOK_SECRET` isn't set, and SHALL pass each verified `push`, `pull_request`, `installation` and `installation_repositories` event to the `Replica` object of each repo it names, computing no view and storing nothing. The repos an event names are those of its `repository`, `repositories`, `repositories_added` and `repositories_removed` whose `full_name` is `<owner>/<repo>` with both parts names the access check's name rule accepts (`web.worker.platform/access-check`), each once. A passed-on event SHALL be answered 200, whatever the objects answer; a verified event of any other kind 204, reaching no object; and a verified event of those four kinds whose body isn't JSON 400, reaching no object.

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
- THEN the object named `o/a` received the event with delivery `d1` and the installation's id, and the Worker stored nothing in the Cache API

#### Scenario: forged webhook
- GIVEN a push webhook with a wrong signature
- WHEN it reaches the Worker
- THEN the Worker answers 401 and no object received anything

#### Scenario: what the webhook answers
- GIVEN a `Replica` platform the test fakes
- WHEN verified webhooks arrive: a `ping`; a `push` for `o/a` while the object fails every request; an `installation_repositories` adding `o/b`, `o/b` again and `../x`; and a `pull_request` and an `installation` whose bodies aren't JSON; then a `push` to a Worker with no `GITHUB_WEBHOOK_SECRET`
- THEN the `ping` answers 204, the `push` 200, the `installation_repositories` 200 having reached `o/b`'s object once and no other, the two unreadable bodies 400, and the last 401, and no object received the `ping`, the unreadable bodies or the last

### Requirement: cache-is-disposable <!-- base: 58ffcb8595e9dfab -->
Every view SHALL show the same data after the Cache API and every `Replica` object's storage are emptied, once rebuilt from GitHub, and nothing outside this component and its children `web.worker.platform`, `web.worker.review` and `web.worker.replica`, which reach them only through `web.worker.platform`'s traits or the object's own storage, SHALL read them. Of the workspace's crates, only `clm-web`, `web.live`'s `clm-web-live` and `clm-web-replica` SHALL depend on `worker`, only `clm-web` SHALL use its Cache API binding, and none SHALL use a KV namespace: `clm-web-live` holds the `Live` object and `clm-web-replica` the `Replica` object, which need `worker`'s Durable Object types and nothing of the caches.

#### Scenario: emptied cache
- GIVEN a repo whose snapshot was served once with a complete index
- WHEN the Cache API and its `Replica` object's storage are emptied and the snapshot is requested until its index completes
- THEN the response equals the first one

#### Scenario: the worker crate's dependants
- GIVEN the workspace's crate manifests and sources
- WHEN each `Cargo.toml` is read for a `worker` dependency, the sources of `clm-web-live` and `clm-web-replica` for `kv`, `KvStore` or `Cache`, and every crate's `src/` for `KvStore`
- THEN no crate but `clm-web`, `clm-web-live` and `clm-web-replica` depends on `worker`, neither object's sources names the caches, and no crate's `src/` names `KvStore`

#### Needs
- web.worker.platform/platform-parts

#### Serves
- architecture/git-is-truth
- architecture/web-optional

### Requirement: pr-routes-removed <!-- base: 678e848e662a1116 -->
The Worker SHALL answer 404 to `GET /api/repos/{o}/{r}/pulls`, `GET /api/repos/{o}/{r}/pulls/{n}/review` and `POST /api/repos/{o}/{r}/pulls/{n}/approve`, making no write to the repository for the last.

#### Serves
- architecture/no-dead-code
- architecture/spec-is-current

#### Scenario: old routes
- GIVEN a signed-in user who can read `o/a`, and an open pull request #7 whose head is the spec branch `spec/x`
- WHEN they request `o/a`'s `pulls`, PR 7's review, and then post PR 7's approve
- THEN each answer is 404, and no ledger ref or other ref of `o/a` changes

### Requirement: live-socket <!-- base: 80387388b34b7bfe -->
`GET /api/repos/{o}/{r}/live` SHALL answer 403, before any other check, unless the request's `Origin` header equals the app's own origin. With that origin and `Upgrade: websocket` it SHALL answer 401 without a valid session and 404 when the user can't read the repo, as every API route does, and otherwise SHALL forward the request to the `Live` object named `<o>/<r>` and return its answer, status, headers and body. The forwarded request SHALL carry the request's method and its `Upgrade` header and no other header, the session cookie never, and the route SHALL answer 502 when the object answers 101 without a socket or can't be reached. Without the upgrade header it SHALL answer 426, and without the `LIVE` binding 503.

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

#### Scenario: what reaches the object
- GIVEN a signed-in user who can read `o/a`, and a `Live` platform the test fakes that records what it is given
- WHEN they request `/api/repos/o/a/live` with `Upgrade: websocket`, their session cookie, `Origin`, `Authorization` and `User-Agent`, then while the fake answers 101 without a socket, then while it answers 404 with a header and a body
- THEN the fake was given a `GET` whose only header is `upgrade`, the second answer is 502, and the third is that 404 with its header and body

#### Needs
- web.live/hub

### Requirement: push-subscriptions <!-- base: c5f1fe545c114b52 -->
`GET /api/push/key` SHALL answer the `VAPID_PUBLIC_KEY` secret, or 404 when it isn't set. `POST /api/repos/{o}/{r}/push` with a body `{"endpoint", "keys": {"p256dh", "auth"}}` SHALL append a `subscribed` record with those three to the user's web ledger, unless that ledger, read at its tip, already holds a live subscription for the endpoint, and SHALL answer 422, writing nothing, unless the endpoint is an `https` URL on a known push service, `p256dh` a 65-byte uncompressed point and `auth` 16 bytes. `DELETE` on that route with `{"endpoint"}` SHALL append an `unsubscribed` record for each live subscription in the user's web ledger, read at its tip, for that endpoint. `POST` and `DELETE` SHALL answer 200 when done, and 422, writing nothing, for a body that isn't the JSON they take; when the user's web ledger can't be read at its tip, or an append fails, they SHALL answer 502, writing nothing more. `GET` on it with `?endpoint=` SHALL answer whether the user's web subscriptions, the live subscriptions of their own branch `clm/ledger/<login>/web` folded alone (`web-subscriptions-by-branch`), hold one for the endpoint, whatever the records of other branches say, and 502 when the ledgers can't be read. Each SHALL make the access check every API route makes.

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

#### Scenario: failures
- GIVEN a signed-in user `alice` who can read `o/a`, with a live subscription for an endpoint
- WHEN she posts a body that isn't JSON and deletes with no body; deletes the endpoint while GitHub fails the read of her web ledger's ref, then while it refuses the ref's move; and asks `GET .../push?endpoint=` while GitHub fails the ledgers' read
- THEN the first two answer 422, the next two 502, and the last 502, and nothing is written

#### Needs
- ledger/subscription-records

### Requirement: live-class-export <!-- base: d6ddd6fdf06837a2 -->
The Worker's wasm module SHALL export the Durable Object class `Live` (`web.live`) that `web/wrangler.toml` binds as `LIVE`, and the wasm build of `clm-web` SHALL succeed.

#### Scenario: the Worker builds
- GIVEN the workspace
- WHEN `cargo build -p clm-web --target wasm32-unknown-unknown` runs
- THEN it succeeds, and the module it builds carries the `Live` class's wasm-bindgen exports

#### Needs
- web.live/builds-for-wasm

### Requirement: proposals <!-- base: e608c36e67c5139d -->
`POST /api/repos/{o}/{r}/proposals` with `/title`, `/body` and `/nodes` SHALL, through `writes-to-object`, append a `proposed` record with the title, its kebab-case slug, the body and the components to `clm/ledger/<login>/web` through the Git Data API with the requester's own token, making no other write to the repository, and answer 200 with the record's `id`. It SHALL answer 422, writing nothing and asking neither GitHub, beyond the session, nor the object, with `{"error": "title"}` for a body that isn't JSON or a `title` that is missing, not a string, empty or over 120 characters once trimmed, or holds a line break; `{"error": "body"}` for a `body` that is missing, not a string or over 8,000 characters once trimmed; `{"error": "nodes"}` for a `/nodes` that isn't a list of at most 10 strings; and `{"error": "slug"}` for a title that lowercases to no ASCII letter or digit, whose slug would be empty. Any other refusal SHALL be the object's (`web.worker.replica/write-refusals`). `DELETE /api/repos/{o}/{r}/proposals/{id}` SHALL, through `writes-to-object`, pass the withdrawal of `{id}` to the object, which appends a `withdrawn` record naming it or refuses. The implementation response SHALL carry `proposals`: the implementation view's (`replica/open-proposals`), in its order, each with `mine` added, whether the writer is the requester's `<login>/web`. Each route SHALL make the access check every API route makes.

#### Scenario: proposed
- GIVEN a signed-in user `alice` who can read `o/a`, whose graph holds component `a`
- WHEN she posts `/title` `"Export CSV"`, `/body` `"Let me export."` and `/nodes` `["a"]`
- THEN the writes made are exactly a blob, a tree, a commit and a ref update on `clm/ledger/alice/web`, with her token, the new `records.jsonl` ends with a `proposed` line with slug `export-csv`, that title, body and components, and the answer carries its `id`

#### Scenario: the slug
- GIVEN the same
- WHEN she posts `/title` `"  Dark mode: on/off!  "`, a title of 70 `a`s, a title of 62 `a`s, a space and `bb`, then `/title` `"!!!"`, each with `/body` `""` and `/nodes` `[]`
- THEN the first three append `proposed` lines with slugs `dark-mode-on-off`, 64 `a`s and 62 `a`s, and the last answers 422 with `error` `slug`, asking neither GitHub nor the object

#### Scenario: refused before asking anyone
- GIVEN the same
- WHEN she posts a body that isn't JSON, one with no `title`, a `title` of 121 characters, one holding a line break, one with no `body`, a `body` of 8,001 characters, `/nodes` `"a"`, `/nodes` `[1]` and `/nodes` of 11 IDs
- THEN the answers are 422 with `error` `title` for the first four, `body` for the next two and `nodes` for the last three, and GitHub, beyond the session, and the object were asked nothing

#### Scenario: open proposals
- GIVEN an implementation view whose `proposals` are `p1` by `alice/web` and `p2` by `bob/web`
- WHEN the implementation response is built for `alice`
- THEN `proposals` lists `p1` with `mine` true, then `p2` with `mine` false, each with the view's other fields as given

#### Serves
- capability/propose-intent
- capability/follow-from-browser
- architecture/git-is-truth

#### Needs
- ledger/proposal-records
- replica/open-proposals
- replica/validate-writes
- web.worker.replica/writes-through-object
- web.worker.replica/write-refusals

### Requirement: decide <!-- base: fb1ef580e8ca1c76 -->
`POST /api/repos/<o>/<r>/capabilities/<slug>/decide` SHALL, through `writes-to-object`, append a `decided` record for `capability/<slug>`, the slug percent-decoded, with the body's `value` and `decision` and `settles` the workaround flags `effort/unsettled-flags` names for it over the default branch's tip and the ledgers the object holds, to `clm/ledger/<login>/web` through the Git Data API with the requester's own token, making no other write, and SHALL answer 200 with the record. It SHALL answer 422, writing nothing and asking neither GitHub, beyond the session, nor the object, for a body without a string `version` of 8 lowercase hex digits, a `value` of `high`, `medium` or `low` and a `decision` of `keep`, `cut`, `redesign` or `accept`. Any other refusal SHALL be the object's (`web.worker.replica/write-refusals`).

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser
- architecture/git-is-truth
- architecture/humans-review-intent

#### Needs
- ledger/decision-records
- effort/unsettled-flags
- replica/validate-writes
- web.worker.replica/writes-through-object
- web.worker.replica/write-refusals

#### Scenario: decide
- GIVEN a capability `x` at version `v` with one unsettled flag `f`
- WHEN the user decides it with `{"version": "v", "value": "low", "decision": "cut"}`
- THEN the writes made are exactly a blob, a tree, a commit and a ref update on `clm/ledger/<login>/web`, with the user's token, the new `records.jsonl` ends with a `decided` line for `capability/x` by `<login>/web` with value `low`, decision `cut` and settles `[f]`, and the answer is that record

#### Scenario: bad body
- WHEN the user decides `x` with no body, a `version` that isn't 8 lowercase hex digits, a `value` of `huge` or a `decision` of `maybe`
- THEN the Worker answers 422 each time, makes no request to GitHub or the object and no write

### Requirement: sign-in-opens-app <!-- base: bf3c81376157cc43 -->
When sign-in completes for an invited login, the callback SHALL send the browser to `/app/`.

#### Scenario: callback redirect
- GIVEN the invited list holds `clunny` and GitHub completes the authorization for `clunny`
- WHEN the browser reaches the callback
- THEN the response sets the session cookie and redirects to `/app/`

### Requirement: architecture-decide <!-- base: 296cef0941ebe8dd -->
`POST /api/repos/<o>/<r>/architecture/<slug>/decide` SHALL do for `architecture/<slug>` what `decide` does for a capability: append a `decided` record for it, the slug percent-decoded, with the body's `value` and `decision` and `settles` the flags `effort/unsettled-flags` names for it, to `clm/ledger/<login>/web` with the requester's own token and no other write, and answer 200 with the record; answer 422, writing nothing and asking neither GitHub, beyond the session, nor the object, for the bodies `decide` refuses; and leave any other refusal to the object (`web.worker.replica/write-refusals`).

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser
- architecture/git-is-truth
- architecture/humans-review-intent

#### Needs
- ledger/decision-records
- effort/unsettled-flags
- replica/validate-writes
- web.worker.replica/writes-through-object
- web.worker.replica/write-refusals

#### Scenario: decide
- GIVEN an architectural constraint `e` at version `v` with one unsettled flag `f`
- WHEN the user decides it with `{"version": "v", "value": "medium", "decision": "redesign"}`
- THEN the writes made are exactly a blob, a tree, a commit and a ref update on `clm/ledger/<login>/web`, the new `records.jsonl` ends with a `decided` line for `architecture/e` with value `medium`, decision `redesign` and settles `[f]`, and the answer is that record

### Requirement: views-from-replica <!-- base: b85c6eb3b6652672 -->
The snapshot, implementation, capabilities and architecture routes SHALL, after the access check, answer the view of that name that the repo's `Replica` object answers, the snapshot with `repo` added (`snapshot-names-its-repo`), computing none of it, and SHALL keep each answer in the Cache API for 30 seconds per repo and view, answering from there without asking the object. While the object answers 503 `{"preparing": <share>}`, the route SHALL answer the same, uncached. When the object can't be reached, or answers anything but 200 with a `view` or 503 with `preparing`, the route SHALL answer 503 `{"error": "unavailable"}`, uncached. The implementation route SHALL add `mine` to each of the view's `proposals`, as `proposals` describes, answering `proposals` `[]` for a view that carries none, and SHALL ask for it neither another view nor, beyond the access check, GitHub.

#### Scenario: polls within 30 seconds
- GIVEN a signed-in user who can read `o/a`, and a `Replica` platform the test fakes
- WHEN the snapshot is requested three times within 30 seconds
- THEN the three answers are the object's snapshot view with `repo` added, and the object was asked once

#### Scenario: preparing
- GIVEN a fake object that answers 503 `{"preparing": 0.4}`
- WHEN the snapshot is requested twice
- THEN both answers are 503 `{"preparing": 0.4}`, and the object was asked twice

#### Scenario: object down
- GIVEN a fake object that fails every request, then one that answers the architecture view with 500 `{"boom": true}`
- WHEN each of the four views is requested of the first, and the architecture twice of the second
- THEN every answer is 503 `{"error": "unavailable"}`, and the second object was asked twice

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

### Requirement: replica-class-export <!-- base: 92f08d8bbc0102b2 -->
The Worker's wasm module SHALL export the Durable Object class `Replica` (`web.worker.replica`) that `web/wrangler.toml` binds as `REPLICA`, and the wasm build of `clm-web` SHALL succeed.

#### Needs
- web.worker.replica/object-class

#### Scenario: the Worker builds
- GIVEN the workspace
- WHEN `cargo build -p clm-web --target wasm32-unknown-unknown` runs
- THEN it succeeds, and the module it builds carries the `Replica` class's wasm-bindgen exports

#### Serves
- capability/follow-from-browser

### Requirement: writes-to-object <!-- base: a68570e6d73914b8 -->
The proposal, withdrawal, answer and decide routes SHALL, after their body check and the access check, make one post to the repo's `Replica` object's `/write`, and answer the status and body it answers, as it gave them, reading no view, no ledger and no spec file, and making no other request to the object or, beyond the access check, to GitHub: an object's reply with an empty body with that status and no body, and any other with `content-type: application/json` and the body as given. The post's body SHALL be the requester's `login` and `token` with: `kind` `propose` and the trimmed `title` and `body` and the deduplicated `nodes`; `kind` `withdraw` and the `id`; `kind` `answer`, the `item`, the slug percent-decoded, and the body's `answer`; or `kind` `decide`, the `id` `capability/<slug>` or `architecture/<slug>` with the slug percent-decoded, and the body's `version`, `value` and `decision`. When the object can't be reached, the route SHALL answer 503 `{"error": "unavailable"}`. A failed body check or access check SHALL post nothing.

#### Scenario: a proposal passed on
- GIVEN `alice`, signed in with token `u`, who can read `o/a`, and a fake object that answers `/write` with 409 `{"error": "taken"}`
- WHEN she posts `/title` `" Export CSV "`, `/body` `""` and `/nodes` `["a", "a"]`
- THEN the answer is 409 `{"error": "taken"}`, the object was asked once, on `/write`, with `login` `alice`, `token` `u`, `kind` `propose`, `title` `Export CSV` and `nodes` `["a"]`, and GitHub was asked nothing beyond the access check

#### Scenario: a decision passed on
- GIVEN the same, and a fake object that answers `/write` with 200 and a body `b`
- WHEN she posts `{"version": "0123abcd", "value": "low", "decision": "cut"}` to `.../architecture/my%20e/decide`
- THEN the answer is 200 with body `b` and `content-type: application/json`, and the one post names `kind` `decide`, `id` `architecture/my e`, that `version`, `value` and `decision`

#### Scenario: a withdrawal passed on
- GIVEN the same, and a fake object that answers `/write` with 404 and an empty body
- WHEN she deletes `.../proposals/p1`
- THEN the answer is 404 with an empty body, and the one post names `kind` `withdraw` and `id` `p1`

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

#### Scenario: a real object's refusals
- GIVEN `alice`, who can read `o/a`, and a real `Replica` object over the fake GitHub
- WHEN she decides capability `x` and architectural constraint `e` at versions they no longer have, `nope` that neither `CAPABILITIES.md` nor `ARCHITECTURE.md` holds, and `x` on a tip whose specs don't parse, on one holding `docs/allowed_signers`, while GitHub fails the read of the default branch's ref and while it refuses the append; proposes a title whose slug the backlog, `resolved` or an open proposal holds, and one naming a component the graph doesn't hold; withdraws `bob`'s proposal, then her own twice; and answers `q` with a choice it doesn't offer and an item the backlog doesn't hold
- THEN each answer is the object's status and body: 409 twice, 422 twice, 422, 422 with `error` `signed`, 502 and 502; 409 with `error` `taken` each time, and 422; 404, then 200, then 404; and 422 with `error` `answer`, and 404 with `error` `unknown`; and only her own withdrawal is written

#### Needs
- web.worker.replica/writes-through-object
- web.worker.replica/write-refusals

#### Serves
- capability/propose-intent
- capability/review-capability-cost
- capability/follow-from-browser

### Requirement: reserved-paths <!-- base: 04959fbee46733da -->
The Worker SHALL redirect a request for `/app`, with a session or without, to `/app/`, and SHALL answer a request for any path its routes don't answer, `/_clm/*` among them, with a bare 404, reading no session and no asset, so the invited list is never served raw and the Worker holds no page of the site.

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

### Requirement: not-invited-page <!-- base: 82a29a2e98c22299 -->
The "not invited" page the callback answers to a login that isn't invited SHALL say that the web app is in early access, that CLM works fully without it by following the quickstart, linking to `/guide/quickstart`, and that the web app is an optional extra for convenience; it SHALL link to Request early access at `https://github.com/clunny/clm/issues/new?template=early-access.yml` and to the homepage at `/`. It SHALL still answer 403, say "not invited", and set no session cookie (`web/invited-users`).

#### Scenario: someone not invited
- GIVEN the invited list holds `clunny` and GitHub completes the authorization for `someone-else`
- WHEN the browser reaches the callback
- THEN the response is 403, sets no session cookie, and its HTML links to `https://github.com/clunny/clm/issues/new?template=early-access.yml`, `/guide/quickstart` and `/`, and says the web app is optional and in early access

#### Serves
- capability/learn-clm

### Requirement: asset-adapter <!-- base: 8c4f0569f42548bb -->
The Worker adapter's `fetch_asset` (`web.worker.platform/asset-responses`) SHALL pass the request's method, path, query and headers, but for `cookie` and `authorization`, to the `ASSETS` binding, and answer the binding's status, every header and the body's bytes. It SHALL answer 503 when there is no `ASSETS` binding, and 502 when the fetch fails or the body can't be read. It SHALL keep, for the isolate's life, only 200 answers to a `GET` with no query of a `/_clm/*` path, and answer a later such request for the same path from what it kept without asking the binding.

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

### Requirement: answers <!-- base: 7d19919b7cf2f1b1 -->
`POST /api/repos/{o}/{r}/backlog/{slug}/answer` with `/answer` SHALL, through `writes-to-object`, append an `answered` record with `item` `{slug}`, percent-decoded, and that answer to `clm/ledger/<login>/web` through the Git Data API with the requester's own token, making no other write to the repository, and answer 200 with the record's `id`. It SHALL answer 422 with `{"error": "answer"}`, writing nothing and asking neither GitHub, beyond the session, nor the object, for a body with no string `/answer`; any other refusal SHALL be the object's (`web.worker.replica/writes-through-object`). The implementation response SHALL carry `answers`: the implementation view's (`replica/open-answers`), in its order, each with `mine` added, whether the writer is the requester's `<login>/web`. The route SHALL make the access check every API route makes.

#### Scenario: answered
- GIVEN a signed-in user `alice` who can read `o/a`, whose backlog holds question `q` with choices `yes` and `no`
- WHEN she posts `/answer` `"yes"` to `/api/repos/o/a/backlog/q/answer`
- THEN the writes made are exactly a blob, a tree, a commit and a ref update on `clm/ledger/alice/web`, with her token, the new `records.jsonl` ends with an `answered` line naming `q` with answer `yes`, and the answer carries its `id`

#### Scenario: no answer
- GIVEN the same
- WHEN she posts to `q`'s route `{}`, `{"answer": 1}` and `{"answer": null}`
- THEN each answer is 422 with `error` `answer`, GitHub, beyond the session, and the object were asked nothing, and nothing is written

#### Scenario: open answers
- GIVEN an implementation view whose `answers` are `a1` by `alice/web` and `b1` by `bob/web`
- WHEN the implementation response is built for `alice`
- THEN `answers` lists `a1` with `mine` true, then `b1` with `mine` false

#### Serves
- capability/follow-from-browser
- architecture/git-is-truth

#### Needs
- ledger/answer-records
- replica/validate-answers
- replica/open-answers
- web.worker.replica/writes-through-object

### Requirement: invited-sign-in <!-- base: 6950d0c1f0104399 -->
The Worker SHALL complete sign-in only for the GitHub logins in the invited-users list it reads through its `ASSETS` binding from `/_clm/invited.json`, which `web`'s build writes (`web/invited-users`), comparing logins without regard to case. For anyone else it SHALL show a "not invited" page and set no session cookie, and every API route SHALL answer 401 without a valid session: one whose login the list no longer holds, and one sealed before sessions kept a login, are not valid. With no list nobody SHALL get in. When GitHub doesn't name the user it SHALL answer 502 with a link back to `/auth/login` and no session cookie.

#### Scenario: invited
- GIVEN the list holds `clunny` and GitHub completes the authorization for `clunny`
- WHEN the browser reaches the callback
- THEN the response sets the session cookie

#### Scenario: not invited
- GIVEN GitHub completes the authorization for `someone-else`
- WHEN the browser reaches the callback
- THEN the response is the "not invited" page, sets no session cookie, and a following request for the repo list answers 401

#### Scenario: no longer invited
- GIVEN a session of `someone-else`, whom the list doesn't hold
- WHEN it asks for the repo list, and again once the list holds `someone-else`
- THEN the first answers 401 and the second 200

#### Scenario: no list
- GIVEN a Worker whose assets hold no `/_clm/invited.json`, and GitHub completing the authorization for `clunny`
- WHEN the browser reaches the callback
- THEN the response sets no session cookie

#### Serves
- architecture/invite-only-web

## REMOVED Requirements

### Requirement: no-scheduled-event <!-- base: d7b14f2e391c8208 -->
