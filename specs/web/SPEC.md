---
title: Web
status: active
owners: ["@clunny"]
depends_on: [web.worker, web.client-live, web.site, view, effort]
paths: ["web/*", "web/e2e/**", "web/public/**", "web/src/*", "web/test/**"]
successor: ""
size_budget: { spec_tokens: 16000, code_tokens: 105000 }
optional: true
serves: [capability/follow-from-browser, capability/see-project-state, capability/review-capability-cost]
---

## Contract
A hosted web app at `https://clm-app.dev/app/`, served by a Cloudflare Worker (`web.worker`) that claims the app's paths on `clm-app.dev`, beside CLM's public site at the root, which `web.site` builds and deploys as a Worker of its own (`public-site`, `site-worker-of-its-own`), where invited people signed in with GitHub see the CLM repos they can read: the spec graph, the spec branches proposed for review, implementation progress and the backlog. It approves a spec branch by appending an `approved` record to the signed-in person's web ledger (`per-writer-ledgers`), records that person's choice to be notified about a repo as a `subscribed` or `unsubscribed` record there (`push-subscriptions-in-ledger`), takes a proposal for the backlog as a `proposed` record, which `clm rebase` files as a backlog item on the next build (`web-proposals-as-ledger-records`), records a comment on a spec branch's diff or a request that it change as a `commented` or `changes_requested` record, the latter holding the branch's land until the author pushes a change or the requester approves (`web-review-feedback-as-ledger-records`), records a review decision on a capability or an architectural constraint as a `decided` record (`review-capability-cost`), and writes nothing else to the repository. Open pages show a change within seconds of its push (`web-live-channel`), and the installed app can notify people when something waits on them. What it caches is derived from git and GitHub and can be deleted at any time (`web-derived-cache`). It works on phones and installs as a PWA.

**Invited users** (`invite-only-web`). Only these GitHub logins may sign in. Anyone else gets a "not invited" page, and sees nothing of any repository. The public site is open to everyone (`web/site-served-directly`). Adding someone is a change to this list.

- `clunny`

## Design
The mockups in `assets/mockups/` are the reference for layout, colour, type and interaction: `assets/mockups/desktop.html` (sidebar, spec graph, PR document and graph tabs, implementation, backlog), `assets/mockups/mobile.html` (tab bar, PR review, graph with bottom sheet, offline implementation) and `assets/mockups/full-document.html` (a changed file in full). Their data is illustrative, and they show a light/system/dark theme switch the app doesn't have: the palette follows the system's setting. Where they and the requirements disagree, the requirements win.

Three parts, the Worker as the child `web.worker`, the public site as the child `web.site`, and the app here, with the app's live socket, Notify me switch and push handlers in the child `web.client-live`:
- `crates/clm-web` (`web.worker`): the Worker, in Rust with `workers-rs`, built for `wasm32-unknown-unknown` (`web-reuses-rust-in-wasm`). It serves the API, the GitHub sign-in, the webhook and the static assets this component builds. Its API is the contract between the two.
- `web/`: the browser app in TypeScript, whose `web/src/main.ts` imports `web.client-live`'s modules from `web/src/live/` and wires them to the page, built with Vite (`web-typescript-frontend`). Graph layout uses ELK (`elkjs`, layered, edges from dependent to dependency) and word-level diffs use `diff` (jsdiff). The framework is the builder's choice. `web/.npmrc` sets `ignore-scripts=true` and `web/package-lock.json` is committed, so no dependency's install script runs, whichever npm command installs it (`unattended-commands`). Unattended sessions add dependencies with `npm install --ignore-scripts` and run the app's checks with `npm ci`, `npm run`, `npm test` and `npx playwright test`. Playwright's Chromium is installed outside the session: once per machine by the human, and by CI's browser-test step. `web/wrangler.toml` and `web/package.json`'s `build:worker` script build and deploy them as one Worker with assets, so the deploy stays here.
- `web/site/` (`web.site`): the homepage, the guide and the blog, a VitePress package of its own with its own `package.json`, lockfile and `wrangler.toml`, which the site workflow builds and deploys as a Worker of its own (`site-worker-of-its-own`). `web`'s build leaves it alone: `web/package.json`'s `build` script builds only the app, with Vite's `base` `/app/`, into `web/dist/app/`, beside the invited list at `web/dist/_clm/invited.json` and the retiring service worker at `web/dist/sw.js` (`app-at-app-path`). The capability harness builds and serves the site for its own tests.

**Loading and refresh.** Everything the browser shows comes from the Worker, which alone holds the user's token. The app keeps the repo list, and for the repo on screen its snapshot and spec-branch list, in memory, the capabilities report once the Capabilities view has drawn it, and the architecture report once the Architecture view has drawn it, so a navigation within a repo draws from them at once while it asks again. A thin loading bar runs across the top of the page for navigations only (`loading-indicator`). The graph doesn't wait for the implementation state: when it arrives, the inspector adds work in flight and the cards take their queue-state rings.

While the page is visible (`document.visibilityState`), the app refreshes about every 60 seconds, matching the Worker's live-state cache (`live-refresh`). A refresh asks again for the endpoints the view on screen was drawn from: the snapshot and the spec-branch list for every repo view, the implementation state for the graph, Build and Backlog, the branch's review on a PR page, the capabilities report on Capabilities, the architecture report on Architecture, and the spec branches' reviews once the graph's inspector has loaded them. The repo list is asked for only on navigation. A refresh compares the parsed responses with those on screen, ignoring the implementation state's `now`. When they are equal it redraws nothing; Build's relative times are drawn against the browser's clock, so they stay current between redraws. Otherwise it redraws the view in place, keeping everything the user set in it: the graph's pan and zoom (as they were, even when the layout moves) and selected component, the page's scroll, tabs and switches (Document or Graph, In flight or Blockers, inline, after or before), the backlog filter, an open bottom sheet, and the full document's current change. A selected component the new state lacks is deselected. In the full document, the current change is kept while its text remains a change, else the nearest change by position is. No refresh runs while an approval, a Request changes, a comment, a proposal, a withdrawal or a decision on a capability or an architectural constraint is in progress, and a refresh begun before one ended draws nothing of what it read. The same refresh also runs at once when the repo's socket brings a notice (**Notices**, below). A failed refresh doesn't fall back to the copy kept for offline use, and the next one tries again.

