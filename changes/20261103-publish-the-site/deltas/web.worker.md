---
component: web.worker
base: { contract: "11431665ae795b9f" }
---

## MODIFIED Contract
The Cloudflare Worker behind `web`: it is deployed with the browser app's static assets under `/app/` and the public site (`web.site`) at the root, which the assets answer without it (`web/site-served-directly`), and answers what `web/wrangler.toml`'s `run_worker_first` routes to it: `/app`, which it redirects to `/app/` (`reserved-paths`), `/auth/*` (GitHub sign-in, which returns to `/app/`), `/api/*` (JSON for the app: the user's repos, a repo's snapshot, its live state, a spec branch's review, approve, comments and change requests on a spec branch, proposals for the backlog, what each capability and each architectural constraint costs and a decision on one, the live socket and push subscriptions), `/webhook`, and `/_clm/*`, which it answers 404 so the invited list is never served raw (`reserved-paths`). The app calls nothing else, and every response shape it reads is this component's contract: changing one is a change here that `web` builds against. The Worker alone holds a user's token, passing it to the repo's `Replica` object only for a decision's, a proposal's or a withdrawal's append, which keeps it no longer than that request; it writes to the repository only to append an `approved`, `subscribed`, `unsubscribed`, `proposed`, `withdrawn`, `commented`, `changes_requested` or `decided` record to the user's web ledger (`per-writer-ledgers`, `push-subscriptions-in-ledger`, `web-proposals-as-ledger-records`, `web-review-feedback-as-ledger-records`), and keeps only state that can be deleted at any time (`web-derived-cache`): caches, each repo's replica, which its `Replica` object (`web.worker.replica`) holds, derived from git and rebuilt from it when lost (`web-repo-replica`), and the open pages' sockets, which its `Live` objects (`web.live`) hold and which store nothing (`web-live-channel`). Its webhook passes each repo's events to the repo's `Replica` object, which recomputes the views, tells the `Live` object, and sends Web Push notifications to the subscriptions in the repo's ledgers. The views are computed by `replica` with the `model`, `check`, `orchestrator`, `view`, `ledger` and `effort` crates compiled to wasm, so the Worker shows what `clm` would report and decides nothing.

Three children answer parts of it beneath this component's router, session and access check: `web.worker.review` the spec-branch list, a spec branch's review, approve, comments and change requests; `web.worker.replica` the views and the writes that need them; and `web.worker.platform` the layer every route stands on. None changes a route or a response shape: what the app reads stays this component's contract.

## ADDED Requirements

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

## REMOVED Requirements

### Requirement: site-behind-sign-in <!-- base: a4a230041e5e442c -->
