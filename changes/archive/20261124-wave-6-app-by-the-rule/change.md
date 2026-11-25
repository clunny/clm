---
status: done
resolves: []
---

## Intent

Wave 6 of phase 5 of `backlog/a-passing-check-proves-less-than-the-spec-claims.md`, its branch B of two: the web app's and the site's components, `web`, `web.views`, `web.client-live` and `web.site` (the app under `web/`, its views under `web/src/views/`, its live socket and push under `web/src/live/`, and the site under `web/site/`), are each brought, in one pass and one delta, to the rule for a requirement (`requirements-state-what-the-contract-shows`), with a small delta of `ci` from which one requirement moves. Each requirement now states what its component shows: `web.views` and `web.client-live` what they draw and do for what they are handed, `web` what the built app does in a browser against the Worker (its routes, bodies, error texts, widths and every write's outcome, moved in from `web.views` where they were stated as the views'), and `web.site` what the built site serves. Scans of source text and of files' lines are cut or restated as what a program reads (`npm config`, `wrangler`'s own configuration reader, the built tree, what `clm affected --readers` selects), and the faults the pass found are mended by each component's build. Branch A, `20261124-wave-6-worker-by-the-rule`, holds the Worker's components and lands first; this branch is rebuilt on it.

## What the pass found

Each component was read against the rule and the code by one reader, then drafted by one drafter who checked each of the reader's entries against the code, with a check and up to three correction rounds. The builds mend, worst first:

