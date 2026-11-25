---
component: web
base: { contract: "8a1658c773daa24a", design: "ea77d328ab1c7442" }
---

## MODIFIED Contract
A hosted web app at `https://clm-app.dev/app/`, served by a Cloudflare Worker (`web.worker`) that claims the app's paths on `clm-app.dev`, beside CLM's public site at the root, which `web.site` builds and deploys as a Worker of its own (`public-site`, `site-worker-of-its-own`), where invited people signed in with GitHub see the CLM repos they can read: the spec graph, the spec branches proposed for review, implementation progress and the backlog, each at a hash route `web.views/hash-routes` reads, which notifications and links name. It approves a spec branch by appending an `approved` record to the signed-in person's web ledger (`per-writer-ledgers`), records that person's choice to be notified about a repo as a `subscribed` or `unsubscribed` record there (`push-subscriptions-in-ledger`), takes a proposal for the backlog as a `proposed` record, which `clm rebase` files as a backlog item on the next build, and its withdrawal as a `withdrawn` record (`web-proposals-as-ledger-records`), records a comment on a spec branch's diff or a request that it change as a `commented` or `changes_requested` record, the latter holding the branch's land until the author pushes a change or the requester approves (`web-review-feedback-as-ledger-records`), records a review decision on a capability or an architectural constraint as a `decided` record (`review-capability-cost`), records a person's answer to a backlog `question` that lists its choices, or `done` on a `human-task`, as an `answered` record (`web-answers-as-ledger-records`), and writes nothing else to the repository. Open pages show a change within seconds of its push (`web-live-channel`), and the installed app can notify people when something waits on them. What it caches is derived from git and GitHub and can be deleted at any time (`web-derived-cache`). It works on phones and installs as a PWA.

**Invited users** (`invite-only-web`). Only these GitHub logins may sign in. Anyone else gets a "not invited" page, and sees nothing of any repository. The public site is open to everyone (`web/site-served-directly`). Adding someone is a change to this list; a request made through the early-access form (`sign-in-early-access`) grants nothing until a maintainer makes that change.

- `clunny`

## MODIFIED Design
The mockups in `assets/mockups/` are the reference for layout, colour, type and interaction: `assets/mockups/desktop.html` (sidebar, spec graph, PR document and graph tabs, implementation, backlog), `assets/mockups/mobile.html` (tab bar, PR review, graph with bottom sheet, offline implementation) and `assets/mockups/full-document.html` (a changed file in full). Their data is illustrative, and they show a light/system/dark theme switch the app doesn't have. Where they and the requirements disagree, the requirements win.

Four parts, the Worker as the child `web.worker`, the public site as the child `web.site`, and the app here, with the app's live socket, Notify me switch and push handlers in the child `web.client-live` and its views in the child `web.views`:
- `crates/clm-web` (`web.worker`): the Worker, in Rust with `workers-rs`, built for `wasm32-unknown-unknown` (`web-reuses-rust-in-wasm`). It serves the API, the GitHub sign-in, the webhook and the static assets this component builds. Its API is the contract between the two.
- `web/`: the browser app in TypeScript, whose `web/src/main.ts` imports `web.client-live`'s modules from `web/src/live/` and `web.views`' from `web/src/views/` and wires them to the page, built with Vite (`web-typescript-frontend`). Graph layout uses ELK (`elkjs`, layered, edges from dependent to dependency) and word-level diffs use `diff` (jsdiff). `web/package-lock.json` is committed. Unattended sessions add dependencies with `npm install --ignore-scripts` and run the app's checks with `npm ci`, `npm run`, `npm test` and `npx playwright test` (`unattended-commands`, `no-install-scripts`). Playwright's Chromium is installed outside the session: once per machine by the human, and by CI's browser-test step. `web/wrangler.toml` and `web/package.json`'s `build:worker` script build and deploy them as one Worker with assets, so the deploy stays here.
- `web/site/` (`web.site`): a package of its own, which the site workflow deploys as a Worker of its own (`site-worker-of-its-own`). `web`'s build leaves it alone (`app-at-app-path`); the capability harness builds and serves it for its own tests.

**Loading and refresh.** Everything the browser shows comes from the Worker, which alone holds the user's token. The app holds the repo list, and for the repo on screen its snapshot, its spec-branch list and each report once its view has drawn it, so a navigation within a repo draws at once while it asks again (`loading-indicator`). The endpoints a view is drawn from, which a refresh asks again (`live-refresh`): the snapshot and the spec-branch list for every repo view; the implementation state for Home, the graph, Build, Backlog, a backlog item's page and a PR's Graph tab; the branch's review on a PR page; the spec branches' reviews on Home, and on the graph once its inspector has loaded them; the capabilities report on Home and Capabilities, and the architecture report on Home and Architecture. The repo list is asked for on navigation only. Build's relative times are drawn against the browser's clock, so they stay current between redraws. In the full document, the current change is kept while its text remains a change, else the nearest change by position is. A notice only says the repo changed, and the 60-second tick runs regardless, so a page without a working socket stays current (`notice-refresh`).

**Views** (`web.views`). The pages the app draws are `web.views`' render functions, and `web.views`' Design says what each shows. This component routes to them, gives them the responses it holds and the callbacks that post, load again or navigate, and keeps what the user set in them across a refresh.

**Look.** Colour tokens on `:root` with a dark and a light palette, following the system's setting, one meaning per colour, the same as `view`'s charts (`colour-meanings`): green for active, converged, current, proven or complete; sky for unbuilt or ready; amber for claimed, behind, oversized or a modified element; rose for blocked, obsolete, a removed element or needs review; indigo for the accent and headings. The graph's two edge colours for a selected component, incoming and outgoing (`web.views/edge-direction`), are tokens of their own that mean nothing else. Inter for text and JetBrains Mono for IDs. On phones the graph pans and pinch-zooms.

**PWA.** Links from the app to the site leave the manifest's scope, so the installed app opens them in a browser tab over it. The service worker precaches the app shell.

**Size** (`slim-components`). This component's code budget is 80,000 tokens, above the default, because it is the shell of one browser app: `web/src/main.ts` routes every view, holds what they were drawn from and runs the one refresh they share, `web/src/styles.css` styles them all, and the end-to-end tests drive them through that shell. The views, which import nothing of the shell's, are the child `web.views`, as the Worker and the live socket, Notify me switch and push handlers are children of their own.

**Deploy.** The app is served at `https://clm-app.dev/app/` and the site at the root (`site-worker-of-its-own`). The `clm-app.dev` Custom Domain, on the zone the operator registered through Cloudflare Registrar, belongs to the site's Worker (`web.site/site-worker`); this Worker claims the app's paths on the same hostname as routes, which Cloudflare runs before the Custom Domain's Worker (`site-served-directly`). The `Replica` objects work on their own alarms (`web-repo-replica`), so nothing runs on a schedule (`worker-deploy`). A binding to a class the Worker doesn't export fails the deploy, and `web.worker` exports `Live` and `Replica`. The app routes by hash, so it needs no fallback page.