**Views.**
- *Spec graph*: an SVG of components and `depends_on` edges, each component drawn inside its parent's box, the root's children inside the root. Each component card shows its status (active or unbuilt), its requirements as a bar segmented by coverage state (current green, behind amber, unbuilt sky) with `n/m` current of total and `✕n` obsolete markers when there are any, and its owners; once the implementation state has arrived, a component that isn't `converged` is ringed in its queue state's colour and names the state. A component the `spec` model marks `oversized` carries `⚠` in its card's label and an amber outline, unless a queue-state ring colours it, as `view`'s SVG draws it (`oversized-is-a-state`, `oversized-component`); a component whose `oversized` is absent or `false` carries neither. The legend has one entry per colour the graph uses, naming its meanings on the graph. A selected PR rings its touched components and highlights their edges. Selecting a component opens an inspector with its contract, dependencies, changes in open PRs, work in flight and backlog items.
- *Spec PRs*: a spec branch's review, which this component's requirements call a PR and its views label "PR" whether or not the host has a page for it (`spec-branch-review`). The list is the Worker's spec-branch list (`GET .../branches`): each `spec/*` branch by name, with its page's number and title, linking out to the page, when it has one. Each entry opens the branch's review, routed by the branch name as one encoded segment (`#/<owner>/<repo>/prs/spec%2Fx`, so `spec/x/graph` and `.../full/<path>` stay unambiguous), while requests to the Worker keep its slashes (`/branches/spec/x/review`), loaded from `GET .../branches/{branch}/review`, with Document and Graph tabs and Approve intent. Its header shows "needs review" (`gate/needs-review`), who approved it (each `approved` record's writer and time from the review's `approvals`, or "not approved"), "checked" or "not checked" from `checked`, and a link to the page when there is one (`spec-branch-review`). Approve posts to `POST .../branches/{branch}/approve` (`approve-by-branch`, `approve-button`); on success it loads the review again, so the approval shown is the ledger's, and a 409 is the Worker's answer when the branch's fingerprint isn't the one sent. In a signed repository, the review's `signed` (`signed-approvals`, `web-approve-off-when-signed`), the note beside Approve, in the Approve bar (pinned above the tab bar on phones), shows the command `clm approve <branch>` with a copy button. The header's approvals are drawn neutral rather than green, each marked "signed" (sky: `clm land` will check it) or "unsigned, doesn't count" (rose) from its `signed`. A signed approval is one `clm land` checks, so the app never calls it counted. A 422 `signed` means the repository became signed after the review was drawn, so the note appears once the review is loaded again. The note shows offline too. **Request changes** (`request-changes`, `web-review-feedback-as-ledger-records`) sits beside Approve, in the Approve bar on phones: it opens a one-field form for the request's text and posts to `POST .../branches/{branch}/request-changes`. It works in a signed repository, where Approve doesn't, since a request withholds a land rather than granting one. The header draws each held request in rose, naming the person part of the writer as the activity feed names actors; a request that isn't held is drawn with the comments under the intent, dimmed. Request changes isn't disabled by an approval or by an earlier request: a second request by the same person is another text. A 403, a reader without write access, says so and leaves the button enabled. **Comments** (`comment-on-review`): the Document tab's intent and each requirement card carry a Comment control that opens a one-field form and posts to `POST .../branches/{branch}/comments`. A 422 shows the Worker's `error` under the form and keeps it. The Document tab shows the intent, then per component each changed element as a card with a couple of dimmed lines of context and word marks, then an impact summary, decisions and backlog edits. **Open in full** shows a changed file whole, as it reads at the head, with unchanged text dimmed, long unchanged runs folded, every change marked and numbered, and an outline marking where they are. It opens scrolled to the first change. ↑/↓ and `j`/`k` step through the changes, and a switch shows changes inline, after or before.
- *Implementation* (Build): counts (pending components by state: claimed, ready and blocked; live changes; requirements still to make current), then a switch between **In flight** and **Blockers**. In flight shows the `flight` view (`implementation-view`); a branch's pull request shows "checked" or "not checked" from `pages`, and nothing while the response's `pages` doesn't yet say. Blockers shows the `blockers` view (`blockers-view`) with the chain to each held-up component. An activity feed (`activity-feed`) names the branch of a land or an approval.
- *Backlog*: a **Waiting on you** row first (`backlog-view`, `requirement-blocks`), then columns by kind (bug, question, deferred, idea). An item something holds (`propose-claims`, `backlog-taken`) carries a mark from the implementation state's `taken`; a bare claim is a live `propose/<slug>` claim. The backlog is drawn from the snapshot first and takes the marks when the implementation state arrives, as the graph takes its rings. The sidebar and tab bar badge the number of items waiting. **New proposal** (`propose-from-backlog`, `web-proposals-as-ledger-records`): a button above the columns opens a form with a title, an intent (plain text or Markdown) and a picker of the graph's components, posting to `POST .../proposals`; a 422 shows which field the Worker's `error` names and keeps the form. The picker takes at most 10 components, the Worker's limit. A row **Proposed** sits between Waiting on you and the columns; **Withdraw** sends `DELETE .../proposals/{id}`.
- *Capabilities* (`review-capability-cost`): every capability's cost and its review, drawn from the Worker's `GET .../capabilities` (`web.worker/capabilities`), which has no history, so commits, questions answered, blocking and sizes are absent and the views say to run `clm capabilities <slug>` for them. The list (`capabilities-list`) keeps the report's order, by cost, highest first; narrower than 900 px each capability is a card with a bar for its share of the costliest. A capability opens its page (`capability-page`) at `#/<owner>/<repo>/capabilities/<slug>`, with the sections in `clm capabilities <slug>`'s order, then the decision buttons (`decide-buttons`). A decision is posted to `POST .../capabilities/<slug>/decide` (`web.worker/decide`) with the capability's version as shown (a page drawn again meanwhile keeps the buttons off), and the report is loaded again from the Worker with `?fresh=` and a random delivery, so the Worker reads the ledgers anew rather than its minute-old copy. On another 422 (the capability left the tip, or its specs don't load) the page shows the tip as it is now. A 401 shows the sign-in page. When the report's `signed` is true, a note says this repository counts only records signed with the reviewer's SSH key and gives `clm capabilities decide <slug> --value high|medium|low --decision keep|cut|redesign|accept`.
- *Architecture* (`review-capability-cost`, `docs/decisions/architecture-review.md`): every architectural constraint, whether it is held and what it costs, with its review, drawn from the Worker's `GET .../architecture` (`web.worker/architecture`). It is a view of its own, after Capabilities in the sidebar and the tab bar, not a tab inside Capabilities. Its cost is a second split, apart from the capabilities', and the view says so. The list (`architecture-list`) keeps the report's order, by cost, highest first; narrower than 900 px each entry is a card. An entry opens its page (`architecture-page`) at `#/<owner>/<repo>/architecture/<slug>`, then the decision buttons (`architecture-decide-buttons`), which post to `POST .../architecture/<slug>/decide` (`web.worker/architecture-decide`) and load the report again as a capability's buttons do. While the report's `indexing` is true, each enforcing rule's state is drawn as unknown, and the view says coverage is still indexing.

