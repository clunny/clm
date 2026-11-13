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
A hosted web app at `https://clm-app.dev/app/`, served by a Cloudflare Worker (`web.worker`) that claims the app's paths on `clm-app.dev`, beside CLM's public site at the root, which `web.site` builds and deploys as a Worker of its own (`public-site`, `site-worker-of-its-own`), where invited people signed in with GitHub see the CLM repos they can read: the spec graph, the spec branches proposed for review, implementation progress and the backlog. It approves a spec branch by appending an `approved` record to the signed-in person's web ledger (`per-writer-ledgers`), records that person's choice to be notified about a repo as a `subscribed` or `unsubscribed` record there (`push-subscriptions-in-ledger`), takes a proposal for the backlog as a `proposed` record, which `clm rebase` files as a backlog item on the next build (`web-proposals-as-ledger-records`), records a comment on a spec branch's diff or a request that it change as a `commented` or `changes_requested` record, the latter holding the branch's land until the author pushes a change or the requester approves (`web-review-feedback-as-ledger-records`), records a review decision on a capability or an architectural constraint as a `decided` record (`review-capability-cost`), and writes nothing else to the repository. Open pages show a change within seconds of its push (`web-live-channel`), and the installed app can notify people when something waits on them. What it caches is derived from git and GitHub and can be deleted at any time (`web-derived-cache`). It works on phones and installs as a PWA.

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

**Views** (`web.views`). The pages the app draws (the spec graph and its inspector, Spec PRs with the full document, Build, Backlog, Capabilities and Architecture) are `web.views`' render functions, and `web.views`' Design says what each shows. This component routes to them, gives them the responses it holds and the callbacks that post, load again or navigate, and keeps what the user set in them across a refresh (**Loading and refresh**).

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
`web/.npmrc` SHALL set `ignore-scripts=true`, so installing `web/`'s dependencies never runs a dependency's install script, whichever npm command installs them.

#### Scenario: npmrc
- GIVEN the repository
- WHEN a test reads `web/.npmrc`
- THEN it sets `ignore-scripts=true`

#### Serves
- architecture/deterministic-enforcement

### Requirement: invited-users
The Worker SHALL complete sign-in only for the GitHub logins in this component's Contract's invited-users list, read from `specs/web/SPEC.md` at build time. For anyone else it SHALL show a "not invited" page and set no session cookie, and every API route SHALL answer 401 without a valid session.

#### Scenario: invited
- GIVEN the list holds `clunny` and GitHub completes the authorization for `clunny`
- WHEN the browser reaches the callback
- THEN the response sets the session cookie

#### Scenario: not invited
- GIVEN GitHub completes the authorization for `someone-else`
- WHEN the browser reaches the callback
- THEN the response is the "not invited" page, sets no session cookie, and a following request for the repo list answers 401

#### Scenario: list from the spec
- GIVEN a build of the Worker
- WHEN a test reads its invited logins
- THEN they are exactly the logins listed in `specs/web/SPEC.md`'s Contract

#### Serves
- architecture/invite-only-web

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
Approve SHALL post `{"fingerprint": <f>}`, where `<f>` is the fingerprint of the review on screen, to the Worker's approve route for that branch, and on success SHALL load the review again and show the approval from its approvals, not from a label. On a 409 it SHALL say the branch changed since the review was shown, show no approval, and load the review again from the Worker, never from the copy kept for offline use, drawing it with that message still shown, so that the next Approve pins what is then on screen (`approve-pins-the-review`); on any other failure it SHALL say "Not approved" with the status. Approve SHALL be disabled offline, when the review has no fingerprint, and once the review lists an approval. It acts on the branch review `spec-branch-review` loads, so the two build together.

#### Scenario: approve
- GIVEN `spec/x`'s review with fingerprint `f1` and no approvals, and a Worker that answers the approve with 200 and then the review with an approval by `clunny/web`
- WHEN the reviewer taps Approve
- THEN one request posts `{"fingerprint": "f1"}` to `/branches/spec/x/approve`, the review is requested again, the header names `clunny/web`, and Approve is disabled

#### Scenario: no fingerprint
- GIVEN a review with no fingerprint
- WHEN it is shown
- THEN Approve is disabled

#### Scenario: branch moved
- GIVEN `spec/x`'s review drawn with fingerprint `f1`, and a Worker that answers that approve with 409 and the review with fingerprint `f2`
- WHEN the reviewer taps Approve
- THEN the status says the branch changed since the review was shown, no approval is shown, and the review on screen is the one with fingerprint `f2`