**Setup.** Nothing in the build or a deploy waits on it:
- the GitHub App exists, and its credentials are Worker secrets;
- the repo's Actions secrets hold `CLOUDFLARE_ACCOUNT_ID` and `CLOUDFLARE_API_TOKEN`, a token from the "Edit Cloudflare Workers" template limited to the operator's account and the `clm-app.dev` zone, with the zone's Workers Routes permission, which both Workers' deploys use.

To rotate a credential, generate a new private key or client secret in the App's settings and `wrangler secret put` it. `web/README.md` gives these commands.

**Tests** (`commands-live-in-their-crates`, `components-own-their-tests`). The browser tests under `web/e2e/` run the built app, with this component's entry file, stylesheet, routes and service worker, so what they show is this component's to state: they cite only its requirements. A view's own behaviour, drawn with the Worker faked, is `web.views`' and `web.client-live`'s to prove in their files. `.github/ISSUE_TEMPLATE/early-access.yml` is `ci`'s file, which web's test reads under a `clm:reads` marker (`sign-in-early-access`); a change to it that turns that test red is filed as a `ci` item (`path-ownership`). Each test that proves a requirement carries its own marker, none on a `describe` block, so no single test can be deleted unseen.

## ADDED Requirements

### Requirement: app-writes
While any write is in progress, an approve, a request for changes, a comment, a proposal, a withdrawal, an answer or a decision on a capability or an architectural constraint, the app SHALL run no refresh, and a refresh begun before one ended SHALL draw nothing it read. A 401 to any of these writes, or to a Notify me switch's `POST` or `DELETE` (`push-in-the-browser`), SHALL show the sign-in page, which no refresh then draws over. A write whose request fails at the network, the browser reporting online, SHALL say beside its button or form that the app is offline, and leave the button or form usable.

#### Scenario: a write in progress
- GIVEN a view on screen, and a Worker that holds its answer to the write in each of these cases: an approve, a request for changes, a comment, a proposal, a withdrawal, an answer and a capability's decision; and, apart, a refresh whose answer comes only after a decision has been drawn
- WHEN 60 seconds pass while the write is held, and then it is answered; and in the last, when the refresh's answer arrives
- THEN no refresh request was sent while the write was held; and the last page still shows the decision

#### Scenario: signed out
- GIVEN a Worker that answers 401 to the write in each of these cases: an approve, a request for changes, a comment, a proposal, a withdrawal, an answer and a capability's decision, and every other request answered as before but for the snapshot, which differs from the one drawn
- WHEN the write is made, and then 60 seconds pass
- THEN the sign-in page is shown, and still is

#### Scenario: not sent
- GIVEN a page whose Worker can't be reached for the write, its request failing at the network while the browser reports online, in each of these cases: an approve, a comment, a proposal, a withdrawal and an answer
- WHEN the write is made
- THEN its button or form says the app is offline, and stays usable

#### Serves
- capability/follow-from-browser
- architecture/ui-quality

### Requirement: worker-deploy
The app's Worker, as `wrangler` reads its configuration in `web/` for a deploy, SHALL have exactly three bindings, `LIVE` to the Durable Object class `Live` (`web.live`), `REPLICA` to the class `Replica` (`web.worker.replica`) and `ASSETS` to the built files (`invited-users`), and so no KV namespace; SHALL create `Live` and then `Replica` in migrations of their own under `new_sqlite_classes`, since the Free plan has only SQLite-backed Durable Objects; and SHALL have triggers holding an empty list of crons, so a deploy clears the Cron Trigger earlier deploys created and nothing calls the Worker on a schedule (a deploy leaves the deployed triggers as they are when `crons` is absent).

#### Scenario: binding
- GIVEN `web/`
- WHEN `wrangler` reads its configuration
- THEN its bindings are exactly `LIVE` with `class_name` `Live`, `REPLICA` with `class_name` `Replica`, and the assets bound as `ASSETS`, with no `kv_namespaces`; and one `[[migrations]]` entry's `new_sqlite_classes` holds `Live` and a later one's holds `Replica`

#### Scenario: the trigger is cleared
- GIVEN `web/`
- WHEN `wrangler` reads its configuration
- THEN `triggers.crons` is present and is an empty list

#### Serves
- capability/follow-from-browser

#### Needs
- web.worker/live-class-export
- web.worker/replica-class-export
- web.live/hub
- web.worker/cache-is-disposable

## MODIFIED Requirements

### Requirement: mobile-layout <!-- base: cc420aa7ec86c7a0 -->
Below 900 px wide, the app SHALL replace the sidebar with a bottom tab bar of five tabs, Home, Backlog, Graph, Build and More, where More lists `PRs`, `Capabilities` and `Arch.`; show one column with no horizontal scrolling; keep Approve visible on a PR; open the inspector as a bottom sheet that ends above the tab bar; show the Capabilities and Architecture lists as one card per entry and no table; and lay out a page's decision buttons as a 2×2 grid, Keep and Cut side by side and Redesign below Keep. At 900 px and wider it SHALL show no tab bar and a sidebar listing Home, Spec graph, Spec PRs, Implementation, Backlog, Capabilities and Architecture in that order, whose repository entry reads `<n> components · <current>/<total> current`, or `<n> components · coverage indexing` while coverage is indexing. Backlog's sidebar entry and tab SHALL be badged with the number of backlog items waiting on a person, and Spec PRs' sidebar entry and More's `PRs` with the number of spec branches, none badged at 0.

#### Scenario: phone
- GIVEN a 390 px wide viewport on a PR
- WHEN the page is shown
- THEN the tab bar is visible, the page doesn't scroll horizontally, and Approve is in view

#### Scenario: capabilities on a phone
- GIVEN a 390 px wide viewport
- WHEN the capabilities list is shown, and then a capability's page
- THEN the list shows one card per capability and no table, neither page scrolls horizontally, and the page's decision buttons, a 2×2 grid with Keep and Cut side by side and Redesign below Keep, can be scrolled into view and tapped

#### Scenario: architecture on a phone
- GIVEN a 390 px wide viewport
- WHEN the architecture list is shown, and then an entry's page
- THEN the tab bar holds Home, Backlog, Graph, Build and More, More lists `PRs`, `Capabilities` and `Arch.`, the list shows one card per entry, neither page scrolls horizontally, and the page's decision buttons can be scrolled into view and tapped

#### Scenario: home on a phone
- GIVEN a 390 px wide viewport and a Home listing a question with choices
- WHEN Home is shown
- THEN the page doesn't scroll horizontally, and the question's answer buttons are in view or can be scrolled into view and tapped

#### Scenario: wide screen
- GIVEN a 1280 px wide viewport on the graph of a repo of 3 components, with coverage complete; and, apart, with coverage indexing
- WHEN the page is shown
- THEN the sidebar lists the seven views in order and the tab bar is hidden, and the repository entry reads `3 components · <current>/<total> current` in the first and `3 components · coverage indexing` in the second