**Look.** Colour tokens on `:root` with a dark and a light palette, one meaning per colour, the same as `view`'s charts (`colour-meanings`): green for active, converged, current, proven or complete; sky for unbuilt or ready; amber for claimed, behind, oversized or a modified element; rose for blocked, obsolete, a removed element or needs review; indigo for the accent and headings. The palette follows the system's light or dark setting. Inter for text and JetBrains Mono for IDs. The graph's canvas, on the spec graph and on a PR's Graph tab, runs from its top to the bottom of the window (on phones, to the top of the tab bar, or of the Approve bar on a PR, as drawn with whatever it holds), at least 320 px tall, rather than the mockups' fixed height; on desktop the inspector beside it scrolls within that height (`graph-fills-height`). At 900 px and wider: a sidebar with the repo switcher and views. Narrower: a bottom tab bar (Graph, PRs, Build, Backlog, Capabilities, Arch.), one column, Approve pinned above the tab bar, the graph pans and pinch-zooms, and the inspector is a bottom sheet.

**Notices** (`web-live-channel`). While the page is visible and online, and no sign-in page is on screen, the app gives `web.client-live`'s socket the live route of the repo on screen, `/api/repos/<o>/<r>/live`, and otherwise none, so the socket closes when the page is hidden, the repo changes or the user signs out. A signed-out session shows itself through the next refresh's 401. Each notice the socket hands over, `{"delivery"}`, starts a refresh at once (`notice-refresh`), whose requests add `?fresh=<delivery>`. A notice that arrives during a refresh starts another when it ends, and notices that arrive meanwhile share it. A notice that arrives while a write is in progress waits until it ends. The 60-second tick runs regardless, so a page without a working socket stays current. The copy kept for offline use is stored under the request's path without `fresh`, so notices add no entries and an offline load finds the newest answer. The sidebar, under the repo picker (on phones, beside the repo switcher), holds `web.client-live`'s Notify me switch for the repo on screen.

**PWA.** The installed app is the app only (`public-site`): a web app manifest with icons whose `id`, `start_url` and `scope` are `/app/`, and a service worker registered with scope `/app/`, served from under it, that never answers or caches a request outside `/app/`. Links from the app to the site leave the scope, so the installed app opens them in a browser tab over it. The service worker precaches the app shell and keeps the last API responses per repo in Cache Storage. Offline, the app shows them with a banner (`installable`), and Approve, Request changes, Comment, New proposal, Withdraw and the decision buttons of a capability and of an architectural constraint are disabled.

**Size** (`slim-components`). This component's code budget is 105,000 tokens, above the default, because this component is one browser app with one shell: `web/src/main.ts` routes every view, holds what they were drawn from and runs the one refresh they share, `web/src/styles.css` styles them all, and the end-to-end tests drive them through that shell. Splitting a view out would leave its child reaching into the shell's state and styles, a cycle `depends_on` can't hold, so the parts that stand alone, the Worker and the live socket, Notify me switch and push handlers, are the children, and the rest stays here. The spec's budget is 16,000 for the same reason: every view's rules are rules of the one app.

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

### Requirement: full-document
The PR view SHALL open any changed file whole, as it reads at the PR's head, with every change marked and numbered, scrolled to its first change, with controls to step to the next and previous change.

#### Scenario: open at first change
- GIVEN a PR whose component spec changes a Design paragraph and a later requirement
- WHEN the reviewer opens that component in full
- THEN the Design change is in view and marked as change 1 of 2, and "next" brings the requirement into view

### Requirement: graph-view
The graph view SHALL draw every component and `depends_on` edge, each component inside its parent, with each component's status, owners and requirements as the `view` component's `spec` model gives them: a bar segmented by coverage state, current green, behind amber and unbuilt sky, with `<current>/<total>`, and the obsolete count when there is one. Once the implementation state has arrived, it SHALL ring each component that isn't `converged` in the colour of the queue state the `flight` model gives it and name that state on the card. It SHALL mark the components a selected PR touches.

#### Scenario: touched node
- GIVEN a repo with components `a` and `b`, where `b` depends on `a`, and a PR touching `b`
- WHEN the graph is shown with that PR selected
- THEN there is an edge from `b` to `a`, and only `b` is marked as touched

#### Scenario: nested node with owners
- GIVEN a repo with components `check` and `check.drift`, `check.drift` owned by `@a`
- WHEN the graph is shown
- THEN `check.drift`'s card is inside `check`'s box and shows `@a`

#### Scenario: a node behind its spec
- GIVEN a repo whose component `a` has 3 requirements, 2 current and 1 cited only at an older version
- WHEN the graph is shown
- THEN `a`'s card shows `2/3` and its bar has a green segment and an amber segment and no sky segment

#### Scenario: queue states
- GIVEN a repo whose implementation state has component `a` `claimed` and component `b` `blocked`, and component `c` with nothing pending
- WHEN the graph is shown and the implementation state has arrived
- THEN `a`'s card carries the `claimed` ring, drawn in amber, and names `claimed`; `b`'s carries the `blocked` ring, drawn in rose, and names `blocked`; and `c`'s has no ring and names no queue state

