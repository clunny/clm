---
title: Web
status: active
owners: ["@clunny"]
depends_on: [web.worker, web.client-live, web.views, web.site, view, effort]
paths: ["web/*", "web/e2e/**", "web/public/**", "web/src/*", "web/test/**"]
successor: ""
size_budget: { spec_tokens: 12000, code_tokens: 80000 }
optional: true
serves: [capability/follow-from-browser, capability/see-project-state, capability/review-capability-cost]
---

## Contract
A hosted web app at `https://clm-app.dev/app/`, served by a Cloudflare Worker (`web.worker`) that claims the app's paths on `clm-app.dev`, beside CLM's public site at the root, which `web.site` builds and deploys as a Worker of its own (`public-site`, `site-worker-of-its-own`), where invited people signed in with GitHub see the CLM repos they can read: the spec graph, the spec branches proposed for review, implementation progress and the backlog. It approves a spec branch by appending an `approved` record to the signed-in person's web ledger (`per-writer-ledgers`), records that person's choice to be notified about a repo as a `subscribed` or `unsubscribed` record there (`push-subscriptions-in-ledger`), takes a proposal for the backlog as a `proposed` record, which `clm rebase` files as a backlog item on the next build (`web-proposals-as-ledger-records`), records a comment on a spec branch's diff or a request that it change as a `commented` or `changes_requested` record, the latter holding the branch's land until the author pushes a change or the requester approves (`web-review-feedback-as-ledger-records`), records a review decision on a capability or an architectural constraint as a `decided` record (`review-capability-cost`), records a person's answer to a backlog `question` that lists its choices, or `done` on a `human-task`, as an `answered` record (`web-answers-as-ledger-records`), and writes nothing else to the repository. Open pages show a change within seconds of its push (`web-live-channel`), and the installed app can notify people when something waits on them. What it caches is derived from git and GitHub and can be deleted at any time (`web-derived-cache`). It works on phones and installs as a PWA.

**Invited users** (`invite-only-web`). Only these GitHub logins may sign in. Anyone else gets a "not invited" page, and sees nothing of any repository. The public site is open to everyone (`web/site-served-directly`). Adding someone is a change to this list.

- `clunny`

## Design
The mockups in `assets/mockups/` are the reference for layout, colour, type and interaction: `assets/mockups/desktop.html` (sidebar, spec graph, PR document and graph tabs, implementation, backlog), `assets/mockups/mobile.html` (tab bar, PR review, graph with bottom sheet, offline implementation) and `assets/mockups/full-document.html` (a changed file in full). Their data is illustrative, and they show a light/system/dark theme switch the app doesn't have: the palette follows the system's setting. Where they and the requirements disagree, the requirements win.

Four parts, the Worker as the child `web.worker`, the public site as the child `web.site`, and the app here, with the app's live socket, Notify me switch and push handlers in the child `web.client-live` and its views in the child `web.views`:
- `crates/clm-web` (`web.worker`): the Worker, in Rust with `workers-rs`, built for `wasm32-unknown-unknown` (`web-reuses-rust-in-wasm`). It serves the API, the GitHub sign-in, the webhook and the static assets this component builds. Its API is the contract between the two.
- `web/`: the browser app in TypeScript, whose `web/src/main.ts` imports `web.client-live`'s modules from `web/src/live/` and `web.views`' from `web/src/views/` and wires them to the page, built with Vite (`web-typescript-frontend`). Graph layout uses ELK (`elkjs`, layered, edges from dependent to dependency) and word-level diffs use `diff` (jsdiff). The framework is the builder's choice. `web/.npmrc` sets `ignore-scripts=true` and `web/package-lock.json` is committed, so no dependency's install script runs, whichever npm command installs it (`unattended-commands`). Unattended sessions add dependencies with `npm install --ignore-scripts` and run the app's checks with `npm ci`, `npm run`, `npm test` and `npx playwright test`. Playwright's Chromium is installed outside the session: once per machine by the human, and by CI's browser-test step. `web/wrangler.toml` and `web/package.json`'s `build:worker` script build and deploy them as one Worker with assets, so the deploy stays here.
- `web/site/` (`web.site`): the homepage, the guide and the blog, a VitePress package of its own with its own `package.json`, lockfile and `wrangler.toml`, which the site workflow builds and deploys as a Worker of its own (`site-worker-of-its-own`). `web`'s build leaves it alone: `web/package.json`'s `build` script builds only the app, with Vite's `base` `/app/`, into `web/dist/app/`, beside the invited list at `web/dist/_clm/invited.json` and the retiring service worker at `web/dist/sw.js` (`app-at-app-path`). The capability harness builds and serves the site for its own tests.

**Loading and refresh.** Everything the browser shows comes from the Worker, which alone holds the user's token. The app keeps the repo list, and for the repo on screen its snapshot and spec-branch list, in memory, the capabilities report once the Capabilities view has drawn it, and the architecture report once the Architecture view has drawn it, so a navigation within a repo draws from them at once while it asks again. A thin loading bar runs across the top of the page for navigations only (`loading-indicator`). The graph doesn't wait for the implementation state: when it arrives, the inspector adds work in flight and the cards take their queue-state rings.

While the page is visible (`document.visibilityState`), the app refreshes about every 60 seconds, matching the Worker's live-state cache (`live-refresh`). A refresh asks again for the endpoints the view on screen was drawn from: the snapshot and the spec-branch list for every repo view, the implementation state for Home, the graph, Build, Backlog and a backlog item's page, the branch's review on a PR page, every spec branch's review, the capabilities report and the architecture report on Home, the capabilities report on Capabilities, the architecture report on Architecture, and the spec branches' reviews once the graph's inspector has loaded them. The repo list is asked for only on navigation. A refresh compares the parsed responses with those on screen, ignoring the implementation state's `now`. When they are equal it redraws nothing; Build's relative times are drawn against the browser's clock, so they stay current between redraws. Otherwise it redraws the view in place, keeping everything the user set in it: the graph's pan and zoom (as they were, even when the layout moves) and selected component, the page's scroll, tabs and switches (Document or Graph, In flight or Blockers, inline, after or before), the backlog filter, an open bottom sheet, and the full document's current change. A selected component the new state lacks is deselected. In the full document, the current change is kept while its text remains a change, else the nearest change by position is. No refresh runs while an approval, a Request changes, a comment, a proposal, a withdrawal, an answer or a decision on a capability or an architectural constraint is in progress, and a refresh begun before one ended draws nothing of what it read. The same refresh also runs at once when the repo's socket brings a notice (**Notices**, below). A failed refresh doesn't fall back to the copy kept for offline use, and the next one tries again.