#### Scenario: badges
- GIVEN a snapshot with one backlog item waiting on a person, and two spec branches
- WHEN the graph is shown at 1280 px, and at 390 px with More open
- THEN the sidebar's Backlog reads `1` and Spec PRs `2`; the tab bar's Backlog reads `1` and More's `PRs` `2`

#### Scenario: bottom sheet
- GIVEN a 390 px wide viewport on the graph
- WHEN a component is tapped
- THEN the inspector opens as a bottom sheet whose bottom edge is at or above the tab bar's top

#### Needs
- web.views/home-waiting-on-you
- web.views/answer-buttons

### Requirement: installable <!-- base: e711892d6876c16a -->
The app SHALL be installable as a PWA: its manifest SHALL name it `CLM`, with `display` `standalone`, `id`, `start_url` and `scope` `/app/`, and 192 and 512 px icons served with their types, and its service worker's scope SHALL be `/app/`, never answering or caching a request outside `/app/`. When the browser is offline it SHALL show the last data it loaded for a repo under the banner `Offline · showing data from <age>`, or `Offline` alone when it holds none; and while the browser is offline or the view is drawn from the copy kept for offline use, it SHALL disable every write: Approve, Request changes, Comment, New proposal, Withdraw, the answer buttons and a capability's and a constraint's decision buttons.

#### Scenario: offline launch
- GIVEN a repo whose PR, item page, Backlog, capability page and constraint page were each viewed once online
- WHEN the app is reloaded offline on each
- THEN it shows that view's last data under `Offline · showing data from <age>`, and Approve, Request changes, Comment, the answer buttons, New proposal, Withdraw and the decision buttons are disabled

#### Scenario: app-only scope
- GIVEN the built app
- WHEN a test reads its manifest and its icons, and registers its service worker
- THEN the manifest names `CLM` with `display` `standalone`, its `id`, `start_url` and `scope` are `/app/`, its 192 and 512 px icons are answered 200 with their types, the service worker's scope is `/app/`, and a request for `/guide/` is not answered from its caches

#### Scenario: nothing kept
- GIVEN a repo never opened on this device
- WHEN the app is opened on it offline
- THEN the banner reads `Offline`

### Requirement: no-install-scripts <!-- base: d8da6553966d1127 -->
Installing `web/`'s dependencies SHALL run no dependency's install script: npm, run there with no `ignore-scripts` on its command line or in its environment, SHALL take `ignore-scripts` as `true` from the package's own settings, whatever the user's npm settings say.

#### Scenario: npmrc
- GIVEN `web/`, no `ignore-scripts` in the environment, and a user npm settings file that sets `ignore-scripts=false`
- WHEN `npm config get ignore-scripts` runs there
- THEN it prints `true`

### Requirement: invited-users <!-- base: 0406720d1038b9d6 -->
The app's build SHALL write the GitHub logins of this component's Contract's invited-users list, read from `specs/web/SPEC.md` at build time, to `_clm/invited.json` beside the built app, failing when the Contract lists no login, and the app's Worker, as `wrangler` reads its configuration in `web/`, SHALL bind the built files as `ASSETS` and send `/_clm/*` to its script, so the Worker completes sign-in only for those logins (`web.worker/invited-sign-in`) and the list is never served as a file.

#### Scenario: list from the spec
- GIVEN a build of the app
- WHEN a test reads its invited logins; and, apart, the invited logins are read from a spec whose list holds no login
- THEN they are exactly the logins listed in `specs/web/SPEC.md`'s Contract; the second read fails

#### Scenario: the list reaches only the Worker
- GIVEN `web/`
- WHEN `wrangler` reads its configuration
- THEN its assets are bound as `ASSETS`, and its `run_worker_first` sends `/_clm/invited.json` to the script

#### Needs
- web.worker/invited-sign-in

### Requirement: live-refresh <!-- base: 28e7b29d7eb0b8a9 -->
While the page is visible, the app SHALL ask the Worker again, about every 60 seconds and whenever the page becomes visible, for the endpoints the view on screen was drawn from, and SHALL make no such requests while it is hidden. When the answers differ from what is shown, ignoring the implementation state's `now`, it SHALL redraw the view in place, keeping what the user set in it: the graph's pan, zoom and selected component, the page's scroll position, its tabs, switches and filters, an open bottom sheet and the full document's current change; a selected component the new state lacks SHALL be deselected, closing the inspector. When they don't, it SHALL leave the view as it is. A refresh that fails SHALL leave the view as it is, drawing nothing from the copy kept for offline use, but for the age `honest-errors` shows once refreshes have failed for 3 minutes; a 401 shows the sign-in page.

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
- GIVEN the spec graph on screen, and a Worker that answers the next refresh with 502; and, apart, a refresh that fails at the network; and, apart, one answered 401
- WHEN the app refreshes
- THEN in the first two the graph is unchanged and no offline banner shows; in the third the sign-in page is shown

#### Scenario: a component removed
- GIVEN the graph with `b` selected and its inspector open, and a snapshot that no longer holds `b`
- WHEN the app refreshes
- THEN no component is selected and the inspector is closed

#### Scenario: filters and the full document
- GIVEN the Backlog filtered to one component; and, apart, a file in full on its second change
- WHEN the app refreshes with changed answers
- THEN the filter is still on, and the full document is still on its second change

### Requirement: approve-by-branch <!-- base: 9a6b33228e508523 -->
Approve SHALL post `{"fingerprint": <f>}`, `<f>` the fingerprint of the review on screen, to the Worker's approve route for that branch. On success it SHALL say `Approved.`, load the review again and show the approval from its approvals, not from a label. On a 409 it SHALL say the branch changed since the review was shown, show no approval, and load the review again; on a 422 whose `error` is `signed`, it SHALL say `Not approved: this repository counts only approvals signed with your SSH key.` and load the review again, so its signed note (`web.views/signed-approvals`) shows; on any other status but a 401 (`app-writes`) it SHALL say `Not approved (<status>).` and leave Approve enabled. Each load SHALL come from the Worker, never the copy kept for offline use, and be drawn with the message still shown, so the next Approve pins what is then on screen (`approve-pins-the-review`). A load that fails SHALL keep the outcome and add that the review should be reloaded, and after a 409 say its new review could not be loaded, drawing no kept copy. When Approve is disabled is `web.views/approve-button`'s to state.

#### Scenario: approve
- GIVEN `spec/x`'s review with fingerprint `f1` and no approvals, and a Worker that answers the approve with 200 and then the review with an approval by `clunny/web`
- WHEN the reviewer taps Approve
- THEN one request posts `{"fingerprint": "f1"}` to `/branches/spec/x/approve`, the review is requested again, the header names `clunny/web`, and Approve is disabled