#### Needs
- check.coverage/state-names
- view/spec-view

### Requirement: implementation-view
The implementation view SHALL show the `view` component's `flight` model: the pending components in its order, each with the state `queue_from_refs` gives it, for a claimed component its last push and when the claim lapses, for a component whose branch has a page that page and whether the ledger holds a `checked` record for its head's code key, for a blocked component the backlog items marked as waiting on a human and the requirements of other components it waits on (`requirement-needs`), and its pending requirements by state, showing no `waiting` state and reading no `waiting_on` field (`needs-alone-order`), and taking a requirement's blockers from its `blocked_by` and `waits_on`, never from `needs_pending`; then the live changes in the model's groups and order, each with its requirements current of those it adds or modifies, and `complete` where it applies.

#### Scenario: claimed node
- GIVEN a pending component claimed by a branch last pushed an hour ago
- WHEN the view is shown
- THEN the component shows as claimed, lapsing in 3 hours

#### Scenario: change progress
- GIVEN an `applied` change whose deltas add two requirements, one current
- WHEN the view is shown
- THEN the change shows 1 of 2 requirements current and isn't marked complete

#### Scenario: blocked by a need
- GIVEN a pending component `a` whose only pending requirement needs `b/y`, uncited
- WHEN the view is shown
- THEN `a` shows as blocked by `b/y`

#### Scenario: no waiting field
- GIVEN a `flight` model whose pending component `a` is `ready` and has no `waiting_on` field
- WHEN the view is shown
- THEN `a` shows as ready, with no line naming what it waits on

#### Scenario: checked page
- GIVEN a claimed component `a` whose branch `impl/a` has page #7, and the implementation state's `pages` says its head was checked
- WHEN the view is shown
- THEN `a` links to page #7 and shows it as checked

#### Scenario: waits on
- GIVEN a pending component `a` whose only pending requirement `a/x` has `waits_on` `["b/y"]`, an empty `blocked_by` and no `needs_pending` field
- WHEN the view is shown
- THEN `a/x` shows as blocked by `b/y`, and no source file under `web/src` or `web/test` names `needs_pending`

#### Needs
- orchestrator/queue-from-refs
- replica/implementation-ledger

### Requirement: activity-feed
The implementation view SHALL list the repo's recent pushes, claims (creations of `impl/*` branches), lands and approvals, newest first: pushes and claims from GitHub's repo events, lands and approvals from the ledger's `landed` and `approved` records.

#### Scenario: approval shows
- GIVEN a ledger holding an `approved` record for the branch `spec/x` at 10:00 and repo events holding a push at 10:05
- WHEN the view is shown
- THEN the feed lists the push, then the approval of `spec/x`

#### Needs
- replica/implementation-ledger

### Requirement: backlog-view
The backlog view SHALL show every backlog item grouped by kind, with its title, components and slug, and SHALL filter by component. It SHALL list `question` and `human-task` items first, under "Waiting on you", each with the requirements it blocks.

#### Scenario: filter
- GIVEN items on components `a` and `b`
- WHEN the view is filtered to `a`
- THEN only items on `a` are shown

#### Scenario: waiting on you
- GIVEN a `human-task` item `setup` whose `blocks` names `c/a`, and an `idea` item
- WHEN the view is shown
- THEN `setup` is listed under "Waiting on you" naming `c/a`, before the `idea` item

#### Needs
- model/parse-backlog

### Requirement: mobile-layout
Below 900 px wide, the app SHALL replace the sidebar with a bottom tab bar, show one column with no horizontal scrolling, and keep Approve visible on a PR.

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
- THEN the tab bar holds six tabs, the last two Capabilities and Arch., the list shows one card per entry, neither page scrolls horizontally, and the page's decision buttons can be scrolled into view and tapped

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

### Requirement: blockers-view
The implementation view SHALL have a Blockers switch that shows the `view` component's `blockers` model in its order: each root blocker with its weight and the components it holds up, those that wait on the human marked as waiting on you, each component linking to its entry in the In flight list.

#### Scenario: blocker weight
- GIVEN components `b` and `c`, each with one pending requirement, which needs `a/x`, uncited on claimed component `a`, and component `d` whose only pending requirement is blocked by `backlog/q` of kind `question`
- WHEN the Blockers switch is shown
- THEN `a` is listed with 2 components held up, and `backlog/q` with 1, marked as waiting on you and listed first

#### Scenario: nothing blocked
- GIVEN a queue with no `blocked` components
- WHEN the Blockers switch is shown
- THEN it says nothing is blocked

#### Serves
- capability/see-project-state

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

### Requirement: queue-states
The implementation view SHALL know exactly the queue states `ready`, `claimed` and `blocked` (`builds-in-subagents`, `needs-alone-order`), and SHALL NOT read a component's `paths` from the `flight` model.

#### Scenario: no interactive or waiting state
- GIVEN the web app's source
- WHEN the tests run
- THEN the implementation view's state order is `claimed`, `ready`, `blocked`, neither its state labels nor the graph's state colours hold `waiting`, and no source file under `web/src` names an `interactive` state, `waiting_on` or `Needs an interactive session`

#### Serves
- capability/see-project-state
- architecture/needs-alone-order

### Requirement: review-intent-first
The PR view SHALL show the PR's `## Intent` section first, above the per-node changes, next to the approve button.

#### Scenario: intent first
- GIVEN a spec PR whose `change.md` has an `## Intent` section
- WHEN the reviewer opens the PR
- THEN the view shows that section first, above the per-node changes, next to the approve button

#### Serves
- capability/approve-intent
- capability/follow-from-browser
- architecture/humans-review-intent

### Requirement: spec-branch-review
The PR list SHALL list the `spec/*` branches the Worker's branch list gives, each by name with its page's number and title when it has a page, and SHALL open each branch's review, routed by branch name and loaded from the Worker's branch review. The review's header SHALL show "needs review" when the review's top-level `review` is true, whether or not it has a `change`, and nothing for it otherwise, the writer and time of each approval the review lists, or "not approved" when it lists none, "checked" or "not checked", and a link to the page when there is one, and SHALL show no labels. The PR list SHALL list nothing but the spec branches: no lands to review. The app SHALL request no `/pulls` route of the Worker.

