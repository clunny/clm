---
component: web.views
base: { design: "a24bddcece8b48c9" }
---

## MODIFIED Design
The modules import nothing of `web`'s outside `web/src/views/`, since an import of `web`'s code from here would need `depends_on: [web]`, a cycle with `web`'s dependency on this component (`check/undeclared-imports`). What a page needs of the app (the responses on screen, what the user set, posting and loading again) comes in through its parameters. `web/src/styles.css`, which styles every page, stays `web`'s: a stylesheet is no import. The build that makes this component moves the modules from `web/src/` and `web/test/fixtures.ts` here, keeping their tests, and re-points the tests' markers at this component's requirement IDs, `web`'s end-to-end tests among them. Graph layout uses ELK (`elkjs`, layered, edges from dependent to dependency) and word-level diffs use `diff` (jsdiff). The mockups in `web`'s `assets/mockups/` are the reference for layout, colour, type and interaction, as `web`'s Design says.

**Views.**
- *Home* (`home-waiting-on-you`): what waits on the person, where a repo opens: the spec branches whose review needs it and has no approval of the branch's fingerprint, from the spec-branch list and each branch's review; the `question` items with `choices` and the `human-task` items, each with its answer buttons (`answer-buttons`); the capabilities and architectural constraints the reports flag; and the other `question` items, each marked to be answered in a session. Each entry links to its page. A section with nothing in it is left out, and a Home with nothing says nothing waits.
- *Spec graph*: an SVG of components and `depends_on` edges, each component drawn inside its parent's box, the root's children inside the root. Each component card shows its status (active or unbuilt), its requirements as a bar segmented by coverage state (current green, behind amber, unbuilt sky) with `n/m` current of total and `✕n` obsolete markers when there are any, and its owners; once the implementation state has arrived, a component that isn't `converged` is ringed in its queue state's colour and names the state. A component the `spec` model marks `oversized` carries `⚠` in its card's label and an amber outline, unless a queue-state ring colours it, as `view`'s SVG draws it (`oversized-is-a-state`, `oversized-component`); a component whose `oversized` is absent or `false` carries neither. The legend has one entry per colour the graph uses, naming its meanings on the graph. A selected PR rings its touched components and highlights their edges. Selecting a component draws the edges from the components that depend on it in one colour and the edges to the components it depends on in another, two colours that mean nothing else, and dims the other edges (`edge-direction`); the legend names both. It also opens the inspector over the graph, never beside it: a drawer from the right edge on desktop, as wide as 640 px or 90% of the window, whichever is less, and a bottom sheet on phones, closed by its × button, by Escape or by selecting the component again (`inspector-overlay`). The inspector holds the component's contract, dependencies, changes in open PRs, work in flight and backlog items, each item linking to its page.
- *Spec PRs*: a spec branch's review, which this component's requirements call a PR and its views label "PR" whether or not the host has a page for it (`spec-branch-review`). The list is the Worker's spec-branch list (`GET .../branches`): each `spec/*` branch by name, with its page's number and title, linking out to the page, when it has one. Each entry opens the branch's review, routed by the branch name as one encoded segment (`#/<owner>/<repo>/prs/spec%2Fx`, so `spec/x/graph` and `.../full/<path>` stay unambiguous), while requests to the Worker keep its slashes (`/branches/spec/x/review`), loaded from `GET .../branches/{branch}/review`, with Document and Graph tabs and Approve intent. Its header shows "needs review" (`gate/needs-review`), who approved it (each `approved` record's writer and time from the review's `approvals`, or "not approved"), "checked" or "not checked" from `checked`, and a link to the page when there is one (`spec-branch-review`). Approve posts to `POST .../branches/{branch}/approve` (`approve-by-branch`, `approve-button`); on success it loads the review again, so the approval shown is the ledger's, and a 409 is the Worker's answer when the branch's fingerprint isn't the one sent. In a signed repository, the review's `signed` (`signed-approvals`, `web-approve-off-when-signed`), the note beside Approve, in the Approve bar (pinned above the tab bar on phones), shows the command `clm approve <branch>` with a copy button. The header's approvals are drawn neutral rather than green, each marked "signed" (sky: `clm land` will check it) or "unsigned, doesn't count" (rose) from its `signed`. A signed approval is one `clm land` checks, so the app never calls it counted. A 422 `signed` means the repository became signed after the review was drawn, so the note appears once the review is loaded again. The note shows offline too. **Request changes** (`request-changes`, `web-review-feedback-as-ledger-records`) sits beside Approve, in the Approve bar on phones: it opens a one-field form for the request's text and posts to `POST .../branches/{branch}/request-changes`. It works in a signed repository, where Approve doesn't, since a request withholds a land rather than granting one. The header draws each held request in rose, naming the person part of the writer as the activity feed names actors; a request that isn't held is drawn with the comments under the intent, dimmed. Request changes isn't disabled by an approval or by an earlier request: a second request by the same person is another text. A 403, a reader without write access, says so and leaves the button enabled. **Comments** (`comment-on-review`): the Document tab's intent and each requirement card carry a Comment control that opens a one-field form and posts to `POST .../branches/{branch}/comments`. A 422 shows the Worker's `error` under the form and keeps it. The Document tab shows the intent, then per component each changed element as a card with a couple of dimmed lines of context and word marks, then an impact summary, decisions and backlog edits. **Open in full** shows a changed file whole, as it reads at the head, with unchanged text dimmed, long unchanged runs folded, every change marked and numbered, and an outline marking where they are. It opens scrolled to the first change. ↑/↓ and `j`/`k` step through the changes, and a switch shows changes inline, after or before.
- *Implementation* (Build): counts (pending components by state: claimed, ready and blocked; live changes; requirements still to make current), then a switch between **In flight** and **Blockers**. In flight shows the `flight` view (`implementation-view`); a branch's pull request shows "checked" or "not checked" from `pages`, and nothing while the response's `pages` doesn't yet say. Blockers shows the `blockers` view (`blockers-view`) with the chain to each held-up component. An activity feed (`activity-feed`) names the branch of a land or an approval.
- *Backlog*: each item opens its page (`backlog-item-page`) at `#/<owner>/<repo>/backlog/<slug>`: its title, kind, priority, components, origin, the requirements it blocks, what holds it, the answers given and its answer buttons, then its body. A **Waiting on you** row first (`backlog-view`, `requirement-blocks`), then columns by kind (bug, question, deferred, idea). An item something holds (`propose-claims`, `backlog-taken`) carries a mark from the implementation state's `taken`; a bare claim is a live `propose/<slug>` claim. The backlog is drawn from the snapshot first and takes the marks when the implementation state arrives, as the graph takes its rings. The sidebar and tab bar badge the number of items waiting. **New proposal** (`propose-from-backlog`, `web-proposals-as-ledger-records`): a button above the columns opens a form with a title and an intent (plain text or Markdown), posting `/nodes` `[]` to `POST .../proposals`: the draft that takes the item up decides its components, as a session filing an item does; a 422 shows which field the Worker's `error` names and keeps the form. A row **Proposed** sits between Waiting on you and the columns; **Withdraw** sends `DELETE .../proposals/{id}`.
- *Capabilities* (`review-capability-cost`): every capability's cost and its review, drawn from the Worker's `GET .../capabilities` (`web.worker/capabilities`), which has no history, so commits, questions answered, blocking and sizes are absent and the views say to run `clm capabilities <slug>` for them. The list (`capabilities-list`) keeps the report's order, by cost, highest first; narrower than 900 px each capability is a card with a bar for its share of the costliest. A capability opens its page (`capability-page`) at `#/<owner>/<repo>/capabilities/<slug>`, with its entry in full from the report's `text`, then the sections in `clm capabilities <slug>`'s order, then the decision buttons (`decide-buttons`). A decision is posted to `POST .../capabilities/<slug>/decide` (`web.worker/decide`) with the capability's version as shown (a page drawn again meanwhile keeps the buttons off), and the report is loaded again from the Worker with `?fresh=` and a random delivery, so the Worker reads the ledgers anew rather than its minute-old copy. On another 422 (the capability left the tip, or its specs don't load) the page shows the tip as it is now. A 401 shows the sign-in page. When the report's `signed` is true, a note says this repository counts only records signed with the reviewer's SSH key and gives `clm capabilities decide <slug> --value high|medium|low --decision keep|cut|redesign|accept`.
- *Architecture* (`review-capability-cost`, `docs/decisions/architecture-review.md`): every architectural constraint, whether it is held and what it costs, with its review, drawn from the Worker's `GET .../architecture` (`web.worker/architecture`). It is a view of its own, after Capabilities in the sidebar and the tab bar, not a tab inside Capabilities. Its cost is a second split, apart from the capabilities', and the view says so. The list (`architecture-list`) keeps the report's order, by cost, highest first; narrower than 900 px each entry is a card. An entry opens its page (`architecture-page`) at `#/<owner>/<repo>/architecture/<slug>`, with its entry in full from the report's `text`, then the decision buttons (`architecture-decide-buttons`), which post to `POST .../architecture/<slug>/decide` (`web.worker/architecture-decide`) and load the report again as a capability's buttons do. While the report's `indexing` is true, each enforcing rule's state is drawn as unknown, and the view says coverage is still indexing.

**Size** (`slim-components`). This component's code budget is 90,000 tokens, above the default, because every page of the one app draws from the same types, DOM helpers and fixtures, which a split into pages would leave in a shared layer as large as any page.

**Markdown** (`repo-markdown`). Every text the app shows from the repository (a component's contract, an intent, a requirement, a backlog item's body, a capability's or a constraint's statement and entry, a decision) goes through `markdown.ts`, which renders it with `marked` and sanitises it with DOMPurify, since anyone who can open a PR writes it and it is shown where the session cookie can approve. Only IDs, slugs and titles are drawn as plain text.

## MODIFIED Requirements
### Requirement: backlog-view <!-- base: 3bb10ff365e56596 -->
The backlog view SHALL show every backlog item grouped by kind, with its title, components and slug, each linking to its page (`backlog-item-page`), and SHALL filter by component. It SHALL list `question` and `human-task` items first, under "Waiting on you", each with the requirements it blocks.

#### Scenario: filter
- GIVEN items on components `a` and `b`
- WHEN the view is filtered to `a`
- THEN only items on `a` are shown

#### Scenario: waiting on you
- GIVEN a `human-task` item `setup` whose `blocks` names `c/a`, and an `idea` item
- WHEN the view is shown
- THEN `setup` is listed under "Waiting on you" naming `c/a`, before the `idea` item

#### Scenario: an item opens its page
- GIVEN repo `o/a` whose backlog holds `idea` item `dark-mode`
- WHEN the view is shown and `dark-mode` is tapped
- THEN the page at `#/o/a/backlog/dark-mode` is shown

#### Needs
- model/parse-backlog

### Requirement: propose-from-backlog <!-- base: 9b8191c8246096b2 -->
The Backlog view SHALL offer New proposal: a form with a title and an intent, and no picker of components, that posts an object with `/title`, `/body` and `/nodes` `[]` to the Worker's proposals route once per tap and none while one is in flight, on success closes and asks for the implementation state again, on a 409 says the title is taken and keeps the form, and on a 422 shows the answer's reason. It SHALL list the implementation state's `proposals` under Proposed, between Waiting on you and the columns, each with its title, the person part of its proposer and its time and a note that the next build files it, with Withdraw on those whose `mine` is true, which sends one `DELETE` for the proposal's `id` and then asks for the implementation state again. Both SHALL be disabled offline.

#### Scenario: proposed
- GIVEN the Backlog of `o/a` online, and a Worker that answers the proposal with 200 and then lists it in `proposals` by `alice/web`
- WHEN the user opens New proposal, enters the title "Export CSV" and the intent "Let me export.", and submits twice in quick succession
- THEN exactly one request posted `/title` `"Export CSV"`, `/body` `"Let me export."` and `/nodes` `[]`, the form holds no picker of components, the form closed, and Proposed lists "Export CSV" with `alice` and the note

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

### Requirement: capability-page <!-- base: 853bfb399a21f59f -->
A capability's page SHALL show, in this order: its entry in full, the report's `text` rendered as Markdown (`repo-markdown`), in place of its statement and why we need it; what it costs; its workarounds, each a `<details>` whose summary is its title and whose body is its text, can we live without it from its `overlap`, its rules collapsed, and its decisions, with a line saying to run `clm capabilities <slug>` for its commits, questions answered, blocking and size, which the Worker's report doesn't hold.

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser

#### Needs
- web.worker/capabilities

#### Scenario: sections
- GIVEN capability `x` whose `text` holds `` `clm land` `` and a `#### Scenario: s` block, with an unsettled flag titled "Judge ignores pipes" whose text is "Builds pipe output."
- WHEN its page is shown
- THEN the sections come in that order, the entry shows `clm land` as code and the scenario `s`, the workaround's summary reads "Judge ignores pipes" and its body "Builds pipe output.", and the page names `clm capabilities x`

### Requirement: architecture-page <!-- base: d01678bdd9888c9d -->
An architectural constraint's page SHALL show, in this order: its entry in full, the report's `text` rendered as Markdown (`repo-markdown`), in place of its statement and its `#### Untested` text; whether it is held, each enforcing rule with its state (current in green, pending in amber, unknown while the report is indexing); what it costs, saying it is counted apart from the capabilities; its workarounds, each a `<details>` as on a capability's page; can we live without it from its `overlap`; and its decisions; then the decision buttons (`architecture-decide-buttons`).

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser

#### Needs
- web.worker/architecture

#### Scenario: sections
- GIVEN entry `e`, whose `text` holds its `#### Enforced by` and `#### Untested` blocks, enforced by `a/x`, current, and `a/y`, pending, with the untested text "Partly judgement." and a flag titled "Gate misfires"
- WHEN its page is shown
- THEN the sections come in that order, the entry holds "Partly judgement.", `a/x` is drawn current and `a/y` pending, and the workaround's summary reads "Gate misfires"

## ADDED Requirements
### Requirement: home-waiting-on-you
The Home view SHALL list, in this order, each section left out when empty: **Review**, each spec branch whose review has `review` true and no approval in `approvals`, by name, linking to its review; **Answer**, each `question` item with `choices` and each `human-task` item, by title, linking to its page, with its answer buttons (`answer-buttons`); **Flagged**, each capability and architectural constraint whose report entry has `flagged` true, linking to its page; and **Answer in a session**, each other `question` item, linking to its page and saying it is answered in a `/clm:triage` session. A Home with every section empty SHALL say nothing waits on the person. The number of entries in Review and Answer SHALL be the count the sidebar and tab bar badge Home with.

#### Scenario: what waits
- GIVEN repo `o/a` with spec branches `spec/x`, whose review needs review and has no approval, and `spec/y`, whose review needs review and has one; question `q` with choices `yes` and `no`; question `r` with none; `human-task` `h`; and a capabilities report flagging `c`
- WHEN Home is shown
- THEN Review lists `spec/x` only, Answer lists `q` and `h`, each with its buttons, Flagged lists `c`, Answer in a session lists `r`, and Home's badge is 3

#### Scenario: nothing waits
- GIVEN a repo with no spec branch needing review, no `question` or `human-task` item and nothing flagged
- WHEN Home is shown
- THEN it says nothing waits on the person and lists no section

#### Serves
- capability/follow-from-browser

#### Needs
- replica/snapshot-backlog-text

### Requirement: backlog-item-page
A backlog item's page, at `#/<owner>/<repo>/backlog/<slug>`, SHALL show its title, kind, priority, components, origin when it has one, the requirements it blocks, what holds it (`backlog-taken`), each open answer to it with the person part of its writer and its time, and its answer buttons (`answer-buttons`), then its body rendered as Markdown (`repo-markdown`). A slug the snapshot's backlog doesn't hold SHALL show that the item isn't in the backlog, with a link to the Backlog.

#### Scenario: an item in full
- GIVEN the snapshot's item `q`, a `question` with `priority` `P2`, components `a`, `blocks` naming `a/x`, and the body `Should we **ship**?`, and an open answer `yes` by `alice/web`
- WHEN `#/o/a/backlog/q` is shown
- THEN the page shows its title, `question`, `P2`, `a`, `a/x`, the answer `yes` by `alice`, and `ship` in bold

#### Scenario: gone
- GIVEN a snapshot whose backlog doesn't hold `gone`
- WHEN `#/o/a/backlog/gone` is shown
- THEN it says the item isn't in the backlog and links to the Backlog

#### Serves
- capability/follow-from-browser
- capability/keep-a-backlog

#### Needs
- replica/snapshot-backlog-text
- web.worker/answers

### Requirement: answer-buttons
A `question` item with `choices` SHALL offer one button per choice, in their order, and a `human-task` item one button, Done, which sends `done`; no other item SHALL offer any. A tap SHALL post `/answer` to `POST .../backlog/{slug}/answer` once, and none while one is in flight, then ask for the implementation state again, so the answer shows from the response's `answers`. The button of an answer the person gave (an answer whose `mine` is true) SHALL be drawn chosen, and tapping another SHALL replace it. A 422 or a 404 SHALL show why under the buttons and keep them enabled. The buttons SHALL be disabled offline.

#### Scenario: answered
- GIVEN item `q` with choices `yes` and `no`, online, and a Worker that answers 200 and then lists the answer `yes` by `alice/web` with `mine` true
- WHEN `yes` is tapped twice in quick succession
- THEN exactly one request posted `/answer` `"yes"` to `/api/repos/o/a/backlog/q/answer`, the implementation state was requested again, and `yes` is drawn chosen

#### Scenario: done
- GIVEN `human-task` item `h`
- WHEN its page is shown and Done is tapped
- THEN one request posted `/answer` `"done"` to `h`'s route

#### Scenario: offline
- GIVEN item `q` with choices, offline
- WHEN its page is shown
- THEN its buttons are disabled

#### Serves
- capability/follow-from-browser
- architecture/per-writer-ledgers

#### Needs
- web.worker/answers

### Requirement: edge-direction
When a component is selected, the graph SHALL draw each edge from a component that depends on it in the incoming colour, each edge to a component it depends on in the outgoing colour, and every other edge dimmed, and the legend SHALL name the two colours. With no component selected, every edge SHALL be drawn alike.

#### Scenario: traced
- GIVEN components `a`, `b` and `c`, where `b` depends on `a` and `a` depends on `c`, and `d`, which depends on `c`
- WHEN `a` is selected
- THEN the edge from `b` to `a` is drawn in the incoming colour, the edge from `a` to `c` in the outgoing colour, the edge from `d` to `c` dimmed, and the legend names both colours

#### Scenario: nothing selected
- GIVEN the same graph and no selection
- WHEN the graph is shown
- THEN no edge is drawn in either colour or dimmed

#### Serves
- capability/see-project-state

### Requirement: inspector-overlay
The component inspector SHALL open over the graph, never narrowing it: on desktop a drawer along the right edge at most 640 px wide and 90% of the window, scrolling within the window's height; on phones a bottom sheet. It SHALL close on its × button, on Escape and on selecting the selected component again, which also clears the selection, and its backlog items SHALL link to their pages.

#### Scenario: desktop
- GIVEN a 1280 px wide viewport on the graph
- WHEN a component is selected
- THEN the graph's canvas is as wide as before, the inspector overlaps it from the right edge and is at most 640 px wide, and Escape closes it

#### Scenario: an item from the inspector
- GIVEN component `a` with backlog item `dark-mode`
- WHEN `a` is selected and `dark-mode` is tapped in the inspector
- THEN the page at `#/o/a/backlog/dark-mode` is shown

#### Serves
- capability/see-project-state

### Requirement: repo-markdown
Every text the app shows from the repository that the Worker gives as Markdown (a component's contract, a change's intent, a requirement's text, a backlog item's body, and a capability's or constraint's statement and `text`) SHALL be rendered as Markdown and sanitised before it reaches the page, in the lists and on the pages alike.

#### Scenario: code in a statement
- GIVEN a capabilities report whose capability `x` has the statement ``Run `clm land` <img src=x onerror=alert(1)>``
- WHEN the list and `x`'s page are shown
- THEN each shows `clm land` in a `code` element, no backtick, and no `onerror` attribute

#### Serves
- capability/follow-from-browser
- capability/see-project-state