#### Scenario: branch moved
- GIVEN `spec/x`'s review drawn with fingerprint `f1`, and a Worker that answers that approve with 409 and the review with fingerprint `f2`
- WHEN the reviewer taps Approve
- THEN the status says the branch changed since the review was shown, no approval is shown, and the review on screen is the one with fingerprint `f2`

#### Scenario: became signed
- GIVEN `spec/c`'s review drawn with `signed` false and no signed note, and a Worker that answers the approve with 422 and `error` `signed`, and then the review with `signed` true
- WHEN the reviewer taps Approve
- THEN the review is requested again, the note shows `clm approve spec/c`, and Approve is disabled and doesn't read "Approved"

#### Scenario: another status
- GIVEN a Worker that answers the approve with 500
- WHEN the reviewer taps Approve
- THEN the status says `Not approved (500).` and Approve is enabled

#### Scenario: the review can't be reloaded
- GIVEN a Worker that answers the approve with 409 and the review then with 502; and, apart, the same with the review failing at the network; and, apart, one that answers the approve with 200 and the review then with 502
- WHEN the reviewer taps Approve in each
- THEN the first two say the branch changed and its new review could not be loaded, and draw no kept copy; the third says `Approved.` and to reload to see the review as it is now

#### Needs
- web.views/worker-requests
- web.views/approve-button
- web.views/signed-approvals
- web.worker.review/approve

### Requirement: loading-indicator <!-- base: bc3e841a091932b2 -->
The app SHALL show a loading bar from the moment a navigation starts, the first load included, until the view navigated to is drawn, or until the browser has been sent on to the first repo's view, and not during background refreshes or the redraw that going online or offline makes. It SHALL draw a view of the repo on screen from the repo list, snapshot, PRs, capabilities report and architecture report it already holds without waiting for them to be fetched again; a navigation to any view, Capabilities and Architecture included, shows the bar.

#### Scenario: slow view
- GIVEN the Worker holds its answer for the view navigated to, in each of these cases: a PR, Capabilities, Architecture, and the app's root, which sends the browser on to the first repo
- WHEN the user navigates to it
- THEN the loading bar is visible while the answer is pending, and is gone once the view is drawn

#### Scenario: between views of a repo
- GIVEN the spec graph of `o/a` is on screen, Capabilities and Architecture were drawn once, and the Worker stops answering
- WHEN the user opens `o/a`'s Backlog, Capabilities and Architecture
- THEN the backlog is drawn from the snapshot already held, and Capabilities and Architecture from the reports held, while the repo list held is drawn and asked for again

#### Scenario: graph before implementation
- GIVEN the snapshot is answered and the implementation state isn't yet
- WHEN the spec graph is opened
- THEN the graph is drawn

#### Scenario: background refresh
- GIVEN the graph on screen, and a Worker that holds its answers to the next refresh
- WHEN 60 seconds pass
- THEN the loading bar stays hidden

#### Scenario: online and offline
- GIVEN the graph on screen
- WHEN the browser goes offline and then online
- THEN the loading bar stays hidden and the view is drawn again each time

### Requirement: graph-fills-height <!-- base: 7acb0b89ca1d1f22 -->
The graph's canvas, on the spec graph and on a PR's Graph tab, SHALL extend from its top to the bottom of the viewport, or below 900 px to the top of the tab bar, or of the Approve bar on a PR, as drawn with whatever it holds, and SHALL be at least 320 px tall; on desktop the inspector beside it SHALL scroll within that height.

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
- WHEN a PR's Graph tab is shown, and again after Approve has been tapped and the review drawn anew
- THEN each time the canvas's bottom edge is within 16 px of the Approve bar's top

#### Scenario: a short window
- GIVEN a viewport 400 px tall
- WHEN the spec graph is shown
- THEN the canvas is 320 px tall

### Requirement: notice-refresh <!-- base: 1ead25d58a6f2800 -->
While the page is visible and online, the app SHALL give `web.client-live`'s socket the repo's live route, `/api/repos/<o>/<r>/live`, opened as a browser `WebSocket`, for the repo on screen, and none while the page is hidden, another repo is shown or the sign-in page is on screen. On each notice the socket hands over it SHALL refresh at once, as `live-refresh` refreshes, with every request carrying `fresh` set to the notice's `delivery`, running at most one refresh at a time: notices that arrive during a refresh SHALL share one refresh after it, carrying the latest one's `delivery`, and a notice that arrives during a write SHALL refresh once the write ends (`app-writes`). The 60-second refresh SHALL go on as before.

#### Scenario: a claim shows within seconds
- GIVEN Build on screen for `o/a` with its socket open, and the Worker starting to answer with component `b` claimed
- WHEN the socket receives `{"delivery": "d1"}`
- THEN within 2 seconds, with no timer advanced, Build shows `b` claimed, and the implementation request carried `fresh=d1`

#### Scenario: hidden
- GIVEN the spec graph on screen with its socket open
- WHEN the page is hidden for 3 minutes and then shown
- THEN the socket was closed while hidden, and a new one is opened when it is shown

#### Scenario: no socket
- GIVEN the graph of `o/a` with its socket open
- WHEN `o/b` is opened; and, apart, the sign-in page is shown; and, apart, the browser goes offline
- THEN `o/a`'s socket is closed, and none but `o/b`'s is open in the first, and none in the other two

#### Scenario: notices during a refresh
- GIVEN a refresh held by the Worker, and, apart, an approval held by the Worker
- WHEN notices `d1` and then `d2` arrive while each is held, and then it is answered
- THEN one refresh follows, and its requests carry `fresh=d2`

#### Needs
- web.views/worker-requests
- web.client-live/live-updates
- web.worker/fresh-live-state
- web.worker/live-socket

### Requirement: fresh-after-write <!-- base: 8dea89096bea9cad -->
Every request the app makes to load data again after the Worker answered one of its writes (an approve, a Request changes, a comment, a proposal, a withdrawal, an answer to a backlog item, or a decision on a capability or an architectural constraint), on each response after which it loads again, success or refusal, SHALL carry `fresh` set to a delivery of the app's own, a random UUID new for each write, so the Worker answers from live state fetched anew rather than its minute-old copy. The copy kept for offline use SHALL be stored under the request's path without `fresh`, as for a notice's refresh.

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

### Requirement: app-at-app-path <!-- base: d9141ff850a1b7e6 -->
`web`'s build SHALL put the app, built with Vite's `base` `/app/`, at `web/dist/app/`, the invited list at `web/dist/_clm/invited.json` and the retiring service worker at `web/dist/sw.js`, and SHALL NOT build `web.site`, which deploys on its own (`site-worker-of-its-own`). Opened at `/app/#/<owner>/<repo>/<view>`, the app SHALL show that view, and its sign-in button SHALL lead to `/auth/login`.

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

