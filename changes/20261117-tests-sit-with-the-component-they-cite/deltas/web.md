---
component: web
base: { design: "c73a09a862670d12" }
---

## MODIFIED Design
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

**Tests** (`commands-live-in-their-crates`, `components-own-their-tests`). The browser tests under `web/e2e/` run the built app, with this component's entry file, stylesheet, routes and service worker, so what they show is this component's to state: they cite only its requirements, the `-in-the-browser` ones for what a view or the live modules draw. A view's own behaviour, drawn with the Worker faked, is `web.views`' and `web.client-live`'s to prove in their files. The tests of `web/wrangler.toml` and `web/.npmrc` sit under `web/src/`, none in the Worker's crate. Until those requirements are built, `changes/20261117-tests-sit-with-the-component-they-cite/moved-tests.txt` names each test that cited another component from this component's files, or this component from another's, with where it goes and what it cites from then on.

## ADDED Requirements

### Requirement: cost-views-in-the-browser
The built app, opened in a browser with the Worker's API answered, SHALL show the Capabilities and Architecture views and their pages as `web.views` draws them (`web.views/capabilities-list`, `architecture-list`, `decide-buttons`, `architecture-decide-buttons`), under this component's stylesheet and routes: at 900 px and wider each list is a table and no card shows; a proof `held <c>/<n>` with `c` less than `n` is drawn in the theme's amber; Architecture is the sidebar's last view, after Capabilities; a row opens its page at `#/<owner>/<repo>/capabilities/<slug>` or `#/<owner>/<repo>/architecture/<slug>`, with the view's sidebar entry marked; and a decision posts once however often it is tapped while in flight, after which the report is asked for again with `fresh` and the page shows what the Worker then answers, never an older report whose request began before the decision.

#### Scenario: the capabilities table
- GIVEN a 1280 px wide window, and a report listing `b` then `a`, with `a` flagged
- WHEN `#/o/a/capabilities` is opened, "⚑ only" is turned on, and `a`'s row is chosen
- THEN the table lists `b` first and no card is visible, then only `a` is listed, and the page at `#/o/a/capabilities/a` shows `a`'s entry with Capabilities marked in the sidebar

#### Scenario: the architecture table
- GIVEN a 1280 px wide window, and a report listing `e` (flagged, one of its two enforcing rules current, with `#### Untested`) then `g` (holding only `#### Untested`)
- WHEN Architecture is chosen in the sidebar, "⚑ only" is turned on, and `e`'s row is chosen
- THEN Architecture is listed last, after Capabilities, and opens `#/o/a/architecture`; `e` comes first and reads `held 1/2 + untested` in the theme's amber, `g` reads `untested`; then only `e` is listed; and the page at `#/o/a/architecture/e` shows `e`'s entry

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
- THEN the first says the entry changed since it was shown, draws it as reworded, and the next tap posts the new version; the second shows the Worker's `command` with the buttons disabled; and the third shows the buttons disabled with `clm architecture decide e --value high|medium|low --decision keep|cut|redesign|accept`

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser

### Requirement: review-in-the-browser
The built app, opened in a browser with the Worker's API answered, SHALL show the PR list and a spec branch's review as `web.views` draws them (`web.views/spec-branch-review`, `full-document`, `signed-approvals`, `approve-button`, `comment-on-review`, `request-changes`), under this component's stylesheet and routes: a branch opens at `#/<owner>/<repo>/prs/<branch>` with its review loaded from the Worker's branch route, and no request goes to a `/pulls` route from any view; a file opened in full is scrolled so its first change is in the window, and `j` brings the next into it; on a phone the signed note, Request changes and its form sit in the Approve bar, wholly in the window, above the tab bar, and the page never scrolls sideways; and Approve, Comment and Request changes each post once however often they are tapped while in flight, after which the review is asked for again.

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
- GIVEN a snapshot whose component `a` is oversized, and an implementation state, not yet answered, in which `a` is blocked
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
The built app, opened in a browser, SHALL wire `web.client-live`'s Notify me switch and push handlers into the page and its service worker (`web.client-live/notify-switch`, `push-shown`): the switch sits in the sidebar under the repo picker once the Worker answers a push key, and is hidden when it answers 404; and the service worker the app registers, `/sw.js`, shows a push and handles a click on it through the handlers it imports.

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
- GIVEN the app opened at `/app/`, its service worker `/sw.js` active, and no app window known to it
- WHEN the worker receives a push `{"title": "o/a: waiting on you", "body": "Which region?", "url": "/app/#/o/a/backlog"}`, and then a click on its notification
- THEN a notification with that title and body, tagged `/app/#/o/a/backlog`, with the icon `/app/icon-192.png`, was shown, and a window was opened at `/app/#/o/a/backlog` on the worker's origin

#### Serves
- capability/follow-from-browser

## MODIFIED Requirements

### Requirement: invited-users <!-- base: cfd18219965d6dc7 -->
The app's build SHALL write the GitHub logins of this component's Contract's invited-users list, read from `specs/web/SPEC.md` at build time, to `_clm/invited.json` beside the built app, and `web/wrangler.toml` SHALL bind the built files to the Worker as `ASSETS` and send `/_clm/*` to its script, so the Worker completes sign-in only for those logins (`web.worker/invited-sign-in`) and the list is never served as a file.

#### Scenario: list from the spec
- GIVEN a build of the app
- WHEN a test reads `_clm/invited.json` in the built files
- THEN its logins are exactly those listed in `specs/web/SPEC.md`'s Contract

#### Scenario: the list reaches only the Worker
- GIVEN the repository
- WHEN a test reads `web/wrangler.toml`
- THEN its assets are bound as `ASSETS`, and its `run_worker_first` sends `/_clm/invited.json` to the script

#### Serves
- architecture/invite-only-web

### Requirement: site-served-directly <!-- base: 13328342baf11ef7 -->
`web/wrangler.toml` SHALL route to this Worker, on `clm-app.dev`, exactly the app's paths, `/app`, `/app/*`, `/api/*`, `/auth/*`, `/webhook`, `/_clm/*` and `/sw.js`, with no Custom Domain, so every other path reaches the site's Worker (`web.site/site-worker`); its `run_worker_first` SHALL send to the script exactly `/api/*`, `/auth/*`, `/webhook`, `/_clm/*` and `/app`, so its assets answer the app's files and `/sw.js` with no script run, and `not_found_handling` SHALL be `"none"`. Changing the routing moves the test fixtures that repeat it (`capabilities/web/harness.ts`'s `run_worker_first` and `not_found_handling`): this build makes that fix, to a capability test it serves, and names it in its pull request (`path-ownership`).

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

### Requirement: no-cron-trigger <!-- base: b75cdebfa1ce6fd7 -->
`web/wrangler.toml` SHALL hold `crons = []` under `[triggers]`, so that a deploy clears the Cron Trigger earlier deploys created and nothing calls the Worker on a schedule. The key SHALL stay, empty: `wrangler deploy` leaves the deployed triggers as they are when `crons` is absent.

#### Scenario: the trigger is cleared
- GIVEN the repository
- WHEN a test under `web/` reads `web/wrangler.toml`
- THEN `triggers.crons` is present and is an empty list

#### Serves
- capability/follow-from-browser

### Requirement: no-install-scripts <!-- base: 0d751f57f2fd19d7 -->
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

### Requirement: approve-by-branch <!-- base: 971e758ab6f3f54a -->
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
- web.worker.review/approve

### Requirement: notice-refresh <!-- base: 4f65ff2700d58dc5 -->
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
- web.client-live/live-updates
- web.worker/fresh-live-state