#### Serves
- capability/follow-from-browser
- capability/approve-intent
- architecture/approve-pins-the-review
- architecture/per-writer-ledgers

#### Needs
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
While the page is visible and online, the app SHALL give `web.client-live`'s socket the live route of the repo on screen, and none while the page is hidden, another repo is shown or the sign-in page is on screen. On each notice the socket hands over it SHALL refresh at once, as `live-refresh` refreshes, with every request carrying `fresh` set to the notice's `delivery`, running at most one refresh at a time and none during an approval. The 60-second refresh SHALL go on as before, and the copy kept for offline use SHALL be stored under each request's path without `fresh`.

#### Scenario: a claim shows within seconds
- GIVEN Build on screen for `o/a` with its socket open, and the Worker starting to answer with component `b` claimed
- WHEN the socket receives `{"delivery": "d1"}`
- THEN within 2 seconds, with no timer advanced, Build shows `b` claimed, and the implementation request carried `fresh=d1`

#### Scenario: hidden
- GIVEN the spec graph on screen with its socket open
- WHEN the page is hidden for 3 minutes and then shown
- THEN the socket was closed while hidden, and a new one is opened when it is shown

#### Scenario: offline copy
- GIVEN a notice with delivery `d1`
- WHEN the refresh it starts asks for `/api/repos/o/a/implementation`
- THEN the request goes to `/api/repos/o/a/implementation?fresh=d1`, and the copy kept for offline use is stored under `/api/repos/o/a/implementation`

#### Serves
- capability/follow-from-browser

#### Needs
- web.client-live/live-updates
- web.worker/fresh-live-state

### Requirement: fresh-after-write
Every request the app makes to load data again after the Worker answered one of its writes (an approve, a Request changes, a comment, a proposal, a withdrawal or a capability's decision), on each answer after which it loads again, success or refusal, SHALL carry `fresh` set to a delivery of the app's own, a random UUID from `crypto.randomUUID()`, new for each write, so the Worker answers from live state fetched anew rather than its minute-old copy. The copy kept for offline use SHALL be stored under the request's path without `fresh`, as for a notice's refresh.

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
`web/wrangler.toml` SHALL hold `crons = []` under `[triggers]`, so that a deploy clears the Cron Trigger earlier deploys created and nothing calls the Worker on a schedule. The key SHALL stay, empty: `wrangler deploy` leaves the deployed triggers as they are when `crons` is absent. The test `binds_the_kv_namespace_and_runs_cron_every_minute` in `crates/clm-web/tests/deploy.rs` pins the old value, so this requirement's build SHALL drop that assertion and the cron from the test's name, as a fix its change forces, named in its pull request (`path-ownership`).

#### Scenario: the trigger is cleared
- GIVEN the repository
- WHEN a test under `web/` reads `web/wrangler.toml`
- THEN `triggers.crons` is present and is an empty list

#### Serves
- capability/follow-from-browser

### Requirement: site-served-directly
`web/wrangler.toml` SHALL route to this Worker, on `clm-app.dev`, exactly the app's paths, `/app`, `/app/*`, `/api/*`, `/auth/*`, `/webhook`, `/_clm/*` and `/sw.js`, with no Custom Domain, so every other path reaches the site's Worker (`web.site/site-worker`); its `run_worker_first` SHALL send to the script exactly `/api/*`, `/auth/*`, `/webhook`, `/_clm/*` and `/app`, so its assets answer the app's files and `/sw.js` with no script run, and `not_found_handling` SHALL be `"none"`. Changing the routing moves the test fixtures that repeat it (`capabilities/web/harness.ts`'s `run_worker_first` and `not_found_handling`, `crates/clm-web/tests/deploy.rs`): this build makes those fixes and names them in its pull request (`path-ownership`).

#### Scenario: who answers what
- GIVEN the repository
- WHEN a test reads `web/wrangler.toml`
- THEN its routes are exactly those seven patterns on `clm-app.dev`, none is a Custom Domain, its `run_worker_first` sends `/api/repos`, `/auth/login`, `/webhook`, `/_clm/invited.json` and `/app` to the script and not `/app/`, `/app/main.js` or `/sw.js`, and `not_found_handling` is `"none"`

#### Scenario: the site isn't the app's
- GIVEN the routes of `web/wrangler.toml`
- WHEN a test matches `/`, `/guide/quickstart`, `/blog/`, `/blog/feed.xml`, `/privacy` and `/404.html` against them
- THEN none matches, and `/app/`, `/api/repos`, `/auth/login`, `/webhook`, `/_clm/invited.json` and `/sw.js` each match one

#### Serves
- capability/learn-clm
- architecture/invite-only-web

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