### Requirement: retire-root-service-worker <!-- base: df279fd83010ca70 -->
The build SHALL put at `/sw.js` a service worker that, once installed in place of the app's old one at scope `/`, unregisters itself and drops its caches, and the app at `/app/` SHALL unregister any service worker registration whose scope is the site's root, so no browser that installed the app before the move keeps serving the old app over the site.

#### Scenario: old install
- GIVEN a browser holding the old service worker at scope `/`, with the old app shell cached
- WHEN it fetches `/sw.js` again and then opens `/`
- THEN the registration at scope `/` is gone, its caches are deleted, and `/` is answered by the network, not the old app shell

#### Scenario: the app clears it
- GIVEN a registration at scope `/` and the app opened at `/app/`
- WHEN the app starts
- THEN the registration at scope `/` is unregistered and the app's own at `/app/` remains

### Requirement: latest-navigation-wins <!-- base: d93cb8bd77d3af53 -->
Opened with no repo in its route, the app SHALL send the browser on to the spec graph of the first repo the Worker's repo list names, replacing the route in the history, or, when the list is empty, show a page saying `No CLM repos yet` that links to installing the GitHub App. Whatever a navigation finds when its requests answer (a view to draw, the first repo to send the browser on to, the install page, an error, the sign-in page), it SHALL draw or do it only while no later navigation has started; one that a later navigation overtook does nothing, so the screen and the route are always the latest navigation's.

#### Scenario: opened at the app's root
- GIVEN a Worker whose repo list names `o/a` then `o/b`
- WHEN the app is opened at `/app/` with no route
- THEN the route becomes `#/o/a/graph` and `o/a`'s spec graph is drawn

#### Scenario: navigated while the repo list is pending
- GIVEN the app opened at `/app/` with no route, and the Worker's answer to `/api/repos` held back
- WHEN the route changes to `#/o/a/prs/spec%2Fx` and the answer then arrives
- THEN the route stays `#/o/a/prs/spec%2Fx` and that spec branch's review is drawn, not the graph

#### Scenario: no repos
- GIVEN a Worker whose repo list is empty
- WHEN the app is opened at `/app/` with no route
- THEN the page says `No CLM repos yet` and links to installing the GitHub App, and the route is unchanged

### Requirement: honest-errors <!-- base: 444ce2bc9bb1d763 -->
When a request for a repo's view fails, the app SHALL say what failed, never more than it knows, and keep what it can show:
- a 403 or a 404 SHALL show that the repo isn't readable with the person's GitHub account, or isn't covered by the App, but a 404 for a spec branch's review SHALL say the branch is no longer open, linking to the PR list;
- any other 4xx but a 401 SHALL say the request failed, with its status, and nothing about access;
- a 503 `{"preparing": <share>}` SHALL show that CLM is preparing the repo, with the share done, held between 0 and 100 %, and ask again every 5 seconds until it draws the view;
- any other 5xx or a network failure SHALL show the copy kept for offline use under the banner `CLM couldn't be reached · showing data from <age>`, when there is one, and otherwise that CLM couldn't load the repo just now and is retrying, and SHALL ask again with backoff, 5, 10, 20, then every 60 seconds, drawing the view once a request succeeds;
- a spec-branch list that fails SHALL never be drawn as no spec branches: the PR list and Home SHALL say the list couldn't be loaded, and the other views draw without it;
- a view that can't be drawn from what the Worker answered SHALL say so, and not that CLM is retrying.

A view on screen whose refreshes have failed for 3 minutes SHALL stay on screen under the banner `Couldn't refresh · showing data from <age>`, until a refresh succeeds.

#### Scenario: busy, nothing kept
- GIVEN a repo never opened on this device, and a Worker that answers its snapshot 503 `{"error": "unavailable"}` twice, then the snapshot
- WHEN the person opens the repo's graph
- THEN the page says CLM couldn't load the repo and is retrying, says nothing about access, and then draws the graph without a reload

#### Scenario: busy, kept copy
- GIVEN a repo whose snapshot was kept an hour ago, and a Worker that answers 502
- WHEN the person opens the repo's graph
- THEN the graph is drawn from the kept copy under `CLM couldn't be reached · showing data from 1 h ago`

#### Scenario: preparing
- GIVEN a Worker that answers the snapshot 503 `{"preparing": 0.4}`, then the snapshot; and, apart, `{"preparing": 1.7}`
- WHEN the person opens the repo's graph
- THEN the page shows that the repo is being prepared, 40% done, and then draws the graph; the second shows 100% done

#### Scenario: no access
- GIVEN a Worker that answers the snapshot 404
- WHEN the person opens the repo
- THEN the page says the repo isn't readable with their account or isn't covered by the App

#### Scenario: other refusals
- GIVEN a Worker that answers the snapshot 403; and, apart, 429
- WHEN the person opens the repo
- THEN the first says the repo isn't readable with their account or isn't covered by the App; the second says the request failed with 429 and nothing about access

#### Scenario: a branch that's gone
- GIVEN a Worker that answers `spec/gone`'s review 404
- WHEN `#/o/a/prs/spec%2Fgone` is opened
- THEN the page says the branch is no longer open, links to `#/o/a/prs`, and says nothing about access

#### Scenario: no branch list
- GIVEN a Worker that answers the spec-branch list 502 and the snapshot as usual
- WHEN `#/o/a/prs`, then Home, then the graph are opened
- THEN the PR list and Home say the spec branches couldn't be loaded, and the graph is drawn

#### Scenario: a view that can't be drawn
- GIVEN a Worker that answers the snapshot 200 with a body the graph can't be drawn from
- WHEN the person opens the repo's graph
- THEN the page says the view couldn't be drawn, and not that CLM is retrying

#### Scenario: refreshes keep failing
- GIVEN the graph on screen, and a Worker that answers every refresh 503 for 3 minutes
- WHEN the refreshes run
- THEN the graph stays, unchanged, under `Couldn't refresh · showing data from <age>`

#### Needs
- web.worker/views-from-replica
- web.views/worker-requests

### Requirement: site-served-directly <!-- base: 3c4d374a5eb37fb4 -->
The app's Worker, as `wrangler` reads its configuration in `web/`, SHALL be named `clm`, with its assets the directory `./dist`, and routed on `clm-app.dev` exactly the app's paths, `/app`, `/app/*`, `/api/*`, `/auth/*`, `/webhook`, `/_clm/*` and `/sw.js`, with no Custom Domain, so every other path reaches the site's Worker (`web.site/site-worker`); its `run_worker_first` SHALL send to the script exactly `/api/*`, `/auth/*`, `/webhook`, `/_clm/*` and `/app`, so its assets answer the app's files and `/sw.js` with no script run, and `not_found_handling` SHALL be `"none"`, so a missing path under `/app/` gets a plain 404.

#### Scenario: who answers what
- GIVEN `web/`
- WHEN `wrangler` reads its configuration
- THEN its routes are exactly those seven patterns on `clm-app.dev`, none is a Custom Domain, its `run_worker_first` sends `/api/repos`, `/auth/login`, `/webhook`, `/_clm/invited.json` and `/app` to the script and not `/app/`, `/app/main.js` or `/sw.js`, its assets are the directory `./dist`, the Worker's name is `clm`, and `not_found_handling` is `"none"`