- **The app's graph and backlog pages throw against the real Worker** (`web`, `web.views`): `main.ts`, `home.ts` and `backlog.ts` read the snapshot's `repo`, which the Worker doesn't send. Branch A's `web.worker/snapshot-names-its-repo` adds it; `web/graph-in-the-browser`, `backlog-in-the-browser`, `web.views/propose-from-backlog` and `home-waiting-on-you` Need it, and the fixture keeps `repo`, now the real shape.
- **A view that throws while drawing says "retrying", and nothing retries** (`web`): every non-HTTP error is classed `retry`. It now says the view couldn't be drawn from what CLM answered; `retry` is kept for offline and 5xx.
- **Every 4xx but 401 says the repo isn't readable** (`web`): 403 and 404 say so, any other 4xx now says what failed with its status, and a deleted spec branch's review (404) says the branch is no longer open, linking the PR list.
- **A failed spec-branch list is drawn as no branches** (`web`): the failure was caught as `[]`. The PR list and Home now say the spec branches couldn't be loaded, until a refresh reads them.
- **A write's 401 shows no sign-in page, or one a refresh draws over** (`web`): Approve says "Signed out. Sign in again to approve." and shows nothing; the Notify switch says "Not turned on: try again."; and after any other write's 401 the next refresh draws the old view over the sign-in page. Every write's 401 now shows the sign-in page, and no refresh draws over it (`app-writes`).
- **The homepage footer's Web app link shows the site's 404 page** (`web.site`): it has no `target="_self"`, so VitePress's router takes it. Every `/app/` link on the site now loads the app.
- **A turn-off whose browser unsubscribe fails reads on** (`web.client-live`): the Worker has dropped the endpoint and the repo is forgotten, but the switch says "Not turned off: try again." It now reads off.
- **A live socket whose `open` throws is never opened again** (`web.client-live`): the URL is already set, so a later `want` returns early. It now retries as after a close.
- **A notification click whose window's `focus` rejects opens no window** (`web.client-live`); it now opens one.
- **Post slugs holding `&`, a space or `<` are written raw** into the feed's `<link>` and `<guid>`, `llms.txt`, the canonical and alternate links, JSON-LD, the blog's pages and the sitemap (`web.site`). Every URL the site writes for a post now carries its slug percent-encoded, through one call.
- **A description holding `"`, `<` or `&` is written unescaped** into `<meta name="description">` (`web.site`): VitePress writes the tag raw; the site's own head now writes it, escaped.
- **A guide page's or a post's link into the repository is left relative**, and a post's `` `[x](nope.md)` `` in a code span fails the build as naming no post (`web.site`). Such links now point to the file on GitHub's `main` as the Markdown copies do, never failing the build, and the guide's and the posts' rewrites skip code spans alike.
- **The star count takes `-5`, `12.5` and `1e3`** (`web.site`): only a whole number of 0 or more is now used; anything else shows no count and warns.
- **The guide's sidebar labels a page `README.md` doesn't link by its file name**, and skips a README link carrying an `#anchor` (`web.site`).
- **A passed claim reads `lapses in now`** (`web.views`); it now reads `lapsed`. **The Architecture table's first column is headed `Requirement`**, the old word for a constraint; it now reads `Constraint`. **The inspector labels every Contract, Design or frontmatter change `modified`**, an added Design included; it now labels each as the review does.
- **A stored `clm-push` that is a JSON array counts as remembered repos** (`web.client-live`), so a renewal posts to `apiBase('0')`. It now counts as nothing.
- **Tests that prove less than they cite**: `web.site`'s doc-readers test copies `clm affected`'s matching, so both `doc-readers-marked` scenarios are proven by a copy; the requirement is restated as what `clm affected --readers` selects, and the test is re-cited only, since building `clm` in `web.site`'s land check is slow and reads files its graph doesn't connect (`land-checks-stay-fast`); the proof by the real code is the root's, in wave 7 (Open questions). Markers on `describe` blocks in `web`, `web.views` and `web.client-live` let a single test be deleted unseen; each test that proves a requirement now carries its own. Site tests pin the guide's wording (they now read it from the file), assert no warning where one is stated, and fail a draft clash on another post; `live.test.ts` advances 3 minutes where the scenario says 5.
- **Rules held twice, given one provider**: the `.npmrc` line rule (in `web` and `web.site`) is npm's own reading now, both requirements read alike; which backlog kinds wait on a person (`web.views`' two copies, one constant); a Worker's configuration read through `wrangler`'s own reader (`web/worker-deploy`, `web.site/site-worker`).
- **Dead code**: `web/wrangler.toml`'s KV binding and `web/README.md`'s KV setup line (after branch A drops the adapter); the line-reading helper of `npmrc.test.ts`; `renderReview`'s optional `onComment` and `onRequest`, whose fallback no caller reaches.
- **Comments that name what doesn't exist**: `web/<slug>` for `web.views`' and `web.client-live`'s requirements in `main.ts`, `sw.js`, `vite.config.ts` and the views; `web/oversized-node`; `types.ts`' file paths; `wrangler.toml`'s `web/no-cron-trigger`.
- **Needs against `depends_on`**: `web.client-live`'s three Needs on `web.worker` (it calls no Worker route; `web` reaches them and takes them up), `web/app-at-app-path`'s on `ci/site-deploy`, `installable`'s and `retire-root-service-worker`'s on `web.site`, and `web.site/guide-pages`' on `ci/web-checks` ordered nothing and go.
- **Found and filed, outside this change's requirements**: the capability harness builds the site with neither `GITHUB_STARS` nor `GITHUB_API`, so every capability run asks GitHub (`backlog/the-capability-harness-builds-the-site-asking-github-on-every-run`, `capability`).

## Cuts

### Under an earlier yes, or moves (nothing lost)