#### Serves
- capability/follow-from-browser
- capability/approve-intent

#### Needs
- web.worker.review/spec-branches
- web.worker.review/pr-review

#### Scenario: branch without a page
- GIVEN the branch list holds `spec/a` with page #7 "Add a" and `spec/b` with no page
- WHEN the PR list is shown and `spec/b` is opened
- THEN both are listed, `spec/a` with #7 and "Add a", and `spec/b`'s review is loaded from `/branches/spec/b/review` and shows no page link

#### Scenario: approved by
- GIVEN `spec/x`'s review lists an approval by `clunny/web` and says `checked` false
- WHEN its review is shown
- THEN the header names `clunny/web` with the approval's time and says "not checked", and no request went to a `/pulls` route

#### Scenario: needs review without a change
- GIVEN `spec/arch`'s review with `review` true and no `change`, and `spec/plain`'s with `review` false and a `change`
- WHEN each review is shown
- THEN `spec/arch`'s header shows "needs review" and `spec/plain`'s doesn't

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

### Requirement: approve-button
Approve intent SHALL send one approve request per tap and none while one is in flight, and once the Worker accepts it SHALL read Approved and stay disabled. It SHALL be disabled offline, for a review without a fingerprint, and for a review that already shows an approval, as `approve-by-branch` says.

#### Scenario: one request, then approved
- GIVEN a spec branch's review, online, with a fingerprint and no approval
- WHEN Approve is tapped twice in quick succession and the Worker accepts the first request
- THEN exactly one approve request was sent, and the button reads Approved and is disabled

#### Scenario: disabled
- GIVEN a review with no fingerprint, a review that already shows an approval, and a review with a fingerprint shown offline
- WHEN each is shown
- THEN Approve is disabled in all three

#### Serves
- capability/follow-from-browser
- capability/approve-intent
- architecture/approve-pins-the-review

### Requirement: pr-document
The Document tab SHALL show, per touched component, every changed requirement, Contract, Design, frontmatter and asset the review gives, each with whether it was added, modified or removed, its text with word marks between before and after and a couple of dimmed lines of context from its file, and a link to its file in full; an HTML asset previewed in a sandboxed frame with no permissions; then each decision the review gives with what it says, and the backlog edits.

#### Scenario: modified requirement, asset and decision
- GIVEN a review that modifies requirement `n/r` by adding the word `quickly`, adds asset `assets/m.html` to `n` and adds decision `d`
- WHEN the Document tab is shown
- THEN `n/r` is marked Modified with `quickly` marked as inserted and its old text shown, `m.html` is marked Added and previewed in a frame whose `sandbox` attribute is empty, and `d` is listed as added with its text

#### Scenario: links to the files
- GIVEN a review whose changes touch the root component `clm`, component `n` and component `m.n`
- WHEN the Document tab is shown
- THEN their links open `specs/SPEC.md`, `specs/n/SPEC.md` and `specs/m/n/SPEC.md` in full

#### Serves
- capability/follow-from-browser
- capability/approve-intent
- architecture/humans-review-intent

### Requirement: backlog-taken
The backlog view SHALL mark each item the implementation state's `taken` holds: `resolved by <id>` when a change on main names it, else `proposed in <id>` when proposed spec work names it, else `claimed` when a bare claim holds it; it SHALL mark no other item.

#### Scenario: marks
- GIVEN items `a`, `b`, `c` and `d`, and a `taken` whose changes are `x` on main naming `a` and `y` proposed naming `b`, and whose claims hold `c`
- WHEN the backlog view is shown and the implementation state has arrived
- THEN `a` reads `resolved by x`, `b` reads `proposed in y`, `c` reads `claimed`, and `d` has no mark

#### Scenario: before the implementation state
- GIVEN the snapshot is answered and the implementation state isn't yet
- WHEN the backlog view is opened
- THEN the backlog is drawn, unmarked

#### Serves
- capability/see-project-state
- capability/keep-a-backlog

#### Needs
- replica/taken-items

### Requirement: inspector-pending
Once the implementation state has arrived with a complete marker index, the component inspector's work in flight SHALL show the selected component's queue state and pending count as the `flight` model gives them, and SHALL list each requirement in that component's `pending`, with its ID and state, and for each `obsolete` one the `<file>:<line>` of every marker that still cites it. A component the `flight` model doesn't list, or lists with nothing pending, SHALL show no requirement. While the response's `coverage` is indexing, the work in flight SHALL say that coverage is indexing and list no requirement, since every line would read `unbuilt`.

#### Scenario: obsolete marker of a removed node
- GIVEN an implementation state with complete coverage whose `flight` lists component `a` as `ready` with `a/x` behind and `gone/y` obsolete, cited by `src/a.rs:12` and `tests/a.rs:3`
- WHEN component `a` is selected
- THEN its work in flight shows `ready` and 2 pending, and lists `a/x` as behind and `gone/y` as obsolete with `src/a.rs:12` and `tests/a.rs:3`

#### Scenario: nothing pending
- GIVEN an implementation state with complete coverage whose `flight` doesn't list component `b`, and lists component `c` as `claimed` with an empty `pending`
- WHEN component `b`, then component `c`, is selected
- THEN neither work in flight lists a requirement, and `b`'s says nothing is pending

#### Scenario: indexing
- GIVEN an implementation state whose `coverage` is indexing and whose `flight` lists component `a` with `a/x` unbuilt
- WHEN component `a` is selected
- THEN its work in flight says coverage is indexing and lists no requirement

#### Serves
- capability/see-project-state
- architecture/component-reconciliation

#### Needs
- check.coverage/state-names
- view/flight-view

### Requirement: signed-approvals
When a spec branch's review says `signed`, the PR view SHALL draw Approve disabled, never reading "Approved", with a note beside it that the repository counts only approvals signed with the reviewer's SSH key and the command `clm approve <branch>` for that branch; SHALL mark each approval in the header "signed" or "unsigned, doesn't count" from its `signed`; and SHALL say "no signed approval" when it lists approvals and none is signed. An approve answered 422 with `error` `signed` SHALL load the review again from the Worker. When the review says `signed` false or doesn't say, the view SHALL show neither the note nor the marks.

#### Serves
- capability/follow-from-browser
- architecture/web-approve-off-when-signed
- architecture/signed-ledger-records