#### Scenario: the site isn't the app's
- GIVEN the routes `wrangler` reads from `web/`
- WHEN a test matches `/`, `/guide/quickstart`, `/blog/`, `/blog/feed.xml`, `/privacy` and `/404.html` against them
- THEN none matches, and `/app/`, `/api/repos`, `/auth/login`, `/webhook`, `/_clm/invited.json` and `/sw.js` each match one

#### Needs
- web.worker/invited-sign-in

### Requirement: sign-in-early-access <!-- base: 50c354b16100ca14 -->
The sign-in page's logo SHALL be a link to `/`. The page SHALL say that CLM works fully without the web app by following the quickstart, linking to `/guide/quickstart`, and that the web app is an optional extra for convenience, in early access, and SHALL link to Request early access at `https://github.com/clunny/clm/issues/new?template=early-access.yml`, the one constant in the app that names it. Sign in with GitHub and Install the GitHub App SHALL stay as they are. The form that link opens, this repository's `.github/ISSUE_TEMPLATE/early-access.yml`, SHALL be a GitHub issue form named `Request early access` that adds the label `early-access`, whose description says the web app is invite-only and in early access, and that CLM works fully without it by following the quickstart at `https://clm-app.dev/guide/quickstart`, with exactly one required field, asking what the requester would like to use the web app for, and SHALL ask for no email address (`early-access-by-issue-form`). Only GitHub renders the form, so it is proven by reading the template.

#### Scenario: signed out
- GIVEN a Worker that answers the repo list 401
- WHEN the app is opened
- THEN the sign-in page's logo links to `/`, its text says CLM works without the web app and that the web app is an optional extra in early access, it links to `/guide/quickstart` and to `https://github.com/clunny/clm/issues/new?template=early-access.yml` with a link reading `Request early access`, and Sign in with GitHub still leads to `/auth/login`

#### Scenario: the form
- GIVEN this repository's `.github/ISSUE_TEMPLATE/early-access.yml`
- WHEN it is parsed as YAML
- THEN it parses, its `name` is `Request early access`, its `labels` hold `early-access`, its `description` holds `invite-only`, `early access`, `works fully without it` and `https://clm-app.dev/guide/quickstart`, its body holds exactly one field with `validations.required` true, and no text in it holds `email`

#### Serves
- capability/learn-clm
- architecture/invite-only-web

### Requirement: home-first <!-- base: 3681eee15812ddb1 -->
The app SHALL open a repo at `#/<owner>/<repo>` on Home (`web.views/home-waiting-on-you`), list Home first in the sidebar and the tab bar, badge it with the number of entries waiting, route `#/<owner>/<repo>/backlog/<slug>` to that item's page (`web.views/backlog-item-page`), and refresh Home and an item's page as `live-refresh` refreshes, keeping the page's scroll. Home SHALL draw from the snapshot at once and add each section as the response it needs arrives.

#### Scenario: a repo opens on Home
- GIVEN a signed-in person with repo `o/a`
- WHEN they choose `o/a` in the repo switcher
- THEN the address is `#/o/a`, Home is shown, and it is the sidebar's first view

#### Scenario: an item's page
- GIVEN repo `o/a` whose backlog holds `q`
- WHEN `#/o/a/backlog/q` is opened
- THEN `q`'s page is shown

#### Scenario: badged
- GIVEN repo `o/a` with a human task and a spec branch's review waiting on the person
- WHEN Home is shown at 1280 px, and at 390 px
- THEN Home's sidebar entry reads `2`, and so does its tab

#### Scenario: sections arrive
- GIVEN a Worker that holds its answers for the implementation state and both reports
- WHEN `#/o/a` is opened, and then the capabilities report is answered
- THEN Home is drawn with its questions before any answer, with no flagged section, and the flagged section is added once the report arrives

#### Needs
- web.views/home-waiting-on-you
- web.views/backlog-item-page

### Requirement: markdown-styles <!-- base: cf566a2219e40e03 -->
Text rendered from the repository's Markdown SHALL style inline code and code blocks in JetBrains Mono, and headings, lists, tables, quotes and links, and a code block or a table wider than its box SHALL scroll within that box, so the page never scrolls horizontally because of it.

#### Scenario: a long code line on a phone
- GIVEN a 390 px wide viewport and a page drawn from the repository's Markdown, such as a capability's, whose text holds a 300-character code block and a five-column table
- WHEN the page is shown
- THEN the page doesn't scroll horizontally, and the code block scrolls within its box

#### Needs
- web.views/repo-markdown

### Requirement: answer-from-app <!-- base: 074f7d4cbb0ebcca -->
On Home and a backlog item's page, the answer buttons (`web.views/answer-buttons`) SHALL show the answers of the implementation state, none until it arrives, SHALL be disabled offline (`installable`), and a tap SHALL run as a write (`app-writes`). A tap SHALL post `{"answer": <answer>}` to `POST /api/repos/<o>/<r>/backlog/<slug>/answer`; on 200 it SHALL ask for the implementation state again (`fresh-after-write`) and redraw the view on screen from it, keeping its scroll; on 422 it SHALL say `The answer was refused.`, on 404 `That item is no longer in the backlog.`, and on any other status but a 401 (`app-writes`) `The answer wasn't recorded (<status>).`, each as text under the buttons, asking for nothing again.

#### Scenario: answered on the item's page
- GIVEN repo `o/a` whose backlog holds question `q` with choices `yes` and `no`, and a Worker that answers the answer with 200 and then the implementation state with the answer `yes` by `alice/web`, `mine` true
- WHEN `#/o/a/backlog/q` is opened and `yes` is tapped
- THEN exactly one request posted `{"answer": "yes"}` to `/api/repos/o/a/backlog/q/answer`, the implementation state was requested again with `fresh`, and `yes` is drawn chosen

#### Scenario: answered on Home
- GIVEN the same repo, with Home on screen
- WHEN `yes` is tapped under `q`
- THEN one request posted `{"answer": "yes"}` to `q`'s route, and Home draws `yes` chosen

#### Scenario: refused
- GIVEN `q`'s page, and a Worker that answers the answer with 422 `{"error": "answer"}`; and, apart, 404; and, apart, 500
- WHEN `yes` is tapped in each
- THEN a message under the buttons says `The answer was refused.`, `That item is no longer in the backlog.` and `The answer wasn't recorded (500).` in turn, the buttons stay enabled, and no implementation state was requested after the post

#### Scenario: offline
- GIVEN `q`'s page drawn from the copy kept for offline use, offline
- WHEN the page is shown
- THEN its answer buttons are disabled

#### Needs
- web.views/answer-buttons
- web.views/home-waiting-on-you
- web.views/backlog-item-page
- web.worker/answers

