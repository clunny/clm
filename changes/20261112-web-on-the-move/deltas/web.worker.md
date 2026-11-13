---
component: web.worker
base: { contract: "2f23e69b2aa50f02" }
---

## MODIFIED Contract
The Cloudflare Worker behind `web`: it is deployed with the browser app's static assets under `/app/`, on the app's routes of `clm-app.dev`, while the public site (`web.site`) is a Worker of its own at the root (`web/site-served-directly`, `site-worker-of-its-own`), and answers what `web/wrangler.toml`'s `run_worker_first` routes to it: `/app`, which it redirects to `/app/` (`reserved-paths`), `/auth/*` (GitHub sign-in, which returns to `/app/`), `/api/*` (JSON for the app: the user's repos, a repo's snapshot, its live state, a spec branch's review, approve, comments and change requests on a spec branch, proposals for the backlog, answers to backlog items, what each capability and each architectural constraint costs and a decision on one, the live socket and push subscriptions), `/webhook`, and `/_clm/*`, which it answers 404 so the invited list is never served raw (`reserved-paths`). The app calls nothing else, and every response shape it reads is this component's contract: changing one is a change here that `web` builds against. The snapshot's backlog items carry `body`, `priority` and `choices` (`replica/snapshot-backlog-text`), the implementation response carries `answers` (`answers`), and each capability and architectural constraint in the reports carries its entry's `text` (`effort/entry-text`). The Worker alone holds a user's token, passing it to the repo's `Replica` object only for a decision's, a proposal's, a withdrawal's or an answer's append, which keeps it only for that request; it writes to the repository only to append an `approved`, `subscribed`, `unsubscribed`, `proposed`, `withdrawn`, `answered`, `commented`, `changes_requested` or `decided` record to the user's web ledger (`per-writer-ledgers`, `push-subscriptions-in-ledger`, `web-proposals-as-ledger-records`, `web-answers-as-ledger-records`, `web-review-feedback-as-ledger-records`), and keeps only state that can be deleted at any time (`web-derived-cache`): caches, each repo's replica, which its `Replica` object (`web.worker.replica`) holds, derived from git and rebuilt from it when lost (`web-repo-replica`), and the open pages' sockets, which its `Live` objects (`web.live`) hold and which store nothing (`web-live-channel`). Its webhook passes each repo's events to the repo's `Replica` object, which recomputes the views, tells the `Live` object, and sends Web Push notifications to the subscriptions in the repo's ledgers. The views are computed by `replica` with the `model`, `check`, `orchestrator`, `view`, `ledger` and `effort` crates compiled to wasm, so the Worker shows what `clm` would report and decides nothing.

Three children answer parts of it beneath this component's router, session and access check: `web.worker.review` the spec-branch list, a spec branch's review, approve, comments and change requests; `web.worker.replica` the views and the writes that need them; and `web.worker.platform` the layer every route stands on. None changes a route or a response shape: what the app reads stays this component's contract.

## MODIFIED Requirements
### Requirement: writes-to-object <!-- base: 0e34d9ff37b23048 -->
The proposal, withdrawal, answer and decide routes SHALL, after their body check and the access check, make one post to the repo's `Replica` object's `/write`, and answer the status and body it answers, as it gave them, reading no view, no ledger and no spec file, and making no other request to the object or, beyond the access check, to GitHub. The post's body SHALL be the requester's `login` and `token` with: `kind` `propose` and the trimmed `title` and `body` and the deduplicated `nodes`; `kind` `withdraw` and the `id`; `kind` `answer`, the `item`, the slug percent-decoded, and the body's `answer`; or `kind` `decide`, the `id` `capability/<slug>` or `architecture/<slug>` with the slug percent-decoded, and the body's `version`, `value` and `decision`. When the object can't be reached, the route SHALL answer 503 `{"error": "unavailable"}`. A failed body check or access check SHALL post nothing.

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


## ADDED Requirements
### Requirement: answers
`POST /api/repos/{o}/{r}/backlog/{slug}/answer` with `/answer` SHALL, through `writes-to-object`, append an `answered` record with `item` `{slug}` and that answer to `clm/ledger/<login>/web` through the Git Data API with the requester's own token, making no other write to the repository, and answer 200 with the record's `id`. It SHALL answer 404 with `{"error": "unknown"}` for an item the replica refuses as `Unknown`, and 422 with `{"error": "answer"}` for a missing or non-string `/answer` or one the replica refuses as `Invalid` (`replica/validate-answers`), writing nothing. The implementation response SHALL carry `answers`: the implementation view's (`replica/open-answers`), in its order, each with `mine` added, whether the writer is the requester's `<login>/web`. The route SHALL make the access check every API route makes.

#### Scenario: answered
- GIVEN a signed-in user `alice` who can read `o/a`, whose backlog holds question `q` with choices `yes` and `no`
- WHEN she posts `/answer` `"yes"` to `/api/repos/o/a/backlog/q/answer`
- THEN the writes made are exactly a blob, a tree, a commit and a ref update on `clm/ledger/alice/web`, with her token, the new `records.jsonl` ends with an `answered` line naming `q` with answer `yes`, and the answer carries its `id`

#### Scenario: refused
- GIVEN the same
- WHEN she posts `/answer` `"maybe"` to `q`'s route, then `"yes"` to `/api/repos/o/a/backlog/nope/answer`
- THEN the first answers 422 with `error` `answer` and the second 404 with `error` `unknown`, and nothing is written

#### Scenario: open answers
- GIVEN an implementation view whose `answers` are `a1` by `alice/web` and `b1` by `bob/web`
- WHEN the implementation response is built for `alice`
- THEN `answers` lists `a1` with `mine` true, then `b1` with `mine` false

#### Serves
- capability/follow-from-browser
- architecture/per-writer-ledgers

#### Needs
- ledger/answer-records
- replica/validate-answers
- replica/open-answers
- web.worker.replica/writes-through-object