**Views** (`web.views`). The pages the app draws (Home, the spec graph and its inspector, Spec PRs with the full document, Build, Backlog and a backlog item's page, Capabilities and Architecture) are `web.views`' render functions, and `web.views`' Design says what each shows. This component routes to them, gives them the responses it holds and the callbacks that post, load again or navigate, and keeps what the user set in them across a refresh (**Loading and refresh**).

**Look.** Colour tokens on `:root` with a dark and a light palette, one meaning per colour, the same as `view`'s charts (`colour-meanings`): green for active, converged, current, proven or complete; sky for unbuilt or ready; amber for claimed, behind, oversized or a modified element; rose for blocked, obsolete, a removed element or needs review; indigo for the accent and headings. The graph's two edge colours for a selected component, incoming and outgoing (`web.views/edge-direction`), are tokens of their own that mean nothing else. The `.md` styles (`markdown-styles`) cover inline code and code blocks, headings, lists, tables, quotes and links, and a code block or a table scrolls within its own box, so no text from the repository widens the page. The palette follows the system's light or dark setting. Inter for text and JetBrains Mono for IDs. The graph's canvas, on the spec graph and on a PR's Graph tab, runs from its top to the bottom of the window (on phones, to the top of the tab bar, or of the Approve bar on a PR, as drawn with whatever it holds), at least 320 px tall, rather than the mockups' fixed height; on desktop the inspector beside it scrolls within that height (`graph-fills-height`). A repo opens on Home (`home-first`), at `#/<owner>/<repo>`, which lists what waits on the person (`web.views/home-waiting-on-you`). At 900 px and wider: a sidebar with the repo switcher and views, Home first. Narrower: a bottom tab bar of five tabs, Home, Backlog, Graph, Build and More, where More lists PRs, Capabilities and Arch.; one column, Approve pinned above the tab bar, the graph pans and pinch-zooms, and the inspector is a bottom sheet.

**Notices** (`web-live-channel`). While the page is visible and online, and no sign-in page is on screen, the app gives `web.client-live`'s socket the live route of the repo on screen, `/api/repos/<o>/<r>/live`, and otherwise none, so the socket closes when the page is hidden, the repo changes or the user signs out. A signed-out session shows itself through the next refresh's 401. Each notice the socket hands over, `{"delivery"}`, starts a refresh at once (`notice-refresh`), whose requests add `?fresh=<delivery>`. A notice that arrives during a refresh starts another when it ends, and notices that arrive meanwhile share it. A notice that arrives while a write is in progress waits until it ends. The 60-second tick runs regardless, so a page without a working socket stays current. The copy kept for offline use is stored under the request's path without `fresh`, so notices add no entries and an offline load finds the newest answer. The sidebar, under the repo picker (on phones, beside the repo switcher), holds `web.client-live`'s Notify me switch for the repo on screen.

**PWA.** The installed app is the app only (`public-site`): a web app manifest with icons whose `id`, `start_url` and `scope` are `/app/`, and a service worker registered with scope `/app/`, served from under it, that never answers or caches a request outside `/app/`. Links from the app to the site leave the scope, so the installed app opens them in a browser tab over it. The service worker precaches the app shell and keeps the last API responses per repo in Cache Storage. Offline, the app shows them with a banner (`installable`), and Approve, Request changes, Comment, New proposal, Withdraw, the answer buttons and the decision buttons of a capability and of an architectural constraint are disabled.

**Size** (`slim-components`). This component's code budget is 80,000 tokens, above the default, because it is the shell of one browser app: `web/src/main.ts` routes every view, holds what they were drawn from and runs the one refresh they share, `web/src/styles.css` styles them all, and the end-to-end tests drive them through that shell. The views, which import nothing of the shell's, are the child `web.views`, as the Worker and the live socket, Notify me switch and push handlers are children of their own.

**Deploy.** The app is served at `https://clm-app.dev/app/` and the site at the root (`site-worker-of-its-own`). The `clm-app.dev` Custom Domain, on the zone the operator registered through Cloudflare Registrar on their Cloudflare account, belongs to the site's Worker (`web.site/site-worker`); this Worker claims the app's paths on the same hostname as routes, which Cloudflare runs before the Custom Domain's Worker. `web/wrangler.toml` declares:
- `name = "clm"`, the Worker;
- `routes` with `zone_name = "clm-app.dev"` and exactly the patterns `clm-app.dev/app`, `clm-app.dev/app/*`, `clm-app.dev/api/*`, `clm-app.dev/auth/*`, `clm-app.dev/webhook`, `clm-app.dev/_clm/*` and `clm-app.dev/sw.js`, and no Custom Domain;
- `[[kv_namespaces]]` with `binding = "CLM"` and `id = "9fa493aaa0a54d219b2dfebb5f681b9e"` (the namespace titled `clm-cache`);
- `[[durable_objects.bindings]]` and a `[[migrations]]` entry with `tag = "v1"` for `Live` (`live-binding`, `web.live`), SQLite-backed since the Free plan has only those. A binding to a class the Worker doesn't export fails the deploy, and `web.worker` exports `Live`; a test under `web/` reads them;
- `[triggers]` with `crons = []`: nothing runs on a schedule, since the `Replica` objects work on their own alarms (`web-repo-replica`), and the empty list is what clears the Cron Trigger earlier deploys created, because `wrangler deploy` leaves the deployed triggers as they are when `crons` is absent (`no-cron-trigger`);
- the static assets, `web/dist/`, holding the app under `app/`, `_clm/invited.json` and the retiring `/sw.js` (`retire-root-service-worker`), whose `run_worker_first` routes only `/api/*`, `/auth/*`, `/webhook`, `/_clm/*` and `/app` to the Worker's script, so the app's own files under `/app/` and `/sw.js` are answered from the assets without a script run. `not_found_handling` is `"none"`: a missing path under `/app/` gets a plain 404. The app routes by hash, so it needs no fallback page.

The move from one Worker to two happens once on the live zone, in an order that never leaves the app's paths unanswered: this Worker's routes are deployed before the site's Worker takes the Custom Domain from it. The build that makes the move says the order in its pull request, and the human runs it.

**Setup.** Nothing in the build or a deploy waits on it:
- the GitHub App exists, and its credentials are Worker secrets;
- the KV namespace `clm-cache` exists;
- the repo's Actions secrets hold `CLOUDFLARE_ACCOUNT_ID` and `CLOUDFLARE_API_TOKEN`, a token from the "Edit Cloudflare Workers" template limited to the operator's account and the `clm-app.dev` zone, with the zone's Workers Routes permission, which both Workers' deploys use.

To rotate a credential, generate a new private key or client secret in the App's settings and `wrangler secret put` it. `web/README.md` gives these commands. `web/README.md` also carries the App's manifest, which must be the one in `web.worker`'s Design; a test under `web/` holds a copy of that manifest and checks the README against it, since no test reads `specs/`. Both list `push` and `pull_request` under `default_events`.

**Tests** (`commands-live-in-their-crates`, `components-own-their-tests`). The browser tests under `web/e2e/` run the built app, with this component's entry file, stylesheet, routes and service worker, so what they show is this component's to state: they cite only its requirements, the `-in-the-browser` ones for what a view or the live modules draw. A view's own behaviour, drawn with the Worker faked, is `web.views`' and `web.client-live`'s to prove in their files. The tests that prove what `web/wrangler.toml` and `web/.npmrc` hold sit under `web/src/`, none in the Worker's crate.

## Requirements

### Requirement: mobile-layout
Below 900 px wide, the app SHALL replace the sidebar with a bottom tab bar of five tabs, Home, Backlog, Graph, Build and More, where More lists PRs, Capabilities and Arch., show one column with no horizontal scrolling, and keep Approve visible on a PR.

#### Scenario: phone
- GIVEN a 390 px wide viewport on a PR
- WHEN the page is shown
- THEN the tab bar is visible, the page doesn't scroll horizontally, and Approve is in view

#### Scenario: capabilities on a phone
- GIVEN a 390 px wide viewport
- WHEN the capabilities list is shown, and then a capability's page
- THEN the list shows one card per capability, neither page scrolls horizontally, and the page's decision buttons can be scrolled into view and tapped

#### Scenario: architecture on a phone
- GIVEN a 390 px wide viewport
- WHEN the architecture list is shown, and then an entry's page
- THEN the tab bar holds Home, Backlog, Graph, Build and More, More lists PRs, Capabilities and Arch., the list shows one card per entry, neither page scrolls horizontally, and the page's decision buttons can be scrolled into view and tapped

#### Scenario: home on a phone
- GIVEN a 390 px wide viewport and a Home listing a question with choices
- WHEN Home is shown
- THEN the page doesn't scroll horizontally, and the question's answer buttons are in view or can be scrolled into view and tapped

#### Serves
- capability/follow-from-browser

#### Needs
- web.views/home-waiting-on-you
- web.views/answer-buttons

### Requirement: installable
The app SHALL be installable as a PWA whose manifest's `id`, `start_url` and `scope` are `/app/` and whose service worker's scope is `/app/`, and when offline SHALL show the last data it loaded for a repo with its age, and disable Approve.

#### Scenario: offline launch
- GIVEN a repo viewed once online
- WHEN the app is reloaded offline
- THEN it shows that repo's last data with its age, and Approve is disabled

#### Scenario: app-only scope
- GIVEN the built app
- WHEN a test reads its manifest and registers its service worker
- THEN the manifest's `id`, `start_url` and `scope` are `/app/`, the service worker's scope is `/app/`, and a request for `/guide/` is not answered from its caches

#### Needs
- web.site/homepage-first-screen
- web.site/guide-pages

#### Serves
- capability/follow-from-browser

### Requirement: no-install-scripts
`web/.npmrc` SHALL set `ignore-scripts=true`, on the last line that sets the key, so installing `web/`'s dependencies never runs a dependency's install script, whichever npm command installs them.

#### Scenario: npmrc
- GIVEN the repository
- WHEN a test reads `web/.npmrc`
- THEN the last line that sets `ignore-scripts` sets it to `true`

#### Scenario: a later line turns scripts back on
- GIVEN the texts `ignore-scripts=true`, an empty file, `ignore-scripts=false`, and `ignore-scripts=true` followed by `ignore-scripts=false`
- WHEN the test's reading is applied to each
- THEN only the first passes

#### Serves
- architecture/deterministic-enforcement

### Requirement: invited-users
The app's build SHALL write the GitHub logins of this component's Contract's invited-users list, read from `specs/web/SPEC.md` at build time, to `_clm/invited.json` beside the built app, and `web/wrangler.toml` SHALL bind the built files to the Worker as `ASSETS` and send `/_clm/*` to its script, so the Worker completes sign-in only for those logins (`web.worker/invited-sign-in`) and the list is never served as a file.

#### Scenario: list from the spec
- GIVEN a build of the app
- WHEN a test reads its invited logins
- THEN they are exactly the logins listed in `specs/web/SPEC.md`'s Contract

#### Scenario: the list reaches only the Worker
- GIVEN the repository
- WHEN a test reads `web/wrangler.toml`
- THEN its assets are bound as `ASSETS`, and its `run_worker_first` sends `/_clm/invited.json` to the script

#### Serves
- architecture/invite-only-web

#### Needs
- web.worker/invited-sign-in

### Requirement: live-refresh
While the page is visible, the app SHALL ask the Worker again, about every 60 seconds and whenever the page becomes visible, for the endpoints the view on screen was drawn from, and SHALL make no such requests while it is hidden. When the answers differ from what is shown, ignoring the implementation state's `now`, it SHALL redraw the view in place, keeping what the user set in it: the graph's pan, zoom and selected component, the page's scroll position, its tabs, switches and filters, an open bottom sheet and the full document's current change. When they don't, it SHALL leave the view as it is. A refresh that fails SHALL leave the view as it is, except that a 401 shows the sign-in page.

#### Scenario: new PR appears
- GIVEN the PR list is on screen, scrolled down, and the Worker starts answering with one more spec PR
- WHEN 60 seconds pass
- THEN the new PR is listed without a reload, and the scroll position is unchanged

#### Scenario: graph keeps its place
- GIVEN the spec graph, panned and zoomed, with component `b` selected, and the snapshot changes `b`'s requirements proven
- WHEN the app refreshes
- THEN `b`'s card shows the new count, `b` is still selected, and the pan and zoom are unchanged

#### Scenario: hidden tab
- GIVEN a page hidden for 3 minutes
- WHEN it becomes visible
- THEN the app made no refresh requests while hidden, and asks again at once

#### Scenario: nothing changed
- GIVEN the Worker answers a refresh as before, but for the implementation state's `now`
- WHEN the app refreshes
- THEN the view's elements are not replaced

#### Scenario: Build keeps its switch
- GIVEN Build is on screen with **Blockers** chosen, and the implementation state changes
- WHEN the app refreshes
- THEN Build shows the new state with **Blockers** still chosen

#### Scenario: refresh fails
- GIVEN the spec graph is on screen and the Worker answers the next refresh with 502
- WHEN the app refreshes
- THEN the graph is unchanged and no offline banner shows

#### Serves
- capability/follow-from-browser

### Requirement: approve-by-branch
Approve SHALL post `{"fingerprint": <f>}`, where `<f>` is the fingerprint of the review on screen, to the Worker's approve route for that branch, and on success SHALL load the review again and show the approval from its approvals, not from a label. On a 409 it SHALL say the branch changed since the review was shown, show no approval, and load the review again from the Worker, never from the copy kept for offline use, drawing it with that message still shown, so that the next Approve pins what is then on screen (`approve-pins-the-review`); on any other failure it SHALL say "Not approved" with the status. When Approve is disabled is `web.views/approve-button`'s to state. It acts on the branch review `spec-branch-review` loads, so the two build together.

#### Scenario: approve
- GIVEN `spec/x`'s review with fingerprint `f1` and no approvals, and a Worker that answers the approve with 200 and then the review with an approval by `clunny/web`
- WHEN the reviewer taps Approve
- THEN one request posts `{"fingerprint": "f1"}` to `/branches/spec/x/approve`, the review is requested again, the header names `clunny/web`, and Approve is disabled

#### Scenario: branch moved
- GIVEN `spec/x`'s review drawn with fingerprint `f1`, and a Worker that answers that approve with 409 and the review with fingerprint `f2`
- WHEN the reviewer taps Approve
- THEN the status says the branch changed since the review was shown, no approval is shown, and the review on screen is the one with fingerprint `f2`

#### Serves
- capability/follow-from-browser
- capability/approve-intent
- architecture/approval-policy
- architecture/git-is-truth

#### Needs
- web.views/worker-requests
- web.views/approve-button
- web.worker.review/approve

### Requirement: live-binding
`web/wrangler.toml` SHALL bind the Durable Object class `Live` as `LIVE` and declare it in a migration under `new_sqlite_classes`.

#### Scenario: binding
- GIVEN the repository
- WHEN a test under `web/` reads `web/wrangler.toml`
- THEN it has a `[[durable_objects.bindings]]` entry with `name` `LIVE` and `class_name` `Live`, and a `[[migrations]]` entry whose `new_sqlite_classes` holds `Live`

#### Serves
- capability/follow-from-browser

#### Needs
- web.worker/live-class-export
- web.live/hub

### Requirement: loading-indicator
The app SHALL show a loading bar from the moment a navigation starts, the first load included, until the view navigated to is drawn, and not during background refreshes. It SHALL draw a view of the repo on screen from the repo list, snapshot, PRs, capabilities report and architecture report it already holds without waiting for them to be fetched again; a navigation to any view, Capabilities and Architecture included, shows the bar.

#### Scenario: slow view
- GIVEN the Worker holds its answer for the view navigated to
- WHEN the user navigates to it
- THEN the loading bar is visible while the answer is pending, and is gone once the view is drawn

#### Scenario: between views of a repo
- GIVEN the spec graph of `o/a` is on screen and the Worker stops answering
- WHEN the user opens `o/a`'s Backlog
- THEN the backlog is drawn from the snapshot already held

#### Scenario: graph before implementation
- GIVEN the snapshot is answered and the implementation state isn't yet
- WHEN the spec graph is opened
- THEN the graph is drawn

#### Serves
- architecture/ui-quality
- capability/follow-from-browser

### Requirement: graph-fills-height
The graph's canvas, on the spec graph and on a PR's Graph tab, SHALL extend from its top to the bottom of the viewport, or below 900 px to the top of the tab bar, or of the Approve bar on a PR, and SHALL be at least 320 px tall.

#### Scenario: desktop
- GIVEN a 1440 × 1000 px viewport
- WHEN the spec graph is shown
- THEN the canvas's bottom edge is within 32 px of the viewport's bottom and the page doesn't scroll vertically

#### Scenario: phone
- GIVEN a 390 × 844 px viewport
- WHEN the spec graph is shown
- THEN the canvas's bottom edge is within 16 px of the tab bar's top

#### Scenario: PR graph on a phone
- GIVEN a 390 × 844 px viewport
- WHEN a PR's Graph tab is shown
- THEN the canvas's bottom edge is within 16 px of the Approve bar's top

#### Serves
- architecture/ui-quality
- capability/follow-from-browser

### Requirement: notice-refresh
While the page is visible and online, the app SHALL give `web.client-live`'s socket the live route of the repo on screen, and none while the page is hidden, another repo is shown or the sign-in page is on screen. On each notice the socket hands over it SHALL refresh at once, as `live-refresh` refreshes, with every request carrying `fresh` set to the notice's `delivery`, running at most one refresh at a time and none during an approval. The 60-second refresh SHALL go on as before. Where the copy kept for offline use is stored is `web.views/worker-requests`' to state.

#### Scenario: a claim shows within seconds
- GIVEN Build on screen for `o/a` with its socket open, and the Worker starting to answer with component `b` claimed
- WHEN the socket receives `{"delivery": "d1"}`
- THEN within 2 seconds, with no timer advanced, Build shows `b` claimed, and the implementation request carried `fresh=d1`

#### Scenario: hidden
- GIVEN the spec graph on screen with its socket open
- WHEN the page is hidden for 3 minutes and then shown
- THEN the socket was closed while hidden, and a new one is opened when it is shown

#### Serves
- capability/follow-from-browser

#### Needs
- web.views/worker-requests
- web.client-live/live-updates
- web.worker/fresh-live-state

### Requirement: fresh-after-write
Every request the app makes to load data again after the Worker answered one of its writes (an approve, a Request changes, a comment, a proposal, a withdrawal, an answer to a backlog item or a capability's decision), on each response after which it loads again, success or refusal, SHALL carry `fresh` set to a delivery of the app's own, a random UUID from `crypto.randomUUID()`, new for each write, so the Worker answers from live state fetched anew rather than its minute-old copy. The copy kept for offline use SHALL be stored under the request's path without `fresh`, as for a notice's refresh.

#### Serves
- capability/follow-from-browser
- capability/approve-intent

#### Needs
- web.worker/fresh-live-state

#### Scenario: approve
- GIVEN `spec/x`'s review with fingerprint `f1` and no approvals, and a Worker that answers the approve with 200
- WHEN the reviewer taps Approve
- THEN the review is requested again from `/branches/spec/x/review?fresh=<u>`, `<u>` a UUID of lowercase hex digits and `-`, and the copy kept for offline use is stored under `/branches/spec/x/review`

#### Scenario: a new delivery per write
- GIVEN the backlog on screen, and a Worker that answers each proposal with 200
- WHEN two proposals are sent one after the other
- THEN each is followed by an implementation request carrying `fresh`, and the two values differ

#### Scenario: a refused write
- GIVEN `spec/x`'s review drawn with fingerprint `f1`, and a Worker that answers that approve with 409
- WHEN the reviewer taps Approve
- THEN the review is requested again with `fresh` set

### Requirement: app-at-app-path
`web`'s build SHALL put the app, built with Vite's `base` `/app/`, at `web/dist/app/`, the invited list at `web/dist/_clm/invited.json` and the retiring service worker at `web/dist/sw.js`, and SHALL NOT build `web.site`, which deploys on its own (`site-worker-of-its-own`). The capability harness built the site through this build, so this build moves it off: `capabilities/web/`'s `globalSetup` builds `web/site/` itself and its harness serves that output at `/` beside the app and the Worker, as the two Workers answer on the zone, a fix this build makes and names in its pull request (`path-ownership`). The app's end-to-end tests SHALL open it at `/app/`. Moving the app moves the test fixtures of other components that open it at `/` (`capabilities/web/`'s harness and specs): this build makes those fixes and names them in its pull request (`path-ownership`). Among them, the harness's `signIn` SHALL open `/app/` once the callback has set the session, wherever the callback sends the browser, so it holds both before and after `web.worker/sign-in-opens-app` points the callback at `/app/`. This requirement is built first of the move's: `installable` and `retire-root-service-worker` assume the app is served from `/app/`.

#### Scenario: built tree
- GIVEN a build of `web/`
- WHEN a test reads `web/dist/`
- THEN `app/index.html` is the app's page and loads its scripts from `/app/`, `_clm/invited.json` holds the invited list, `sw.js` is the retiring service worker, and there is no `index.html` or `guide/` at its root

#### Scenario: the app's links
- GIVEN the app at `/app/`
- WHEN it is opened at `/app/#/o/a/backlog`
- THEN it shows `o/a`'s backlog, and its sign-in button leads to `/auth/login`

#### Needs
- web.site/site-worker
- ci/site-deploy

#### Serves
- capability/learn-clm
- capability/follow-from-browser

### Requirement: retire-root-service-worker
The build SHALL put at `/sw.js` a service worker that, once installed in place of the app's old one at scope `/`, unregisters itself and drops its caches, and the app at `/app/` SHALL unregister any service worker registration whose scope is the site's root, so no browser that installed the app before the move keeps serving the old app over the site.

#### Scenario: old install
- GIVEN a browser holding the old service worker at scope `/`, with the old app shell cached
- WHEN it fetches `/sw.js` again and then opens `/`
- THEN the registration at scope `/` is gone, its caches are deleted, and `/` is answered by the network, not the old app shell

#### Scenario: the app clears it
- GIVEN a registration at scope `/` and the app opened at `/app/`
- WHEN the app starts
- THEN the registration at scope `/` is unregistered and the app's own at `/app/` remains

#### Needs
- web.site/homepage-first-screen
- web.site/guide-pages

#### Serves
- capability/follow-from-browser

### Requirement: latest-navigation-wins
Opened with no repo in its route, the app SHALL send the browser on to the spec graph of the first repo the Worker's repo list names, replacing the route in the history, or show the page that asks for the GitHub App to be installed when the list is empty. Whatever a navigation finds when its requests answer (a view to draw, the first repo to send the browser on to, the install page, an error, the sign-in page), it SHALL draw or do it only while no later navigation has started; one that a later navigation overtook does nothing, so the screen and the route are always the latest navigation's.

#### Scenario: opened at the app's root
- GIVEN a Worker whose repo list names `o/a` then `o/b`
- WHEN the app is opened at `/app/` with no route
- THEN the route becomes `#/o/a/graph` and `o/a`'s spec graph is drawn

#### Scenario: navigated while the repo list is pending
- GIVEN the app opened at `/app/` with no route, and the Worker's answer to `/api/repos` held back
- WHEN the route changes to `#/o/a/prs/spec%2Fx` and the answer then arrives
- THEN the route stays `#/o/a/prs/spec%2Fx` and that spec branch's review is drawn, not the graph

#### Serves
- architecture/ui-quality
- capability/follow-from-browser

### Requirement: honest-errors
When a request for a repo's view fails, the app SHALL say what failed, never more than it knows, and keep what it can show:
- a 404 SHALL show that the repo isn't readable with the person's GitHub account, or isn't covered by the App;
- a 503 `{"preparing": <share>}` SHALL show that CLM is preparing the repo, with the share done, and ask again every 5 seconds until it draws the view;
- any other 5xx or a network failure SHALL show the copy kept for offline use, with its age, when there is one, and otherwise that CLM couldn't load the repo just now and is retrying, and SHALL ask again with backoff, 5, 10, 20, then every 60 seconds, drawing the view once a request succeeds.

A view on screen whose refreshes have failed for 3 minutes SHALL stay on screen and show how old it is, until a refresh succeeds.

#### Scenario: busy, nothing kept
- GIVEN a repo never opened on this device, and a Worker that answers its snapshot 503 `{"error": "unavailable"}` twice, then the snapshot
- WHEN the person opens the repo's graph
- THEN the page says CLM couldn't load the repo and is retrying, says nothing about access, and then draws the graph without a reload

#### Scenario: busy, kept copy
- GIVEN a repo whose snapshot was kept an hour ago, and a Worker that answers 502
- WHEN the person opens the repo's graph
- THEN the graph is drawn from the kept copy with its age shown

#### Scenario: preparing
- GIVEN a Worker that answers the snapshot 503 `{"preparing": 0.4}`, then the snapshot
- WHEN the person opens the repo's graph
- THEN the page shows that the repo is being prepared, 40% done, and then draws the graph

#### Scenario: no access
- GIVEN a Worker that answers the snapshot 404
- WHEN the person opens the repo
- THEN the page says the repo isn't readable with their account or isn't covered by the App

#### Scenario: refreshes keep failing
- GIVEN the graph on screen, and a Worker that answers every refresh 503 for 3 minutes
- WHEN the refreshes run
- THEN the graph stays, unchanged, and shows how old it is

#### Serves
- capability/follow-from-browser
- architecture/ui-quality

#### Needs
- web.worker/views-from-replica

### Requirement: replica-binding
`web/wrangler.toml` SHALL bind the Durable Object class `Replica` as `REPLICA` and declare it in a migration of its own under `new_sqlite_classes`, after the one that declares `Live`.

#### Scenario: binding
- GIVEN the repository
- WHEN a test under `web/` reads `web/wrangler.toml`
- THEN it has a `[[durable_objects.bindings]]` entry with `name` `REPLICA` and `class_name` `Replica`, and a `[[migrations]]` entry after the `Live` one whose `new_sqlite_classes` holds `Replica`

#### Serves
- capability/follow-from-browser

#### Needs
- web.worker/replica-class-export

### Requirement: no-cron-trigger
`web/wrangler.toml` SHALL hold `crons = []` under `[triggers]`, so that a deploy clears the Cron Trigger earlier deploys created and nothing calls the Worker on a schedule. The key SHALL stay, empty: `wrangler deploy` leaves the deployed triggers as they are when `crons` is absent.

#### Scenario: the trigger is cleared
- GIVEN the repository
- WHEN a test under `web/` reads `web/wrangler.toml`
- THEN `triggers.crons` is present and is an empty list

#### Serves
- capability/follow-from-browser

### Requirement: site-served-directly
`web/wrangler.toml` SHALL route to this Worker, on `clm-app.dev`, exactly the app's paths, `/app`, `/app/*`, `/api/*`, `/auth/*`, `/webhook`, `/_clm/*` and `/sw.js`, with no Custom Domain, so every other path reaches the site's Worker (`web.site/site-worker`); its `run_worker_first` SHALL send to the script exactly `/api/*`, `/auth/*`, `/webhook`, `/_clm/*` and `/app`, so its assets answer the app's files and `/sw.js` with no script run, and `not_found_handling` SHALL be `"none"`.

#### Scenario: who answers what
- GIVEN the repository
- WHEN a test reads `web/wrangler.toml`
- THEN its routes are exactly those seven patterns on `clm-app.dev`, none is a Custom Domain, its `run_worker_first` sends `/api/repos`, `/auth/login`, `/webhook`, `/_clm/invited.json` and `/app` to the script and not `/app/`, `/app/main.js` or `/sw.js`, its assets are the directory `./dist`, the Worker's name is `clm`, and `not_found_handling` is `"none"`

#### Scenario: the site isn't the app's
- GIVEN the routes of `web/wrangler.toml`
- WHEN a test matches `/`, `/guide/quickstart`, `/blog/`, `/blog/feed.xml`, `/privacy` and `/404.html` against them
- THEN none matches, and `/app/`, `/api/repos`, `/auth/login`, `/webhook`, `/_clm/invited.json` and `/sw.js` each match one

#### Serves
- capability/learn-clm
- architecture/invite-only-web

#### Needs
- web.worker/invited-sign-in

### Requirement: links-to-site
The sidebar's `◆ CLM` logo SHALL be a link to `/`, the site's homepage, and the sidebar's foot SHALL hold a Guide link to `/guide/` beside Add repos. Below 900 px wide, the top bar SHALL begin with a `◆` link to `/`, labelled for assistive technology as the CLM homepage, before the repo switcher. These links SHALL be plain links that load the site, never routes of the app's hash router.

#### Scenario: desktop sidebar
- GIVEN the app open on a repo's spec graph in a 1280×800 window
- WHEN a test reads the sidebar
- THEN its logo is a link whose `href` is `/`, and a link reading `Guide` with `href` `/guide/` sits beside Add repos

#### Scenario: phone top bar
- GIVEN the app open on a repo's spec graph in a 390×844 window
- WHEN a test reads the top bar
- THEN its first element is a link whose `href` is `/` with an accessible name naming the CLM homepage, and it comes before the repo switcher

#### Serves
- capability/learn-clm

### Requirement: sign-in-early-access
The sign-in page's logo SHALL be a link to `/`. The page SHALL say that CLM works fully without the web app by following the quickstart, linking to `/guide/quickstart`, and that the web app is an optional extra for convenience, in early access, and SHALL link to Request early access at `https://github.com/clunny/clm/issues/new?template=early-access.yml`, the one constant in the app that names it. Sign in with GitHub and Install the GitHub App SHALL stay as they are.

#### Scenario: signed out
- GIVEN a Worker that answers the repo list 401
- WHEN the app is opened
- THEN the sign-in page's logo links to `/`, its text says CLM works without the web app and that the web app is an optional extra in early access, it links to `/guide/quickstart` and to `https://github.com/clunny/clm/issues/new?template=early-access.yml` with a link reading `Request early access`, and Sign in with GitHub still leads to `/auth/login`

#### Serves
- capability/learn-clm

### Requirement: home-first
The app SHALL open a repo at `#/<owner>/<repo>` on Home (`web.views/home-waiting-on-you`), list Home first in the sidebar and the tab bar, badge it with the number of entries waiting, route `#/<owner>/<repo>/backlog/<slug>` to that item's page (`web.views/backlog-item-page`), and refresh Home and an item's page as **Loading and refresh** says, keeping the page's scroll. Home SHALL draw from the snapshot at once and add each section as the response it needs arrives.

#### Scenario: a repo opens on Home
- GIVEN a signed-in person with repo `o/a`
- WHEN they choose `o/a` in the repo switcher
- THEN the address is `#/o/a`, Home is shown, and it is the sidebar's first view

#### Scenario: an item's page
- GIVEN repo `o/a` whose backlog holds `q`
- WHEN `#/o/a/backlog/q` is opened
- THEN `q`'s page is shown

#### Serves
- capability/follow-from-browser

#### Needs
- web.views/home-waiting-on-you
- web.views/backlog-item-page

### Requirement: markdown-styles
Text rendered from the repository's Markdown SHALL style inline code and code blocks in JetBrains Mono, and headings, lists, tables, quotes and links, and a code block or a table wider than its box SHALL scroll within that box, so the page never scrolls horizontally because of it.

#### Scenario: a long code line on a phone
- GIVEN a 390 px wide viewport and a backlog item whose body holds a 300-character code block and a five-column table
- WHEN its page is shown
- THEN the page doesn't scroll horizontally, and the code block scrolls within its box

#### Serves
- capability/follow-from-browser

#### Needs
- web.views/repo-markdown

### Requirement: answer-from-app
The app SHALL give Home and a backlog item's page the answer buttons' `Answering` (`web.views/answer-buttons`): the implementation state's `answers`, none until it arrives, whether the app is offline, and an `onAnswer` that runs as a write (**Loading and refresh**), so no refresh runs during it. `onAnswer` SHALL post `{"answer": <answer>}` to `POST /api/repos/<o>/<r>/backlog/<slug>/answer`; on 200 it SHALL ask for the implementation state again (`fresh-after-write`) and redraw the view on screen from it, keeping its scroll; on 422 it SHALL say the answer was refused, on 404 that the item is no longer in the backlog, on any other status that the answer wasn't recorded, with the status, and when the request isn't sent that it is offline, each as text under the buttons, asking for nothing again; a 401 shows the sign-in page, as for the app's other writes.

#### Scenario: answered on the item's page
- GIVEN repo `o/a` whose backlog holds question `q` with choices `yes` and `no`, and a Worker that answers the answer with 200 and then the implementation state with the answer `yes` by `alice/web`, `mine` true
- WHEN `#/o/a/backlog/q` is opened and `yes` is tapped
- THEN exactly one request posted `{"answer": "yes"}` to `/api/repos/o/a/backlog/q/answer`, the implementation state was requested again with `fresh`, and `yes` is drawn chosen

#### Scenario: answered on Home
- GIVEN the same repo, with Home on screen
- WHEN `yes` is tapped under `q`
- THEN one request posted `{"answer": "yes"}` to `q`'s route, and Home draws `yes` chosen

#### Scenario: refused
- GIVEN `q`'s page, and a Worker that answers the answer with 422 `{"error": "answer"}`
- WHEN `yes` is tapped
- THEN a message under the buttons says the answer was refused, the buttons stay enabled, and no implementation state was requested after the post

#### Scenario: offline
- GIVEN `q`'s page drawn from the copy kept for offline use, offline
- WHEN the page is shown
- THEN its answer buttons are disabled

#### Serves
- capability/follow-from-browser
- architecture/git-is-truth

#### Needs
- web.views/answer-buttons
- web.views/home-waiting-on-you
- web.views/backlog-item-page
- web.worker/answers

### Requirement: cost-views-in-the-browser
The built app, opened in a browser with the Worker's API answered, SHALL show the Capabilities and Architecture views and their pages as `web.views` draws them (`web.views/capabilities-list`, `architecture-list`, `decide-buttons`, `architecture-decide-buttons`), under this component's stylesheet and routes: in a 1280 px wide window each list is a table and no card shows; a proof `held <c>/<n>` with `c` less than `n` is drawn in the theme's amber; Architecture is the sidebar's last view, after Capabilities; a row opens its page at `#/<owner>/<repo>/capabilities/<slug>` or `#/<owner>/<repo>/architecture/<slug>`, with the view's sidebar entry marked; and a decision posts once however often it is tapped while in flight, after which the report is asked for again with `fresh` and the page shows what the Worker then answers, never an older report whose request began before the decision.

#### Scenario: the capabilities table
- GIVEN a 1280 px wide window, and a report listing `b` then `a`, with `a` flagged
- WHEN `#/o/a/capabilities` is opened, "⚑ only" is turned on, and `a`'s row is chosen
- THEN the table lists `b` first and no card is visible, then only `a` is listed, and the page at `#/o/a/capabilities/a` shows `a`'s entry with Capabilities marked in the sidebar

#### Scenario: the architecture table
- GIVEN a 1280 px wide window, and a report listing `e` (flagged, one of its two enforcing rules current, with `#### Untested`) then `g` (holding only `#### Untested`)
- WHEN Architecture is chosen in the sidebar from the graph; and when `#/o/a/architecture` is opened, "⚑ only" is turned on, and `e`'s row is chosen
- THEN in the first Architecture is listed last, after Capabilities, and opens `#/o/a/architecture`; in the second `e` comes first and reads `held 1/2 + untested` in the theme's amber, `g` reads `untested`; then only `e` is listed; and the page at `#/o/a/architecture/e` shows `e`'s entry

#### Scenario: indexing
- GIVEN an architecture report whose `indexing` is true
- WHEN `#/o/a/architecture` is opened
- THEN `e`'s proof reads `? / 2 + untested` and the note says coverage is still indexing

#### Scenario: a constraint decided
- GIVEN `e`'s page with value medium chosen, and a Worker that holds its answer to the decision
- WHEN Redesign is tapped twice
- THEN Redesign is disabled while the request is out, one request posted `{"version", "value": "medium", "decision": "redesign"}` with the version shown, the report was asked for once more, with `fresh`, and the page shows `redesign (value medium)` by its writer

#### Scenario: a capability decided
- GIVEN `a`'s page with value low chosen, and a Worker that holds its answer to the decision
- WHEN Cut is tapped twice, and again after another value is chosen
- THEN one request posted `{"version", "value": "low", "decision": "cut"}` with the version shown, the report was asked for once more, with `fresh`, and the page shows `cut (value low)` by its writer

#### Scenario: an older report arrives after a decision
- GIVEN `a`'s page, and a refresh whose request is answered, with the report as it was before, only after a decision has been drawn
- WHEN Cut is tapped and that answer arrives
- THEN the page still shows the decision

#### Scenario: moved, and signed
- GIVEN a Worker that answers a decision with 409 and the report with the entry reworded at a new version; and, apart, one that answers 422 with `error` `signed` and a `command`; and a report whose `signed` is true
- WHEN a decision is tapped on a capability's and on a constraint's page for the first, and on a capability's page for the second, and a constraint's page is shown for the third
- THEN the first says the entry changed since it was shown and draws it as reworded, and on the capability's page the next tap posts the new version; the second shows the Worker's `command` with the buttons disabled; and the third shows the buttons disabled with `clm architecture decide e --value high|medium|low --decision keep|cut|redesign|accept`

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser

### Requirement: review-in-the-browser
The built app, opened in a browser with the Worker's API answered, SHALL show the PR list and a spec branch's review as `web.views` draws them (`web.views/spec-branch-review`, `full-document`, `signed-approvals`, `approve-button`, `comment-on-review`, `request-changes`), under this component's stylesheet and routes: a branch opens at `#/<owner>/<repo>/prs/<branch>` with its review loaded from the Worker's branch route, and no request goes to a `/pulls` route from the review, the graph or Build; a file opened in full is scrolled so its first change is in the window, and `j` brings the next into it; on a phone the signed note sits in the Approve bar, in the window and above the tab bar, Request changes, Approve and the request's form are wholly in the window, and the page never scrolls sideways; Approve and Request changes each post once however often they are tapped while in flight; and after a request for changes or a comment is taken, the review is asked for again.

#### Scenario: a branch without a page
- GIVEN a branch list holding `spec/c` with page #7 "Change b" and `spec/d` with no page
- WHEN `#/o/a/prs` is opened and `spec/d` is chosen
- THEN both are listed, `spec/c` with #7 and its title, the address is `#/o/a/prs/spec%2Fd`, and the review, loaded from `/branches/spec/d/review`, shows no page link

#### Scenario: approved by
- GIVEN `spec/c`'s review listing an approval by `clunny/web` and saying `checked` false
- WHEN its review, then the graph with a component selected, then Build are opened
- THEN the header names `clunny/web` with the approval's time and says "not checked", and no request went to a `/pulls` route

#### Scenario: a file in full
- GIVEN a review whose document changes two lines far below its top
- WHEN the file is opened in full and `j` is pressed
- THEN the first change is in the window, marked `Change 1 of 2`, and then the second, marked `Change 2 of 2`

#### Scenario: became signed
- GIVEN `spec/c`'s review drawn with no signed note, and a Worker that answers the approve with 422 and `error` `signed`, and then the review with `signed` true
- WHEN Approve is tapped
- THEN the review is requested again, the note shows `clm approve spec/c`, and Approve is disabled and doesn't read "Approved"

#### Scenario: the signed note on a phone
- GIVEN a 390 px wide window, and `spec/c`'s review with `signed` true and one approval with `signed` false
- WHEN the review is opened
- THEN the note is in the window, inside the Approve bar and above the tab bar, the header marks the approval "unsigned, doesn't count", and the page is no wider than the window

#### Scenario: one request, then approved
- GIVEN `spec/c`'s review with a fingerprint and no approval, and a Worker that holds its answer to the approve
- WHEN Approve is tapped twice
- THEN exactly one approve request was sent, and the button reads Approved and is disabled

#### Scenario: a comment on a requirement
- GIVEN `spec/c`'s review with fingerprint `f1`, which modifies `n/r`, and a Worker that answers the comment with 200 and then the review with a comment on `n/r` by `alice/web`; and, apart, a Worker that answers a comment 422
- WHEN "Name the rule" is sent from `n/r`'s card in the first, and a comment from the intent in the second
- THEN the first posted `{"fingerprint": "f1", "target": "n/r", "text": "Name the rule"}` once, asked for the review again, closed the form and shows the comment by `alice` on the card; the second keeps the form with its text and says the Worker refused it

#### Scenario: changes requested
- GIVEN `spec/c`'s review with fingerprint `f1` and `signed` true, and a Worker that holds its answer to the request, then answers 200 and the review with a held request by `alice/web`
- WHEN "Split it" is submitted twice from the Approve bar
- THEN exactly one request posted `{"fingerprint": "f1", "text": "Split it"}`, the review was asked for again, and the header says changes requested by `alice` with "Split it"

#### Scenario: a request the Worker doesn't take
- GIVEN a Worker that answers the request with 409 and then the review with fingerprint `f2`; and, apart, one that answers 403
- WHEN a request is submitted in each, and again in the first
- THEN the first says the branch changed since the review was shown, draws the review with `f2`, keeps the text, and the next submit posts `f2`; the second says write access is needed and leaves the form usable

#### Scenario: Request changes on a phone
- GIVEN a 390 px wide touch window on `spec/c`'s review
- WHEN Request changes is tapped
- THEN Request changes and Approve were wholly in the window, in the Approve bar, the form's send button is wholly in the window, and the page is no wider than the window

#### Serves
- capability/follow-from-browser
- capability/approve-intent

### Requirement: graph-in-the-browser
The built app, opened in a browser with the Worker's API answered, SHALL show the graph and its inspector as `web.views` draws them (`web.views/graph-view`, `oversized-component`, `inspector-overlay`), under this component's stylesheet and routes: a queue-state ring is a solid outline in the theme's colour for the state, and an oversized component's outline is the theme's amber until a ring colours it; the state word stays inside its card however long the component's ID is; a graph opened with the implementation state already loaded by another view is ringed at once; a PR's Graph tab, at `#/<owner>/<repo>/prs/<branch>/graph`, marks only the components the PR touches; and the inspector opens over the graph as a drawer from the window's right edge, at most 640 px wide and no taller than the window, leaving the canvas as wide as it was.

#### Scenario: queue states
- GIVEN an implementation state with `b` claimed and `a` blocked
- WHEN `#/o/a/graph` is opened
- THEN `b` names `claimed` and is outlined solid in the theme's amber, and `a` names `blocked` and is outlined solid in the theme's rose

#### Scenario: a long ID
- GIVEN the same with `b` renamed to an ID far too long for its card
- WHEN the graph is opened
- THEN the card's state word reads `claimed` and ends inside the card

#### Scenario: ringed from a state already held
- GIVEN Build opened for `o/a`, and a Worker that then answers no further implementation request
- WHEN the address changes to `#/o/a/graph`
- THEN `b` is ringed as claimed and `a` names `blocked`

#### Scenario: a PR's graph
- GIVEN a review that touches `b`, which depends on `a`
- WHEN `#/o/a/prs/spec%2Fc/graph` is opened
- THEN only `b` is marked as touched, with its edge to `a`; and on `#/o/a/graph` the inspector of `b` lists `spec/c` and that of `a` says no open PR changes it

#### Scenario: oversized
- GIVEN a snapshot whose child components, `a` among them, are oversized, and an implementation state, not yet answered, in which `a` is blocked
- WHEN the graph is opened, and then the implementation state arrives
- THEN `a` is first outlined in the theme's amber with the legend reading `behind · claimed · oversized`, and then keeps its `⚠` and is outlined in the theme's rose

#### Scenario: the inspector on desktop
- GIVEN a 1280 px wide window on the graph
- WHEN `b` is selected, and Escape is pressed
- THEN the canvas is as wide as before, the inspector shows `b`'s contract in a drawer that ends at the window's right edge, overlaps the canvas, is at most 640 px wide and no taller than the window, and Escape closes it

#### Scenario: closing the inspector
- GIVEN the inspector open on `b`
- WHEN its × is tapped, `b` is selected again, and then once more
- THEN × closes it and clears the selection and its highlighted edges, the first tap opens it, and the second closes it

#### Serves
- capability/see-project-state
- capability/follow-from-browser

### Requirement: backlog-in-the-browser
The built app, opened in a browser with the Worker's API answered, SHALL show the Backlog as `web.views` draws it (`web.views/backlog-view`, `backlog-taken`, `propose-from-backlog`), under this component's routes: the backlog is drawn from the snapshot before the implementation state arrives and takes its marks when it does; an item opens its page at `#/<owner>/<repo>/backlog/<slug>`, from the Backlog and from the inspector, and a slug the backlog doesn't hold says so with a link back; and New proposal posts once however often it is submitted while in flight, after which, and after a Withdraw, the implementation state is asked for again.

#### Scenario: marks arrive
- GIVEN an implementation state, not yet answered, whose `taken` holds change `x` on main naming `crash` and a bare claim on `later`
- WHEN `#/o/a/backlog` is opened, and then the implementation state arrives
- THEN `crash` is first drawn with no mark, and then `crash` reads `resolved by x`, `later` reads `claimed`, and `setup` has no mark

#### Scenario: an item opens its page
- GIVEN repo `o/a` whose backlog holds `later`, on component `b`
- WHEN `later` is tapped in the Backlog, `#/o/a/backlog/gone` is opened and its link followed, and `later` is tapped in the inspector of `b`
- THEN the page at `#/o/a/backlog/later` shows its title, `gone`'s page says it isn't in the backlog and leads back to `#/o/a/backlog`, and the inspector's link opens `#/o/a/backlog/later`

#### Scenario: proposed
- GIVEN the Backlog of `o/a`, and a Worker that holds its answer to the proposal, then answers 200 and lists it in `proposals` by `alice/web`
- WHEN New proposal is opened, the title "Export CSV" and the intent "Let me export." are entered, and it is submitted twice
- THEN exactly one request posted `{"title": "Export CSV", "body": "Let me export.", "nodes": []}`, the form closed, the implementation state was asked for again, and Proposed lists "Export CSV" with `alice` and the note that the next build files it

#### Scenario: a proposal the Worker doesn't take
- GIVEN a Worker that answers the proposal with 409, and, apart, one that answers 422 naming `nodes`
- WHEN a proposal is submitted in each
- THEN the first keeps the form with its title, saying the title is taken, and the second says the Worker refused the nodes

#### Scenario: withdraw
- GIVEN Proposed listing `p1` with `mine` true and `p2` with `mine` false
- WHEN `p1`'s Withdraw is tapped
- THEN `p2` shows no Withdraw, one `DELETE` went to `/api/repos/o/a/proposals/p1`, the implementation state was requested once more, and `p1` is gone

#### Serves
- capability/keep-a-backlog
- capability/see-project-state
- capability/follow-from-browser

### Requirement: push-in-the-browser
The built app, opened in a browser, SHALL wire `web.client-live`'s Notify me switch and push handlers into the page and its service worker (`web.client-live/notify-switch`, `push-shown`): the switch shows in the sidebar once the Worker answers a push key, and is hidden when it answers 404; and the service worker the app registers, `/app/sw.js` with scope `/app/`, shows a push and handles a click on it through the handlers it imports.

#### Scenario: no key
- GIVEN a 1280 px wide window, and a Worker answering 404 for the push key
- WHEN `#/o/a/graph` is opened and the key has been asked for
- THEN the sidebar shows no switch

#### Scenario: a key and no subscription
- GIVEN a Worker answering a push key, and a browser with no subscription
- WHEN `#/o/a/graph` is opened
- THEN the sidebar shows the switch, off and enabled, and no request went to the repo's push route

#### Scenario: turn on
- GIVEN a browser with notification permission granted, and a Worker answering the key `k` and `{"subscribed": false}` for `o/a`
- WHEN the switch is turned on, and then another view is chosen
- THEN the push manager was subscribed with `userVisibleOnly` and `applicationServerKey` `k`, one `POST /api/repos/o/a/push` carried that subscription, the switch reads on, and the redraw asked the push route for the subscription's endpoint and still reads on

#### Scenario: the service worker shows a push
- GIVEN the app opened at `/app/`, its service worker `/app/sw.js` active, and no app window known to it
- WHEN the worker receives a push `{"title": "o/a: waiting on you", "body": "Which region?", "url": "/app/#/o/a/backlog"}`, and then a click on its notification
- THEN a notification with that title and body, tagged `/app/#/o/a/backlog`, with the icon `/app/icon-192.png`, was shown, and a window was opened at `/app/#/o/a/backlog` on the worker's origin

#### Serves
- capability/follow-from-browser