### Requirement: cost-views-in-the-browser <!-- base: 34e19461763831ae -->
The built app, opened in a browser with the Worker's API answered, SHALL show the Capabilities and Architecture views and their pages as `web.views` draws them (`web.views/capabilities-list`, `architecture-list`, `decide-buttons`, `architecture-decide-buttons`), under this component's stylesheet and routes: in a 1280 px wide window each list is a table and no card shows; a proof `held <c>/<n>` with `c` less than `n` is drawn in the theme's amber; Architecture is the sidebar's last view, after Capabilities; a row opens its page at `#/<owner>/<repo>/capabilities/<slug>` or `#/<owner>/<repo>/architecture/<slug>`, with the view's sidebar entry marked; and a decision posts once however often it is tapped while in flight, after which the report is asked for again from the Worker with `fresh`, never the copy kept for offline use, and the page shows what the Worker then answers, never an older report whose request began before the decision.

#### Scenario: the capabilities table
- GIVEN a 1280 px wide window, and a report listing `b` then `a`, with `a` flagged
- WHEN `#/o/a/capabilities` is opened, "⚑ only" is turned on, and `a`'s row is chosen
- THEN the table lists `b` first and no card is visible, then only `a` is listed, and the page at `#/o/a/capabilities/a` shows `a`'s entry with Capabilities marked in the sidebar

#### Scenario: its own view
- GIVEN a repo on screen at 1280 px
- WHEN the sidebar is shown
- THEN Architecture is listed after Capabilities, and choosing it opens `#/<owner>/<repo>/architecture`

#### Scenario: the architecture table
- GIVEN a 1280 px wide window, and a report listing `e` (flagged, one of its two enforcing rules current, with `#### Untested`) then `g` (holding only `#### Untested`)
- WHEN `#/o/a/architecture` is opened, "⚑ only" is turned on, and `e`'s row is chosen
- THEN `e` comes first and reads `held 1/2 + untested` in the theme's amber, `g` reads `untested`; then only `e` is listed; and the page at `#/o/a/architecture/e` shows `e`'s entry

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
- GIVEN a Worker that answers a decision with 409 and the report with the entry reworded at a new version; and, apart, one that answers 422 with `error` `signed` and a `command`; and a report whose `signed` is true; and, apart, a report kept for offline use and a Worker that answers a decision with 200 and the report then with 502
- WHEN a decision is tapped on a capability's and on a constraint's page for the first, and on a capability's page for the second, a constraint's page is shown for the third, and a decision is tapped on a capability's and on a constraint's page for the fourth
- THEN the first says the entry changed since it was shown and draws it as reworded, and on the capability's page the next tap posts the new version; the second shows the Worker's `command` with the buttons disabled; the third shows the buttons disabled with `clm architecture decide e --value high|medium|low --decision keep|cut|redesign|accept`; and the fourth draws no kept copy and says its report could not be loaded again

### Requirement: review-in-the-browser <!-- base: 7df79f16a4566c53 -->
The built app, opened in a browser with the Worker's API answered, SHALL show the PR list and a spec branch's review as `web.views` draws them (`web.views/spec-branch-review`, `full-document`, `signed-approvals`, `approve-button`, `comment-on-review`, `request-changes`), under this component's stylesheet and routes: a branch opens at `#/<owner>/<repo>/prs/<branch>` with its review loaded from the Worker's branch route, and no request goes to a `/pulls` route from the review, the graph or Build; a file opened in full is scrolled so its first change is in the window, and `j` brings the next into it, while a path the review doesn't change says `<path> isn't changed in <branch>.`; on a phone the signed note sits in the Approve bar, in the window and above the tab bar, Request changes, Approve and the request's form are wholly in the window, and the page never scrolls sideways. A comment SHALL post `{"fingerprint", "target", "text"}`, the review's fingerprint, `null` for the intent or the requirement's ID for a card, and the text, to the Worker's comments route for the branch, and a request for changes `{"fingerprint", "text"}` to its request-changes route; Approve, Comment and Request changes each post once however often they are tapped while in flight. On success a comment or request SHALL load the review again from the Worker, never the copy kept for offline use, and draw it once the form has closed, or say `Sent. Reload to see the review as it is now.` when that load fails; on a 409 it SHALL say the branch changed since the review was shown, load the review again and keep the form's text, so the next send pins the review then on screen; on a 403 it SHALL say commenting needs read access, or requesting changes needs write access; on a 422 that the Worker refused the field its `error` names, or the request when it names none; and on any other status but a 401 (`app-writes`) `Not sent (<status>).`, each leaving the form usable.

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

#### Scenario: a file not in the review
- GIVEN `spec/c`'s review, which doesn't change `README.md`
- WHEN `#/o/a/prs/spec%2Fc/full/README.md` is opened
- THEN the page says `README.md isn't changed in spec/c.`

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

#### Scenario: branch moved
- GIVEN the review drawn with fingerprint `f1`, and a Worker that answers the request with 409 and the review with fingerprint `f2`
- WHEN the reviewer submits a request, and then again
- THEN the status says the branch changed since the review was shown, the review on screen is the one with fingerprint `f2`, the text is kept, and the next submit posts `f2`

#### Scenario: a request the Worker doesn't take
- GIVEN a Worker that answers the request with 403; and, apart, a comment with 403; and, apart, a request with 422 naming no field; and, apart, a request with 500
- WHEN each is sent
- THEN the first says write access is needed, the second read access, the third that the Worker refused the request, the fourth `Not sent (500).`, and each leaves the form usable with its text

#### Scenario: Request changes on a phone
- GIVEN a 390 px wide touch window on `spec/c`'s review
- WHEN Request changes is tapped
- THEN Request changes and Approve were wholly in the window, in the Approve bar, the form's send button is wholly in the window, and the page is no wider than the window

### Requirement: graph-in-the-browser <!-- base: 1d6b482897a95bf9 -->
The built app, opened in a browser with the Worker's API answered, SHALL show the graph and its inspector as `web.views` draws them (`web.views/graph-view`, `oversized-component`, `inspector`, `inspector-overlay`), under this component's stylesheet and routes, under a crumb naming the snapshot's `repo`: a queue-state ring is a solid outline in the theme's colour for the state, and an oversized component's outline is the theme's amber until a ring colours it; the state word stays inside its card however long the component's ID is; a graph opened with the implementation state already loaded by another view is ringed at once; a PR's Graph tab, at `#/<owner>/<repo>/prs/<branch>/graph`, marks only the components the PR touches; the inspector opens over the graph as a drawer from the window's right edge, at most 640 px wide and 90% of the window and no taller than the window, never narrowing the canvas; and with no component selected, the inspector's place says `Select a component to see its contract, changes, work and backlog.`