#### Needs
- web.worker.review/signed-repository

#### Scenario: signed repository
- GIVEN `spec/x`'s review with `signed` true, a fingerprint, and one approval by `clunny/web` with `signed` false
- WHEN the PR view is drawn online
- THEN Approve is disabled and doesn't read "Approved", the note shows `clm approve spec/x`, the header marks `clunny/web` "unsigned, doesn't count" and says "no signed approval", and tapping Approve sends no request

#### Scenario: signed approval
- GIVEN `spec/x`'s review with `signed` true and one approval by `dana/9f9f9f9f` with `signed` true
- WHEN the PR view is drawn
- THEN the header marks `dana/9f9f9f9f` "signed" and doesn't say "no signed approval"

#### Scenario: became signed
- GIVEN `spec/x`'s review drawn with `signed` false and fingerprint `f1`, and a Worker that answers the approve with 422 and `error` `signed`, and then the review with `signed` true
- WHEN the reviewer taps Approve
- THEN the review is requested again and the note shows `clm approve spec/x`

#### Scenario: unsigned repository
- GIVEN `spec/x`'s review with `signed` false, a fingerprint and no approvals, and another review with no `signed` field
- WHEN each PR view is drawn online
- THEN Approve is enabled and no note or approval mark shows

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

### Requirement: propose-from-backlog
The Backlog view SHALL offer New proposal: a form with a title, an intent and a picker of the graph's components that posts an object with `/title`, `/body` and `/nodes` to the Worker's proposals route once per tap and none while one is in flight, on success closes and asks for the implementation state again, on a 409 says the title is taken and keeps the form, and on a 422 shows the answer's reason. It SHALL list the implementation state's `proposals` under Proposed, between Waiting on you and the columns, each with its title, components, the person part of its proposer and its time and a note that the next build files it, with Withdraw on those whose `mine` is true, which sends one `DELETE` for the proposal's `id` and then asks for the implementation state again. Both SHALL be disabled offline.

#### Scenario: proposed
- GIVEN the Backlog of `o/a` online, whose graph holds components `a` and `b`, and a Worker that answers the proposal with 200 and then lists it in `proposals` by `alice/web`
- WHEN the user opens New proposal, enters the title "Export CSV", the intent "Let me export." and picks `a`, and submits twice in quick succession
- THEN exactly one request posted `/title` `"Export CSV"`, `/body` `"Let me export."` and `/nodes` `["a"]`, the form closed, and Proposed lists "Export CSV" with `a`, `alice` and the note

#### Scenario: title taken
- GIVEN a Worker that answers the proposal with 409
- WHEN the user submits
- THEN the form stays open saying the title is taken

#### Scenario: withdraw
- GIVEN Proposed listing `p1` by `alice/web` with `mine` true and `p2` by `bob/web` with `mine` false
- WHEN the view is shown and `p1`'s Withdraw is tapped
- THEN `p2` shows no Withdraw, one `DELETE` went to `/api/repos/o/a/proposals/p1`, and the implementation state was requested again

#### Serves
- capability/keep-a-backlog
- capability/follow-from-browser
- architecture/per-writer-ledgers

#### Needs
- web.worker/proposals

### Requirement: comment-on-review
The PR view's Document tab SHALL carry a Comment control on the intent and on each requirement card that posts `{"fingerprint": <the review's fingerprint>, "target": <null for the intent, the requirement's ID for a card>, "text"}` to the Worker's comments route for the branch, once per tap and none while one is in flight, and loads the review again on success. It SHALL draw each entry of the review's `comments` under its target, the intent for a null target or a requirement the review shows no card for, with the person part of its writer and its time, marked "on an earlier version" when `current` is false, and each entry of `change_requests` whose `held` is false under the intent the same way, marked "since approved" when `current` is true. A review without the two fields SHALL be read as having none. Comment SHALL be disabled offline and when the review has no fingerprint.

#### Scenario: comment on a requirement
- GIVEN `spec/x`'s review with fingerprint `f1`, which modifies `n/r`, and a Worker that answers the comment with 200 and then the review with a comment on `n/r` by `alice/web` at `f1`
- WHEN the reviewer taps Comment on `n/r`'s card, enters "Name the rule" and submits
- THEN one request posted `{"fingerprint": "f1", "target": "n/r", "text": "Name the rule"}` to `/branches/spec/x/comments`, and `n/r`'s card shows the comment by `alice`

#### Scenario: earlier version
- GIVEN a review with fingerprint `f2` whose `comments` holds one at `f1` with `current` false and target `null`, and whose `change_requests` holds one at `f2` with `current` true and `held` false
- WHEN the Document tab is shown
- THEN the comment is drawn under the intent, marked "on an earlier version", and the request under it, marked "since approved"

#### Serves
- capability/approve-intent
- capability/follow-from-browser
- architecture/humans-review-intent
- architecture/approve-pins-the-review

#### Needs
- web.worker.review/review-feedback

### Requirement: request-changes
The PR view SHALL offer Request changes beside Approve, in the Approve bar on phones, which takes a text and posts `{"fingerprint": <the review's fingerprint>, "text"}` to the Worker's request-changes route for the branch, once per tap and none while one is in flight; on success it SHALL load the review again, and on a 409 say the branch changed since the review was shown and load the review again, as Approve does. The header SHALL show each entry of `change_requests` whose `held` is true as changes requested by the person part of its writer, with its time and text, and say the branch won't land until the author pushes a change or the requester approves. Request changes SHALL be disabled offline and when the review has no fingerprint, and enabled in a signed repository, after an approval and after an earlier request.

#### Scenario: requested
- GIVEN `spec/x`'s review with fingerprint `f1` and `signed` true, online, and a Worker that answers the request with 200 and then the review with a held request by `alice/web`
- WHEN the reviewer taps Request changes, enters "Split it" and submits twice in quick succession
- THEN exactly one request posted `{"fingerprint": "f1", "text": "Split it"}` to `/branches/spec/x/request-changes`, and the header says changes requested by `alice` with "Split it"

#### Scenario: branch moved
- GIVEN the review drawn with fingerprint `f1`, and a Worker that answers the request with 409 and the review with fingerprint `f2`
- WHEN the reviewer submits a request
- THEN the status says the branch changed since the review was shown, and the review on screen is the one with fingerprint `f2`

