---
component: web
base: { design: "c102386f7d5e06d5" }
---

## MODIFIED Design
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

## MODIFIED Requirements
### Requirement: mobile-layout <!-- base: b86f52c64d42ba41 -->
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

## ADDED Requirements
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