#### Scenario: queue states
- GIVEN an implementation state with `b` claimed and `a` blocked
- WHEN `#/o/a/graph` is opened
- THEN the crumb reads `o/a`, `b` names `claimed` and is outlined solid in the theme's amber, and `a` names `blocked` and is outlined solid in the theme's rose

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
- THEN × closes it and clears the selection and its highlighted edges, and the inspector's place says `Select a component to see its contract, changes, work and backlog.`; the first tap opens it, and the second closes it

#### Needs
- web.worker/snapshot-names-its-repo

### Requirement: backlog-in-the-browser <!-- base: 5eb37285b87481c7 -->
The built app, opened in a browser with the Worker's API answered, SHALL show the Backlog as `web.views` draws it (`web.views/backlog-view`, `backlog-taken`, `propose-from-backlog`), under this component's routes: the backlog is drawn from the snapshot before the implementation state arrives and takes its marks when it does; and an item opens its page at `#/<owner>/<repo>/backlog/<slug>`, from the Backlog and from the inspector, and a slug the backlog doesn't hold says so with a link back. New proposal SHALL post `{"title", "body", "nodes": []}` to the Worker's proposals route once however often it is submitted while in flight; on 200 it SHALL ask for the implementation state again (`fresh-after-write`) and draw the view from it, on a 409 say `That title is taken: choose another.`, on a 422 that the Worker refused the field its `error` names, or the request when it names none, and on any other status but a 401 (`app-writes`) `Not proposed (<status>).`, keeping the form. Withdraw SHALL send `DELETE` to `/api/repos/<o>/<r>/proposals/<id>`, on success ask for the implementation state again, and on any other status but a 401 say `Not withdrawn (<status>).` under the proposal.

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
- GIVEN a Worker that answers the proposal with 409; and, apart, one that answers 422 naming `nodes`; and, apart, 422 naming nothing; and, apart, 500
- WHEN a proposal is submitted in each
- THEN the first keeps the form with its title, saying the title is taken, the second says the Worker refused the nodes, the third the request, and the fourth `Not proposed (500).`

#### Scenario: withdraw
- GIVEN Proposed listing `p1` with `mine` true and `p2` with `mine` false
- WHEN `p1`'s Withdraw is tapped
- THEN `p2` shows no Withdraw, one `DELETE` went to `/api/repos/o/a/proposals/p1`, the implementation state was requested once more, and `p1` is gone

#### Scenario: a withdrawal refused
- GIVEN Proposed listing `p1` with `mine` true, and a Worker that answers its `DELETE` with 409
- WHEN `p1`'s Withdraw is tapped
- THEN `Not withdrawn (409).` shows under `p1`, which is still listed, and no implementation state was asked for after the `DELETE`

#### Needs
- web.worker/snapshot-names-its-repo

### Requirement: push-in-the-browser <!-- base: bf36ad5a8eab6583 -->
The built app, opened in a browser, SHALL wire `web.client-live`'s Notify me switch and push handlers into the page and its service worker (`web.client-live/notify-switch`, `notify-toggle`, `push-renewal`, `push-shown`), the switch showing in the sidebar under the repo picker, and on phones beside the repo switcher. What it gives the switch's `Notify` SHALL be: a push manager only when the browser has `Notification` and `PushManager` and the app's service worker registered, the push manager of that worker once it is ready, and none otherwise, asking no key then; as its key, the `key` that `GET /api/push/key` answers, none when it answers 404 or no key, and a failure for any other error; as whether a repo holds an endpoint, whether `GET /api/repos/<o>/<r>/push?endpoint=<the endpoint, percent-encoded>`, carrying `fresh` when one is given, answers `"subscribed": true`; as the permission and its ask, the browser's `Notification` permission and its request; as online, whether the app isn't offline (`installable`); as its writes, the app's writes to the Worker, a 401 showing the sign-in page (`app-writes`); and as its storage, the browser's `localStorage`, a read that throws answering nothing stored and a write that throws failing. The service worker the app registers, `/app/sw.js` with scope `/app/`, SHALL show a push and handle a click on it through `web.client-live`'s push handlers, which it imports.

#### Scenario: no key
- GIVEN a 1280 px wide window, and a Worker answering 404 for the push key
- WHEN `#/o/a/graph` is opened and the key has been asked for
- THEN the sidebar shows no switch

#### Scenario: no Web Push
- GIVEN a 1280 px wide window and a Worker answering a push key, with a browser that has no `PushManager`; and, apart, a browser in which the app's service worker fails to register
- WHEN `#/o/a/graph` is opened in each, and the page has settled
- THEN the sidebar shows no switch, and no request went to `/api/push/key` or to the repo's push route

#### Scenario: a key and no subscription
- GIVEN a Worker answering a push key, and a browser with no subscription
- WHEN `#/o/a/graph` is opened at 1280 px, and at 390 px
- THEN the sidebar shows the switch, off and enabled, and no request went to the repo's push route; at 390 px the switch sits in the top bar beside the repo switcher

#### Scenario: turn on
- GIVEN a browser with notification permission granted, and a Worker answering the key `k` and `{"subscribed": false}` for `o/a`
- WHEN the switch is turned on, and then another view is chosen
- THEN the push manager was subscribed with `userVisibleOnly` and `applicationServerKey` `k`, one `POST /api/repos/o/a/push` carried that subscription, the switch reads on, and the redraw asked `GET /api/repos/o/a/push?endpoint=<the subscription's endpoint, percent-encoded>` and still reads on

#### Scenario: signed out on a turn-on
- GIVEN a browser with notification permission granted, a Worker answering the key `k`, and then 401 to `POST /api/repos/o/a/push`
- WHEN the switch for `o/a` is turned on
- THEN the sign-in page is shown

#### Scenario: storage that throws
- GIVEN a browser whose `localStorage` throws on every read and write, a browser subscription held from page load, and a Worker answering the key `k` and `{"subscribed": true}` for `o/a`
- WHEN `#/o/a/graph` is opened and the switch is turned off
- THEN the switch was drawn on, one `DELETE /api/repos/o/a/push` was sent, the switch reads off, and the subscription's `unsubscribe` was never called

#### Scenario: the service worker shows a push
- GIVEN the app opened at `/app/`, its service worker `/app/sw.js` active, and no app window known to it
- WHEN the worker receives a push `{"title": "o/a: waiting on you", "body": "Which region?", "url": "/app/#/o/a/backlog"}`, and then a click on its notification
- THEN a notification with that title and body, tagged `/app/#/o/a/backlog`, with the icon `/app/icon-192.png`, was shown, and a window was opened at `/app/#/o/a/backlog` on the worker's origin

#### Needs
- web.worker/push-subscriptions
- web.client-live/notify-switch
- web.client-live/notify-toggle
- web.client-live/push-renewal
- web.client-live/push-shown

## REMOVED Requirements

### Requirement: live-binding <!-- base: a60502c69faa0e8a -->

### Requirement: replica-binding <!-- base: 69721fd8187d4997 -->

### Requirement: no-cron-trigger <!-- base: 6ab19e2c36312003 -->