#### Scenario: disabled
- GIVEN a review with no fingerprint, and a review with a fingerprint shown offline
- WHEN each is shown
- THEN Request changes is disabled in both

#### Serves
- capability/approve-intent
- capability/follow-from-browser
- architecture/humans-review-intent
- architecture/approve-pins-the-review

#### Needs
- web.worker.review/review-feedback

### Requirement: capabilities-list
The Capabilities view SHALL list every capability of the Worker's report in its order, marking each with an unsettled flag ⚑, with a "⚑ only" filter that shows only those. At 900 px and wider it SHALL be a table of the capability's slug and statement, its cost (`≈$`, work tokens and reviews), its unsettled workarounds, "Live without?" from its `overlap`, and its latest decision; narrower, one card per capability with the same facts. Choosing a capability SHALL open its page.

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser

#### Needs
- web.worker/capabilities

#### Scenario: ranked and flagged
- GIVEN a report listing `b` ($9) then `a` ($2), with `a` flagged
- WHEN the view is shown
- THEN `b` comes before `a`, only `a` is marked ⚑, and with "⚑ only" on only `a` is listed

### Requirement: capability-page
A capability's page SHALL show, in this order: its statement, why we need it, what it costs, its workarounds, each a `<details>` whose summary is its title and whose body is its text, can we live without it from its `overlap`, its rules collapsed, and its decisions, with a line saying to run `clm capabilities <slug>` for its commits, questions answered, blocking and size, which the Worker's report doesn't hold.

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser

#### Needs
- web.worker/capabilities

#### Scenario: sections
- GIVEN capability `x` with an unsettled flag titled "Judge ignores pipes" whose text is "Builds pipe output."
- WHEN its page is shown
- THEN the sections come in that order, the workaround's summary reads "Judge ignores pipes" and its body "Builds pipe output.", and the page names `clm capabilities x`

### Requirement: decide-buttons
A capability's page SHALL offer a value (high, medium or low) and Keep, Cut, Redesign and Accept as is, in a 2×2 grid below 900 px. A tap SHALL post `{"version", "value", "decision"}` to the Worker's decide route with the capability's version as shown, one request per tap and none while one is in flight, and then load the report again from the Worker, never the copy kept for offline use: on 200 the page SHALL show the new decision, and on 409 say the capability changed since it was shown. The buttons SHALL be disabled offline, before a value is chosen and when the report's `signed` is true; when it is, a note SHALL give `clm capabilities decide <slug> --value high|medium|low --decision keep|cut|redesign|accept`, as it SHALL after a 422 whose `error` is `signed`, giving the Worker's `command`.

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser
- architecture/humans-review-intent

#### Needs
- web.worker/decide

#### Scenario: one request per tap
- GIVEN capability `x` at version `v`, online, with value low chosen
- WHEN Cut is tapped twice in quick succession and the Worker accepts the first
- THEN exactly one request posted `{"version": "v", "value": "low", "decision": "cut"}` to `/capabilities/x/decide`, the report was asked for again, and the page shows the decision

#### Scenario: moved
- GIVEN the page drawn at version `v`, and a Worker that answers the decision with 409 and the report with `x` at version `w`
- WHEN a decision is tapped
- THEN the page says the capability changed since it was shown, and the page on screen is `x` at `w`

#### Scenario: disabled
- GIVEN a page offline, a page with no value chosen, and a page whose report has `signed` true
- WHEN each is shown
- THEN the decision buttons are disabled in all three, and the signed one's note gives `clm capabilities decide x --value high|medium|low --decision keep|cut|redesign|accept`

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

### Requirement: architecture-list
The Architecture view SHALL be a view of its own, listed after Capabilities in the sidebar as Architecture and in the tab bar as Arch. It SHALL list every entry of the Worker's architecture report in its order, marking each flagged one ⚑, with a "⚑ only" filter that shows only those, and a note that an entry's cost is its enforcing rules' share, counted apart from the capabilities'. At 900 px and wider it SHALL be a table of the entry's slug and statement, its proof (`held <c>/<n>`, amber when `c` is less than `n`, ` + untested` after it when it also holds `#### Untested`, or `untested` alone), its cost (`≈$`, work tokens and reviews), its unsettled workarounds, "Live without?" from its `overlap`, and its latest decision; narrower, one card per entry with the same facts. While the report's `indexing` is true, the proof SHALL read `? / <n>` and the note SHALL say coverage is still indexing. Choosing an entry SHALL open its page.

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser

#### Needs
- web.worker/architecture

#### Scenario: ranked, held and flagged
- GIVEN a report listing `e` ($5, flagged, enforced by two rules of which one is current) then `g` ($0, holding only `#### Untested`)
- WHEN the view is shown
- THEN `e` comes before `g`, `e` is marked ⚑ and shows `held 1/2` in amber, `g` shows `untested`, and with "⚑ only" on only `e` is listed

#### Scenario: its own view
- GIVEN a repo on screen at 1280 px
- WHEN the sidebar is shown
- THEN Architecture is listed after Capabilities, and choosing it opens `#/<owner>/<repo>/architecture`

#### Scenario: indexing
- GIVEN a report whose `indexing` is true
- WHEN the view is shown
- THEN `e`'s proof reads `? / 2` and the note says coverage is still indexing

### Requirement: architecture-page
An architectural constraint's page SHALL show, in this order: its statement; whether it is held, each enforcing rule with its state (current in green, pending in amber, unknown while the report is indexing), then its `#### Untested` text when it has one; what it costs, saying it is counted apart from the capabilities; its workarounds, each a `<details>` as on a capability's page; can we live without it from its `overlap`; and its decisions; then the decision buttons (`architecture-decide-buttons`).

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser

#### Needs
- web.worker/architecture

#### Scenario: sections
- GIVEN entry `e` enforced by `a/x`, current, and `a/y`, pending, with the untested text "Partly judgement." and a flag titled "Gate misfires"
- WHEN its page is shown
- THEN the sections come in that order, `a/x` is drawn current and `a/y` pending, "Partly judgement." follows them, and the workaround's summary reads "Gate misfires"