- **`ci/early-access-form` moves into `web/sign-in-early-access`** (the human's yes, 2026-11-24): the clause that `.github/ISSUE_TEMPLATE/early-access.yml` exists, asks one question and asks for no email, and the template's "CLM works fully without it", proven by a new `web/src/early-access.test.ts` reading the template under a `clm:reads` marker; scenario `the form` by name. Its sentence "A maintainer grants a request by adding the login to `web`'s invited list; the form grants nothing" goes to `web`'s Contract. `ci`'s Design names the form's rule as `web`'s (the file stays `ci`'s). The Rust test `community_files.rs::early_access_form_asks_one_question_and_no_email` is deleted with its file and `mod` line.
- **`web/live-binding`, `replica-binding` and `no-cron-trigger` merge into `web/worker-deploy`**: the same bindings, migrations and empty crons, as what `wrangler` reads for a deploy, "exactly", which drops KV. Scenarios `binding` (one for both) and `the trigger is cleared` by name. Tests re-cited; none deleted. The gate lists `web/replica-binding` as gone, not merged, because a merge it counts needs the merged statement's sentences and `replica-binding`'s `binding` scenario folds into `live-binding`'s; its content, `REPLICA` and its migration, is in `worker-deploy`.
- **`web.views`' requirements that state `web`'s code move to `web`** with their scenario names (the session's decision): Approve's 422 `signed` reload and `became signed` to `approve-by-branch`; the comment and request-changes posts and `branch moved` to `review-in-the-browser`; the proposal and withdrawal posts to `backlog-in-the-browser`; the sidebar order and `its own view` to `cost-views-in-the-browser`; the widths, the 2×2 grid and the More menu to `mobile-layout`; the inspector's drawer and `desktop` to `graph-in-the-browser` and `mobile-layout`; the header's component count and the closed inspector's note to `web`. `web.views` keeps what it draws for what it is handed, through callbacks.
- **Folds under one name**: `review-in-the-browser`'s `became signed` into the moved `approve-by-branch` `became signed`; `web.views`' `desktop` into `graph-in-the-browser`'s `the inspector on desktop` (the name stays as `graph-fills-height`'s); the 409 case of `a request the Worker doesn't take` into the moved `branch moved`.
- **`web/honest-errors`' test of `api.ts` moves to `web.views`** (`errors.test.ts::falls back to the kept copy, but not for a live refresh or a preparing 503` → `web/src/views/api.test.ts`, citing `web.views/worker-requests`); `honest-errors` Needs it, so `web`'s build deletes the old copy after `web.views`' has added the new.
- **Source scans under wave 1's yes on per-crate retired-word scans** (`web.views`): `says-architectural-constraint`'s `the shipped app` (the root's `clm/retired-terms` holds the term; the on-screen heading is restated and mended); `says-component`'s scan sentence and its "keep their names" sentence (lost: a check that a new string doesn't say the old word for component, a plain English word nothing at the root lists); `queue-states`' and `implementation-view`'s greps for a state and fields the queue no longer sends (behaviour tests keep the views from reading them). The tests `says-constraint.test.ts::the shipped app: no file under web/src holds the old term` and `says-component.test.ts::no string literal under web/src holds an old word` carry only their `describe`'s marker, so no list line can name them; the session deletes them before the build run (Build order; `queue-states.test.ts`' scan has a marker and is listed).
- **Duplicates of what another requirement states** (`web.views`): `spec-branch-review`'s "requests no `/pulls` route" (`web/review-in-the-browser` states it); `answer-buttons`' route and status texts (`web/answer-from-app`); "tapping another SHALL replace it" (`ledger/answer-records`' fold). (`web.site`): `blog-index`'s "the nav of every page SHALL link to it as Blog, after Guide" (`nav-web-app`).
- **Design sentences that restate a requirement, place tests, or record a finished move** (no SHALL): `web`'s Deploy bullets, the KV namespace and its Setup bullet, the one-Worker-to-two paragraph, most of PWA, Notices, Loading and Look, and the two test-placement sentences; `web.views`' module-move history and Tests paragraph; `web.site`'s fourteen sentences now each a requirement's, its sentence on how a test selects the GitHub button, and the posts' `install-repository` sentence.
- **`app-path.spec.ts:93`'s assertion that `invited.json` is `['clunny']`**: a line, not a test; line 115's check from the spec stays.
- **Source-text assertions replaced, not cut** (`web.site`): the walk of `web/site/` for the repository's name, `config.ts`'s `SITE_OUT` literal, `.npmrc`'s lines and `wrangler.toml`'s lines; each requirement is restated as behaviour and its test rewritten under the same name.

### Needing the human's yes now

`web`:
- **`no-install-scripts`' scenario `a later line turns scripts back on`**, and its test `web/src/npmrc.test.ts::a later line can turn scripts back on`. Its subject is the test's own line reader, which goes: npm's own reading replaces it. Lost: nothing about the app. Saves a scenario and a test. The test has no marker of its own (only its `describe`'s), so no list line can name it; the session deletes it before the build run (Build order).
- **`app-at-app-path`'s two sentences about tests** ("The app's end-to-end tests SHALL open it at `/app/`", and the harness's `signIn` opening `/app/` wherever the callback sends the browser), and its move history. Lost: no requirement says `capabilities/web/`'s `signIn` copes with either callback target; that mattered only during the move, now done. No test goes. `docs/decisions/capability-tests-follow-rules.md` cites it as an example, now history.
- **`web/e2e/live.spec.ts::a silent drop: a socket that stops answering ping is closed and another opened`**. It proves `web.client-live/live-updates`' `a silent drop` under a `notice-refresh` marker, which states nothing of `ping`; `live.test.ts`'s test of the same name proves it, and the e2e `pong` comes from the mock, not `web.live`'s object. Lost: one browser run of the ping timing over a real `WebSocket`. Saves a 61-second clock run. No marker of its own, so no list line; the session deletes it before the build run (Build order).
- **`web/src/readme.test.ts::carries the App's manifest`** and the Design sentence that `web/README.md` carries the App's manifest and a test checks it. Lost: a check that the README's manifest matches `web.worker`'s Design (branch A proposes cutting the Design's copy too). Uncited, so no list line; the session deletes it before the build run (Build order).

`web.views`:
- **`says-new-state-words`' scenario `the shipped app` and three sentences**: no file under `web/src/` holds the old status name or the old backlog-kind name; the flag type's doc comment reads `A workaround flag open on a capability`; a test builds either old word from pieces. File text, a doc comment, and a rule about tests. Lost: the root's list holds the kind's `kind:` form only in backticks, so an unquoted `kind: <old>` in `web/src/` would pass. Saves 3 sentences and 2 tests. The two tests, `says-new-state-words.test.ts::the shipped app: no file under web/src holds the first old name as a word, nor the second after clm or as a kind` and `::the Flag type is documented as a workaround flag open on a capability`, carry only their `describe`'s marker, so no list line can name them; the session deletes them before the build run (Build order).

`web.site`:
- **`agent-markdown`'s "None of them SHALL be committed.", its scenario `not committed`, and its test** `seo.spec.ts::no built agent file is committed`. Its subject is which files git tracks; the build writes every copy after VitePress copies `public/`, so a committed copy is overwritten anyway. What could reach a visitor, a stale `public/guide/<x>.md` with no guide file behind it, is kept as output in `each copy is its file`. Lost: a check that nobody commits a file the build overwrites. The decision `site-found-by-search-and-agents` keeps "never committed".

`web.client-live`: none.

## What the requirements no longer state

`web`:
- `app-at-app-path`: "The app's end-to-end tests SHALL open it at `/app/`."; "the harness's `signIn` SHALL open `/app/` once the callback has set the session, wherever the callback sends the browser".
- `no-install-scripts`: "`web/.npmrc` SHALL set `ignore-scripts=true`, on the last line that sets the key"; restated as what npm takes from the package's settings.
- `live-binding`, `replica-binding`, `no-cron-trigger`: "`web/wrangler.toml` SHALL bind … / SHALL hold `crons = []` … The key SHALL stay, empty"; restated in `worker-deploy` as what `wrangler` reads, "the key stays" as triggers holding an empty list of crons.
- `approve-by-branch`: "on any other failure it SHALL say "Not approved" with the status" (now per status: the 422 `signed` case, `Not approved (<status>).`, and a 401 or an unsent write under `app-writes`); "It acts on the branch review `spec-branch-review` loads, so the two build together" (a build-order note).
- `notice-refresh`: "none during an approval" (now none during any write, `app-writes`).
- `answer-from-app`: "a 401 shows the sign-in page, as for the app's other writes", and "when the request isn't sent that it is offline" (both now `app-writes`, which makes the first true of Approve too).
- `fresh-after-write`: `crypto.randomUUID()` by name (now "a random UUID new for each write").
- Design (no SHALL): the sentences under Cuts.

`web.views`:
- `signed-approvals`: "An approve answered 422 with `error` `signed` SHALL load the review again from the Worker." (to `web/approve-by-branch`).
- `request-changes`: the post of `{"fingerprint", "text"}`, the reload on success and the 409's message and reload (to `web/review-in-the-browser`); "in the Approve bar on phones".
- `comment-on-review`: the post of `{fingerprint, target, text}` and the reload (to `web/review-in-the-browser`).
- `propose-from-backlog`: the proposal's post, its reload, the 409 and 422 texts, and Withdraw's `DELETE` and reload (to `web/backlog-in-the-browser`).
- `answer-buttons`: the post route and reload, "A 422 or a 404 SHALL show why", and "tapping another SHALL replace it".
- `approve-button`: "SHALL send one approve request per tap and none while one is in flight, and once the Worker accepts it SHALL read Approved"; reworded to the callback.
- `spec-branch-review`: "The app SHALL request no `/pulls` route of the Worker."; "loaded from the Worker's branch review".
- `capabilities-list`, `architecture-list`: "At 900 px and wider it SHALL be a table …; narrower, one card per capability" (both are drawn here; which shows is `web`'s); `architecture-list`: "SHALL be a view of its own, listed after Capabilities in the sidebar as Architecture and in the tab bar as Arch" (to `web`; the tab-bar half was never true).
- `decide-buttons`: "in a 2×2 grid below 900 px" (to `web/mobile-layout`) and "never the copy kept for offline use" (to `web/cost-views-in-the-browser`, which reloads a decision's report from the Worker); `architecture-decide-buttons`: "laid out as there" and the same.
- `inspector-overlay`: "SHALL open over the graph, never narrowing it: on desktop a drawer … on phones a bottom sheet." (to `web/graph-in-the-browser` and `mobile-layout`, the drawer's "at most 640 px wide and 90% of the window" kept); "on selecting the selected component again, which also clears the selection" (now `graph-view`'s).
- `says-architectural-constraint`: its first sentence, restated for the screen as the `Constraint` heading.
- `says-component`: the scan sentence and the "keep their names" sentence; the header's component count and the closed inspector's note (to `web`); "the backlog's component … picker" (there is none).
- `says-new-state-words`: three sentences.
- `zoom-at-pointer`: "The zoom's limits SHALL be unchanged." (now "between 0.3 and 3 times").
- `home-waiting-on-you`: "The number of entries in Review and Answer SHALL be the count the sidebar and tab bar badge Home with." (now the count it gives its caller; the badging is `web/home-first`'s).
- `implementation-view`: "showing no `waiting` state and reading no `waiting_on` field"; "never from `needs_pending`"; "each with the state `queue_from_refs` gives it".
- `queue-states`: "SHALL know exactly the queue states", reworded to what it labels, orders and shows.

`web.client-live`:
- `live-updates`: "hold one **WebSocket**" (now one socket made by the `open` it was given; the `WebSocket` is `web`'s).
- `notify-switch`: "once a service worker is registered, the browser has `Notification` and `PushManager`, and the Worker answers a push key", and "what `GET /api/repos/<o>/<r>/push?endpoint=<endpoint>` answers" (to `web/push-in-the-browser`); "outside its label" (now "whose tap doesn't toggle the switch").
- `push-renewal`: "in `localStorage`" (now the env's storage; `localStorage` and a throwing read move to `web`).
- Contract: where tests sit, and which `web/vite.config.ts` builds into `sw-push.js` (to `web`).

`web.site`:
- `agent-markdown`: "None of them SHALL be committed."
- `blog-index`: "the nav of every page SHALL link to it as Blog, after Guide".
- `install-repository`: "and no other file under `web/site/` SHALL name it, except the blog's posts …" (restated over the built files).
- `site-no-install-scripts`: "`web/site/.npmrc` SHALL set `ignore-scripts=true`" (restated as what npm reads there).
- `site-worker`: "`web/site/wrangler.toml` SHALL declare …" (restated as what `wrangler` reads and serves).
- `guide-pages`: "with a sidebar listing them in the order `docs/guide/README.md` gives" (restated in full).
- `click-events`: "with the element's text" (restated as the link's text less a trailing arrow).
- `doc-readers-marked`: "Each test file under `web/site/` that builds the site from, or reads, this repository's own `docs/guide/` or `docs/spec-format.md` … SHALL hold a `clm:reads` marker … whose glob matches every such path it reads" (its subject was the test files' text; restated as what `clm affected --readers` selects when a guide page or `docs/spec-format.md` changes; both scenarios keep their names and text).

`ci`:
- `early-access-form`, whole, moved to `web/sign-in-early-access` (its "grants nothing" sentence to `web`'s Contract).

## Narrowings

- `web/fresh-after-write` no longer names `crypto.randomUUID()`; its scenario still checks the UUID's form.
- `web.views/architecture-list`'s "in the tab bar as Arch" was never true; `web/mobile-layout` states the More menu's `PRs`, `Capabilities` and `Arch.`.
- `web.views/approve-button` says "not again until the review is drawn anew" where it said "none while one is in flight": the code's behaviour.
- `web.views/decide-buttons` `moved` no longer says the page on screen is `x` at `w`, and `inspector-overlay` `an item from the inspector` only that its link leads to the item's page: the redraw and the navigation are `web`'s, which states them. `inspector-overlay` keeps only closing and the item links, under its old slug.
- `web.client-live/notify-switch`'s "and none otherwise" is now "hidden until the env gives both a push manager and a key"; what stands behind a null push manager is `web/push-in-the-browser`'s. `push-renewal`'s delete of the old endpoint is "one `DELETE`, not sent again whatever it answers", what the code does.
- `web.site/install-repository`: a source file that names `clunny/clm` and reaches no built file is no longer refused; only built output is. `site-no-install-scripts`: an environment or command line that sets `ignore-scripts=false` is outside the clause (npm puts both above the project's settings). `click-events`: the label is the link's text less a trailing `↓` or `→`, as the code and its test already were; and it widens to Read the quickstart and the nav's site title, and plainly leaves out a `mailto:` link. `copy-command-event` widens to a code block or inline code anywhere, as the code does.
- Every other reword widens; each outcome change the gate lists is a widening or the same behaviour stated exactly.

## Build order

From the Needs, each build taking the requirements whose Needs are current:

1. Branch A's builds first: `web.worker`'s `snapshot-names-its-repo` and `cache-is-disposable` (its build drops the KV adapter; `deploy.rs::readme_records_how_to_redo_the_setup` and `deploy.rs::binds_the_kv_namespace` are deleted whole by the session on a small code branch before the build run, as branch A's change says), and the reworded `web.worker/live-socket`, `live-class-export`, `replica-class-export` and `web.live/hub`.
2. `web.views` (its `propose-from-backlog` and `home-waiting-on-you` Need `web.worker/snapshot-names-its-repo`), `web.client-live` (no Needs), `web.site` (no Needs of another wave component; `doc-readers-marked` Needs the current `check.affected/affected-readers`), in any order.
3. `web`: the Needs order only those of its requirements that Need `web.views/worker-requests`, `signed-approvals` and others, `web.client-live/notify-toggle` and `push-renewal`, `web.site/site-worker`, or branch A's requirements above, after those builds. Its other requirements (`review-in-the-browser`, `cost-views-in-the-browser`, `fresh-after-write`, `live-refresh` and more) have no Need on `web.views`, so `clm next` offers `web` beside `web.views`; running all of `web` after `web.views` is the session's build-session order, not a Need's, as for `ci` below. It drops `web/wrangler.toml`'s `[[kv_namespaces]]` and `web/README.md:50`'s KV line after `web.worker`'s build has deleted the two tests that read them, and deletes the moved `errors.test.ts` test after `web.views`' build has added its copy.
4. `ci`, after `web` (no Need orders it; the session runs it then): it deletes `community_files.rs` and its `mod` line in `crates/clm/tests/ci_suite/main.rs`, so `web/src/early-access.test.ts` already proves the form when `ci`'s test of it goes, and the form is always proven.

No component builds twice on this branch.

**Steps no requirement states.** No backlog item carries these; each is named here for the session or for its build:
- The session, after the human's yes and before the build run, on small code branches: deletes the seven tests on the cut lists that carry only a `describe`'s marker, so no `deleted-tests.txt` line can name them, before any build puts a marker on each test (a test that gains a marker and is then deleted is refused at the land). `web.views`' four: `says-component.test.ts::no string literal under web/src holds an old word`, `says-constraint.test.ts::the shipped app: no file under web/src holds the old term`, and `says-new-state-words.test.ts`' two (`the shipped app: no file under web/src holds the first old name as a word, nor the second after clm or as a kind` and `the Flag type is documented as a workaround flag open on a capability`). `web`'s three: `npmrc.test.ts::a later line can turn scripts back on`, `live.spec.ts::a silent drop: a socket that stops answering ping is closed and another opened`, and `readme.test.ts::carries the App's manifest`. The build material says these deletions have landed.
- The session runs `web`'s build before `ci`'s, so the early-access form is always proven (step 4 above).
- `web`: delete `npmrc.test.ts`' line-reading helper; drop `web/README.md:50`'s KV line; fix the comments naming `web/<slug>` for `web.views`' and `web.client-live`'s requirements and `wrangler.toml`'s `web/no-cron-trigger`; add `yaml` as a devDependency (`npm install --ignore-scripts -D yaml`) for `early-access.test.ts`.
- `web`, `web.views` and `web.client-live`: one marker on each test that proves a requirement and none on a `describe` block (stated in `web`'s Design, not by a requirement), so the re-cite of every test each build makes anyway leaves none behind a block's marker.
- `web.views`: the comments naming `web/<slug>`, `web/oversized-node` and `types.ts`' file paths; `renderReview`'s `onComment` and `onRequest` made required, the fallback dropped.
- `web.site`: none beyond its requirements; the doc-readers test is re-cited only, and builds no `clm`.

## Decisions

The human's (2026-11-24):
- **Two branches.** A: `web.worker`, `web.worker.platform`, `web.worker.replica`, `web.worker.review` and `web.live`, with small deltas outside the wave where a clause moves or a provider's call is needed. B, this one: `web`, `web.views`, `web.client-live` and `web.site`, with a small delta of `ci`. A lands first; B is rebuilt on A.
- **No release; wave 6's branches are approved in a session** with `clm approve`.
- **Early access**: `ci/early-access-form` moves into `web/sign-in-early-access`, the clause that the form exists, asks one question and asks for no email, proven by reading `.github/ISSUE_TEMPLATE/early-access.yml`; `ci`'s requirement is removed and its scenario name moved; `web.worker/not-invited-page` keeps only its link. The template's "CLM works fully without the web app" is kept in the clause.

The session's, that this branch carries:
- The snapshot names its repo (branch A's `web.worker/snapshot-names-its-repo`); `web`'s and `web.views`' requirements that read it Need it, and `web/src/views/fixtures.ts` keeps `repo`.
- A test behind a `describe` marker is a CLM defect, filed by the session (`backlog/a-marker-above-a-describe-block-counts-once-for-coverage-while`); inside the wave, the builds of `web`, `web.views` and `web.client-live` put a marker on each test that proves a requirement. The seven `notify.test.ts` tests re-cited from `notify-switch` to `notify-toggle` keep their names and a marker each, so no list line names them.
- Approve on a 401 shows the sign-in page, as every other write does. A failed spec-branch list says what failed; a 4xx other than 401, 403 and 404 says what failed; a deleted spec branch's review says the branch is gone.
- KV is dead code: `web` drops the `wrangler.toml` binding, after `web.worker`'s build drops its adapter and the two `deploy.rs` tests that read the binding (`worker-deploy` Needs `web.worker/cache-is-disposable`).
- `web`'s file-text requirements are restated as behaviour: `worker-deploy` as what `wrangler`'s own configuration reader takes from `web/` (not `wrangler deploy --dry-run`, whose output names neither the migrations nor the triggers), and `no-install-scripts` as what `npm config get ignore-scripts` reads, as `web.site`'s do.
- `web.views`' requirements that state `web`'s code move to `web` with their scenario names; `web.views` keeps what it draws for what it is handed.
- `web.site/doc-readers-marked` stays `web.site`'s until wave 7, restated as what `clm affected --readers` selects; its test builds no `clm` in the land check (`land-checks-stay-fast`) and is re-cited only, and the proof by the real code is the root's, for wave 7. Every `/app/` link on the site loads the app. Copy reporting takes the code's wider reading (inline code anywhere). Post slugs are escaped wherever the site writes them, mended by the build. `robots.txt` is unchanged.
- `ping`→`pong` is `web.live/hub`'s to state (branch A); `web.client-live/live-updates` relies on it and proves its side against a fake socket. A gone push endpoint is branch A's fix (`web.worker.replica`), which `web.client-live`'s Design names as why a renewal's failed `DELETE` isn't resent.
- After this branch's architecture audit: a decision's report is loaded again from the Worker, never the copy kept for offline use, restated in `web/cost-views-in-the-browser`; the inspector drawer's "90% of the window" is kept in `web/graph-in-the-browser`; `.github/ISSUE_TEMPLATE/early-access.yml` stays `ci`'s file, and `web`'s Design says a change to it that turns `web`'s test red is filed as a `ci` item (`path-ownership`); the seven tests that carry only a `describe`'s marker are deleted by the session before the build run (Build order).
- The capability harness's request to GitHub is filed as a `bug` on `capability`, not fixed here.
- Change IDs: A `20261124-wave-6-worker-by-the-rule`, B `20261124-wave-6-app-by-the-rule`.

## Open questions

- **`web`'s spec is about 17,600 tokens of its 12,000** (from 15,143), for `backlog/the-brakes-on-the-spec-s-growth-only-advise`: the Design was cut by about 1,300 tokens, but the newly stated write outcomes, badges, offline writes and error cases, and the clauses moved in from `web.views` and `web.client-live`, add more. Nothing else can be cut without losing a stated behaviour. The session's recommendation: no split in this wave; raise the budget or split later (the `*-in-the-browser` requirements are about half the spec). `web.site` is about 14,700 of its 12,000 (from about 12,000), nearly all newly stated scenarios; `web.views` is about 19,300 of its 20,000.
- **`web.site/doc-readers-marked`'s proof by the real `clm affected --readers`** is the root's, for wave 7: until then its test copies `clm-affected`'s matching, so a change to that matching can leave both scenarios passing while the real selection differs.
- **What no run here shows**: Cloudflare answering `ping` with `pong` (`web.client-live/live-updates` relies on it; branch A names it); `web.site/site-worker`'s 404 under `wrangler dev` if `workerd` can't run offline under `ignore-scripts`, in which case the build flags `clm workaround web.site/site-worker` and serves the stand-in. `worker-deploy`'s test reads `wrangler`'s `unstable_` configuration API: a lockfile bump can change it, and the test then fails loudly, never passes falsely. Accepted.

## Also

- One line is added to branch A's `changes/20261124-wave-6-worker-by-the-rule/deleted-tests.txt`: `DELETE crates/clm-web/src/proposals.rs::slugs_as_clm_defect_does`. The test cites `web.worker/proposals` and was on A's approved cut list in plain words (the Worker's copy of the slug rule goes; `proposals`' scenario `the slug` proves slugs end to end). A's assembly dropped its line because it named the test `proposals.rs::tests::slugs_as_clm_defect_does`, with a `tests::` prefix the scanner doesn't use, and took it for uncited.