### Requirement: architecture-decide-buttons
An architectural constraint's page SHALL offer the value and decision buttons `decide-buttons` describes, laid out as there, posting `{"version", "value", "decision"}` to the Worker's architecture decide route with the entry's version as shown, one request per tap and none while one is in flight, then loading the architecture report again from the Worker, never the copy kept for offline use: on 200 the page SHALL show the new decision, and on 409 say the entry changed since it was shown. The buttons SHALL be disabled offline, before a value is chosen and when the report's `signed` is true; then, and after a 422 whose `error` is `signed`, a note SHALL give `clm architecture decide <slug> --value high|medium|low --decision keep|cut|redesign|accept`, or the Worker's `command`.

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser
- architecture/humans-review-intent

#### Needs
- web.worker/architecture-decide

#### Scenario: one request per tap
- GIVEN entry `e` at version `v`, online, with value medium chosen
- WHEN Redesign is tapped twice in quick succession and the Worker accepts the first
- THEN exactly one request posted `{"version": "v", "value": "medium", "decision": "redesign"}` to `/architecture/e/decide`, the architecture report was asked for again, and the page shows the decision

#### Scenario: signed
- GIVEN a page whose report has `signed` true
- WHEN it is shown
- THEN the decision buttons are disabled and the note gives `clm architecture decide e --value high|medium|low --decision keep|cut|redesign|accept`

### Requirement: says-architectural-constraint
Every file under `web/src/` SHALL call an entry of `ARCHITECTURE.md` an architectural constraint, on screen and in comments, and none SHALL hold the old term (the word `architecture` followed by the word `requirement`, singular or plural, in any case). With no entries the Architecture view SHALL say `No architectural constraints: ARCHITECTURE.md holds none.`; with entries and none shown under "⚑ only", `No architectural constraint is flagged.`; and the page of a slug the report doesn't hold, `The report holds no architectural constraint <slug>.`

#### Scenario: an empty report
- GIVEN an architecture report with no entries
- WHEN the Architecture view is shown
- THEN it says `No architectural constraints: ARCHITECTURE.md holds none.`

#### Scenario: the shipped app
- GIVEN the repository
- WHEN the web app's tests run
- THEN no file under `web/src/` holds the old term

#### Serves
- architecture/plain-terms
- capability/follow-from-browser

### Requirement: says-component
The text the web app shows SHALL say component and components, never the old words, as `component-words` defines them: its headings, labels, filters, notes and counts (the graph's and the repository header's component count, the Implementation view's `Components` section and `Pending components` stat, the backlog's component filter and picker, the PR view's `Components touched` and its note when no component changed, the inspector's empty note, and the capability and architecture pages' cost note, `a component's cost is split by its rules' share`). The JSON fields it reads and sends, CSS classes, element IDs and `data-` attributes, and the imports of Node.js modules are data or code, and keep their names (`component-words`). A test reads the string literals and template text in `web/src/` but its tests and fails naming the file, the line and the word for each old word `component-words` doesn't keep.

#### Scenario: the Implementation view
- GIVEN a snapshot with two pending components
- WHEN the Implementation view renders
- THEN it shows the heading `Components` and the stat `Pending components`, and no shown text holds an old word

#### Serves
- architecture/plain-terms

### Requirement: oversized-component
The graph view SHALL mark each component whose `spec` model entry has `oversized` `true` with `⚠` on its card and an amber outline, unless a queue-state ring colours the card, and while any component is oversized its legend's amber entry SHALL also name `oversized`. A component whose `oversized` is `false` or absent SHALL carry neither.

#### Serves
- capability/see-project-state
- architecture/slim-components

#### Needs
- replica/snapshot-size

#### Scenario: oversized and not
- GIVEN a `spec` model where component `a` has `oversized` true and nothing pending, component `b` has `oversized` false, and component `c` has no `oversized`
- WHEN the graph is shown
- THEN `a`'s card shows `⚠` with an amber outline and the legend names `oversized`, and `b`'s and `c`'s cards show no `⚠`

#### Scenario: queue ring wins the outline
- GIVEN component `a` with `oversized` true and queue state `blocked`
- WHEN the graph is shown and the implementation state has arrived
- THEN `a`'s card shows `⚠` and the `blocked` ring in rose

### Requirement: says-new-state-words
No file under `web/src/`, its tests included, SHALL hold the old name `plain-terms-renames` gives for the component status `unbuilt` as a word of its own, nor the old name it gives for the backlog kind `workaround` in the forms `clm/retired-terms` lists it, after `clm` and a space or as `kind: <old name>`, matched as `clm/retired-terms` matches; the latter as a plain English word is free. The app SHALL show a component's status as the `spec` model gives it and map no other word to `unbuilt`: the model writes `unbuilt` and reads the old word only from files (`renamed-words-still-load`), so it never hands the app the old word. The doc comment on the type of a capability's open workaround flags SHALL call one `A workaround flag open on a capability`. A test that needs either old word SHALL build it from pieces, so the word never appears whole in the file.

#### Scenario: an unbuilt component
- GIVEN a snapshot whose `spec` model gives component `b` the status `unbuilt`
- WHEN the inspector opens on `b`
- THEN its header names the status `unbuilt`

#### Scenario: the shipped app
- GIVEN the repository
- WHEN the web app's tests run
- THEN no file under `web/src/` holds the first old name as a word of its own or the second in either listed form, and a line using the second as a plain English word passes

#### Serves
- architecture/plain-terms

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

### Requirement: zoom-at-pointer
Zooming the graph SHALL keep the point under the pointer where it is on screen: the wheel zooms around the pointer, a pinch around the midpoint between the two pointers, and the zoom buttons around the canvas's centre. The zoom's limits SHALL be unchanged.

#### Scenario: wheel over a card
- GIVEN the spec graph with node `b`'s card under the pointer
- WHEN the person zooms in with the wheel
- THEN the point of `b`'s card under the pointer is still under it, within a pixel

#### Scenario: buttons
- GIVEN the spec graph, with the point at the canvas's centre on node `a`'s card
- WHEN the person taps Zoom in
- THEN that point of `a`'s card is still at the canvas's centre

#### Serves
- capability/follow-from-browser
- architecture/ui-quality

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
