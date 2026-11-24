---
component: web.views
base: { contract: "399bb6010de9b8b2", design: "0c110b68a89b9e3c" }
---

## MODIFIED Contract
The views of `web`'s browser app, modules under `web/src/views/`, each with its Vitest tests beside it, which `web/src/main.ts` imports and wires to the page:
- the pages' render calls, each taking the Worker's responses it draws from and the callbacks for what the page does (post, load again, navigate, close) and returning the page's DOM: the spec graph (`graph.ts`: `layoutGraph`, which lays a `spec` model out and answers asynchronously; `renderGraph`, which starts from a pan and zoom (`Viewport`) its caller gives and reports each change of it; and `queueStates`, `ringGraph` and `traceEdges`, which take an implementation state's queue states and a selection into a drawn graph in place) and its inspector (`inspector.ts`); Spec PRs (`pr.ts`) and the full document (`full-document.ts`), whose call answers a handle that steps through the changes and gives and restores its place across a redraw; Build (`implementation.ts`); Backlog and a backlog item's page (`backlog.ts`, with `waiting`, the items waiting on a person); Capabilities (`capabilities.ts`, with `decide`, which posts a decision through the `post` its caller gives and loads the report again through its caller) and Architecture (`architecture.ts`); Home (`home.ts`, with `homeCount`, the count Home is badged with); and the answer buttons Home and an item's page both draw (`answers.ts`: `answerButtons`, `offered` and the `Answering` they take);
- what they share with `web`'s shell: the Worker API's types (`types.ts`); the one module for requests to the Worker (`api.ts`: `get`, with options for a background refresh, a `fresh` delivery and an answer not to keep, `post`, `send`, `apiBase`, and the failures `SignedOut`, `Offline` and `HttpError`, which carries the status and a preparing share); the hash routes (`route.ts`: `parseRoute`, `href`, `reviewHref` and `reviewPath`); DOM building (`dom.ts`); sanitised Markdown (`markdown.ts`); the one-field forms (`forms.ts`); times (`time.ts`) and word marks (`diff.ts`);
- the fixtures the views' tests and `web`'s end-to-end tests draw from (`fixtures.ts`), shaped as the Worker's responses are.

## MODIFIED Design
The modules import nothing of `web`'s outside `web/src/views/`, since an import of `web`'s code from here would need `depends_on: [web]`, a cycle with `web`'s dependency on this component (`check/undeclared-imports`). What a page needs of the app (the responses on screen, what the user set) comes in through its parameters, and what it does goes out through its callbacks: the pages draw and call, and `web`'s shell routes, posts, loads again and redraws (`web/approve-by-branch`, `web/review-in-the-browser`, `web/backlog-in-the-browser`, `web/answer-from-app`). `decide` is the one call here that posts, through the `post` its caller gives. `web/src/styles.css`, which styles every page and picks a list's table or its cards by the window's width, stays `web`'s: a stylesheet is no import. Graph layout uses ELK (`elkjs`, layered, edges from dependent to dependency) and word-level diffs use `diff` (jsdiff). The mockups in `web`'s `assets/mockups/` are the reference for layout, colour, type and interaction, as `web`'s Design says.

**Views.**
- *Home* (`home-waiting-on-you`): what waits on the person, where a repo opens, from the snapshot, the spec branches' reviews that have arrived, the implementation state's answers and the two reports; each entry links to its page.
- *Spec graph* (`graph-view`, `oversized-component`, `edge-direction`, `zoom-at-pointer`): an SVG of components and `depends_on` edges, each component drawn inside its parent's box, the root's children at the top level and the canvas labelled `<root> · root`, marked as `view`'s SVG marks them (`oversized-is-a-state`). The graph doesn't wait for the implementation state: the rings, and the inspector's work in flight, are taken in place when it arrives, so the graph keeps its pan and zoom. Selecting a component opens its inspector (`inspector`, `inspector-pending`, `inspector-overlay`), which `web` lays over the graph.
- *Spec PRs* (`spec-branch-review`): a spec branch's review, which this component's requirements call a PR and its views label "PR" whether or not the host has a page for it, with Document and Graph tabs (`review-intent-first`, `pr-document`) and the Approve bar: Approve intent (`approve-button`, `signed-approvals`, `web-approve-off-when-signed`) and Request changes (`request-changes`, `web-review-feedback-as-ledger-records`). Request changes works in a signed repository, where Approve doesn't, since a request withholds a land rather than granting one, and an approval or an earlier request doesn't disable it: a second request by the same person is another text. In a signed repository the header's approvals are drawn neutral rather than green, each marked by whether `clm land` will check it, so the app never calls one counted. Comments (`comment-on-review`) sit under their targets. **Open in full** (`full-document`) shows a changed file whole.
- *Implementation* (Build): counts, then a switch between **In flight**, the `flight` view (`implementation-view`, `queue-states`), and **Blockers**, the `blockers` view (`blockers-view`), beside the activity feed (`activity-feed`).
- *Backlog* (`backlog-view`, `backlog-taken`, `requirement-blocks`, `propose-claims`): drawn from the snapshot first, taking the marks when the implementation state arrives, as the graph takes its rings. **New proposal** (`propose-from-backlog`, `web-proposals-as-ledger-records`) has no picker of components: the draft that takes the item up decides them, as a session filing an item does. Each item opens its page (`backlog-item-page`), with its answer buttons (`answer-buttons`).
- *Capabilities* (`review-capability-cost`, `capabilities-list`, `capability-page`, `decide-buttons`): drawn from the Worker's `GET .../capabilities` (`web.worker/capabilities`), which has no history, so commits, questions answered, blocking and sizes are absent and the views say to run `clm capabilities <slug>` for them.
- *Architecture* (`architecture-list`, `architecture-page`, `architecture-decide-buttons`, `docs/decisions/architecture-review.md`): a view of its own, not a tab inside Capabilities, drawn from the Worker's `GET .../architecture` (`web.worker/architecture`) with the Capabilities view's pieces. Its cost is a second split, apart from the capabilities'.

**Forms.** The comment, request and proposal forms keep what was typed in one store of drafts, keyed by the scope their caller gives, so the redraws a refresh makes keep an open form and its text; a draft is taken out while it is sent and put back when the send doesn't go through.

**Size** (`slim-components`). This component's code budget is 90,000 tokens, above the default, because every page of the one app draws from the same types, DOM helpers and fixtures, which a split into pages would leave in a shared layer as large as any page.

**Markdown** (`repo-markdown`). Every text the app shows from the repository (a component's contract, an intent, a requirement, a backlog item's body, a capability's or a constraint's statement and entry, a decision) goes through `markdown.ts`, which renders it with `marked` and sanitises it with DOMPurify, since anyone who can open a PR writes it and it is shown where the session cookie can approve. Only IDs, slugs and titles are drawn as plain text.

## MODIFIED Requirements

### Requirement: full-document <!-- base: fcfa5b34ab44b5a7 -->
The PR view SHALL open any changed file whole, as it reads at the PR's head, with every change marked and numbered and its first change current, with controls to step to the next and previous change, each step scrolling the change it goes to into view. ↑ and ↓, and `j` and `k`, SHALL step to the next and the previous change, except while the focus is in a field, and stepping SHALL stop at the first and the last. A run of more than 3 unchanged paragraphs SHALL fold to its first and last, with `⋯ <n> unchanged paragraphs` for the `<n>` it hides, which opens it. An outline SHALL list the file's headings and mark those followed by a change before the next heading, and a heading tapped SHALL go to the first change after it. A switch SHALL show the changes inline, as after or as before, still numbered. A file added whole SHALL be one change, and a file with no change SHALL read `No changes`. Given the place its caller kept from an earlier drawing, it SHALL go back to the change whose text is the same, else to the change of the same number, the last when there are fewer.

#### Scenario: open at first change
- GIVEN a PR whose component spec changes a Design paragraph and a later requirement
- WHEN the reviewer opens that component in full
- THEN the Design change is in view and marked as change 1 of 2, and "next" brings the requirement into view

#### Scenario: stepping and folding
- GIVEN a file with two changes separated by 6 unchanged paragraphs and headed `## Design` and `### Requirement: r`, a file added whole, and a file with no change
- WHEN each is opened in full, and in the first `j` is pressed three times, `k` once, `j` once with the focus in a text field, the fold is tapped, the switch is set to after and then to before, `### Requirement: r` is tapped in the outline, and it is drawn again with the place it gave
- THEN the first reads `Change 2 of 2` after the third `j` and `Change 1 of 2` after the `k`, the `j` in the field steps nothing, the run reads `⋯ 4 unchanged paragraphs` until tapped and then shows them all, after and before each show one side still numbered, the outline marks both headings and the tap goes to change 2, and the redraw opens at the same change; the added file reads `Change 1 of 1`; and the unchanged one reads `No changes`

### Requirement: graph-view <!-- base: 7bdf4eb6d9064110 -->
The graph view SHALL draw every component and `depends_on` edge, each component inside its parent, with each component's status, owners and requirements as the `view` component's `spec` model gives them: a bar segmented by coverage state, current green, behind amber and unbuilt sky, with `<current>/<total>`, and the obsolete count when there is one. While the snapshot's coverage is indexing, each card SHALL read `indexing <n>%`, the share done, in place of its counts, with no segment in its bar; a component with no requirements SHALL draw an empty bar and `0/0`. The root SHALL NOT be drawn as a card or box: its ID labels the canvas as `<root> · root`. An edge to a component the model doesn't hold SHALL be left out. Once the implementation state has arrived with complete coverage, the graph SHALL ring each component that isn't `converged` in the colour of the queue state the `flight` model gives it, `ready` sky, `claimed` amber, `blocked` rose and any other state the muted colour, and name that state on the card; it SHALL take the rings in place when the state arrives or changes, a component no longer pending losing its ring, and draw none while the implementation state's coverage is indexing. The legend SHALL name each colour the graph uses with its meanings: `active · current` green, `unbuilt · ready` sky, `behind · claimed` amber, `obsolete · blocked` rose, `touched by PR` in the accent, and `→ depends on`, with `other queue state` in the muted colour only while a ring uses it. It SHALL mark the components a selected PR touches. Selecting a card or a box SHALL report that component to its caller, and selecting the component its caller gives as selected SHALL report none. The graph SHALL start from the pan and zoom its caller gives, and report each change of them.

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

#### Scenario: indexing
- GIVEN a snapshot whose coverage is `indexing` with `done` 0.4 and whose component `a` has 1 requirement, and a complete snapshot whose component `z` has none
- WHEN each graph is shown
- THEN `a` reads `indexing 40%`, not `1/1`, and its bar holds no segment; and `z` draws an empty bar and `0/0`

#### Scenario: rings arrive and change
- GIVEN a drawn graph of `a`, `b` and `c`, and implementation states in turn: one whose coverage is indexing with `a` `claimed`; one with `a` `claimed`, `b` `converged` and `c` `parked`; and one with `a` `blocked` and nothing else pending
- WHEN each is taken into the drawn graph in turn
- THEN the first rings nothing; the second rings `a` amber, `b` not at all and `c` in the muted colour named `parked`, and the legend shows `other queue state`; and the third rings `a` rose, `c` not at all, and the legend no longer shows `other queue state`

#### Scenario: legend
- GIVEN a graph with no selection, no ring in the muted colour and no oversized component
- WHEN it is shown
- THEN its legend reads, in order, `active · current` green, `unbuilt · ready` sky, `behind · claimed` amber, `obsolete · blocked` rose, `touched by PR` in the accent, and `→ depends on`

#### Scenario: root and missing ends
- GIVEN a `spec` model whose root is `clm`, with component `a` depending on `b` and on `x`, which the model doesn't hold
- WHEN the graph is shown
- THEN the canvas is labelled `clm · root`, no card or box is drawn for `clm`, and the only edge is from `a` to `b`

#### Scenario: selection and viewport
- GIVEN a nested component `check` with child `check.drift`, and a graph drawn with `check` given as selected and the pan and zoom `x` 10, `y` 20 at 1.5 times
- WHEN `check`'s box is selected, then `check.drift`'s card, and then the graph is panned
- THEN the stage is drawn at 10, 20 and 1.5 times, the graph reports none, then `check.drift`, and the pan is reported with the new place

#### Needs
- check.coverage/state-names
- view/spec-view

### Requirement: implementation-view <!-- base: 3ce3f29a4aaed782 -->
The implementation view SHALL show the `view` component's `flight` model (`view/flight-view`). First its counts: the pending components, with `<n> <state>` for each state the totals give, `claimed`, `ready` and `blocked` first in that order (`queue-states`); the live changes; and the requirements still to make current. Then the pending components in the model's order, each with its queue state; for a claimed component, its last push, the time its claim lapses less the response's `expiry_hours`, and how long until the claim lapses, or `lapsed` once it has; for a component whose `impl/<id>` branch has a page in the response's `pages`, that page's number linking to it and `checked` or `not checked` from its `checked`, or nothing about the check while `pages` doesn't say; its pending requirements by state, each followed by what blocks it, the items in its `blocked_by` and the requirements in its `waits_on`; and for each entry of its `blocked_by`, the item or the requirement, then, for a requirement with a non-empty `components`, ` on ` and each of those components in the order given, separated by `, `, each linking to that component's entry, never to the component the first part of the ID names (`orchestrator/queue-from-refs`). A requirement blocker without `components`, as in a view stored before the queue gave them, SHALL be shown as its ID alone, with no link. An item blocker of kind `question` or `human-task` SHALL be marked as waiting on you. Then the live changes, in the model's groups and order, each with its requirements current of those it adds or modifies, and `complete` where it applies. With no pending component it SHALL say `Nothing is pending: every component is up to its spec.`, and with no live change `No live changes.`. Times SHALL be drawn against the clock its caller gives, so they stay current between redraws: a time past reads `just now` under a minute, and otherwise `<m> min`, under an hour, `<h> h`, under 48 hours, or `<d> d`, followed by `ago`, and a time ahead of the clock, as from a clock stepped back, reads `just now`; how long until a lapse reads `<h> h`, `<h> h <m> m` or, under an hour, `<m> min`.

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

#### Scenario: unchecked or unknown page
- GIVEN component `a` whose branch's page #7 has `checked` false, and component `b` whose branch's page #8 has no `checked`
- WHEN the view is shown
- THEN `a` shows `#7` and `not checked`, and `b` shows `#8` and nothing about the check

#### Scenario: waits on
- GIVEN a pending component `a` whose only pending requirement `a/x` has `waits_on` `["b/y"]` and an empty `blocked_by`
- WHEN the view is shown
- THEN `a/x` shows as blocked by `b/y`

#### Scenario: a blocker pending on another component than its ID names
- GIVEN a `flight` model in which component `b` is `blocked` by the requirement `b/gone` with `components` `[c, d]`, and component `e` by the requirement `a/x` with no `components`
- WHEN the implementation view is drawn
- THEN `b`'s entry reads `Blocked by b/gone on c, d`, `c` and `d` each linking to its own entry and nothing linking to `b`'s, and `e`'s entry reads `Blocked by a/x` with no link

#### Scenario: counts and empty
- GIVEN a `flight` model whose totals give 2 pending components, `ready` 1 and `claimed` 1, 3 live changes and 5 requirements to do; and one with no pending component and no live change
- WHEN each view is shown
- THEN the first's counts read `2`, `1 claimed, 1 ready`, `3` and `5`; and the second says `Nothing is pending: every component is up to its spec.` and `No live changes.`

#### Scenario: times
- GIVEN the clock at `t`, a response whose `expiry_hours` is 4, and claims lapsing at `t` + 3 h 20 min, at `t` + 12 min and at `t` − 1 s
- WHEN the view is shown, and drawn again with the clock an hour later
- THEN the first reads last push `40 min ago` and lapses in `3 h 20 m`, the second lapses in `12 min`, and the third reads `lapsed`; drawn again, the first reads `1 h ago` and lapses in `2 h 20 m`; and times 10 s, 38 min, 1 h and 3 days past read `just now`, `38 min ago`, `1 h ago` and `3 d ago`, and one 5 minutes ahead reads `just now`

#### Needs
- orchestrator/queue-from-refs
- replica/implementation-ledger

### Requirement: activity-feed <!-- base: a884943fdc7c6b35 -->
The implementation view SHALL list the repo's recent pushes, claims (creations of `impl/*` branches), lands and approvals, newest first: pushes and claims from GitHub's repo events, lands and approvals from the ledger's `landed` and `approved` records. A push SHALL read `Pushed to <ref>`, a claim `Claimed <ref>`, a land `<actor> landed <ref>`, an approval `<actor> approved <ref>`, and an entry of any other kind `<kind> <ref>`, each with how long ago, and its actor when it has one. With no entry the feed SHALL say `No recent activity.`

#### Scenario: approval shows
- GIVEN a ledger holding an `approved` record for the branch `spec/x` at 10:00 and repo events holding a push at 10:05
- WHEN the view is shown
- THEN the feed lists the push, then the approval of `spec/x`

#### Scenario: what each reads
- GIVEN an activity list holding a claim of `impl/a`, a land of `spec/x` by `alice`, an entry of kind `merge` on `main`, and, apart, an empty list
- WHEN each feed is shown
- THEN the first reads `Claimed impl/a`, `alice landed spec/x` and `merge main`, each with how long ago, the land's with `alice`; and the second says `No recent activity.`

#### Needs
- replica/implementation-ledger

### Requirement: backlog-view <!-- base: c6475681d29643f3 -->
The backlog view SHALL show every backlog item with its title, components and slug, each linking to its page (`backlog-item-page`). It SHALL list `question` and `human-task` items first, under "Waiting on you", each with the requirements it blocks; then the other items in columns by kind, `Bugs`, `Deferred` and `Ideas`, then a column for each other kind, named by the kind, in the order the backlog first gives it. It SHALL filter by component, Waiting on you included, with a filter that lists `all` and each component an item names, sorted.

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

#### Scenario: other kinds and the filter
- GIVEN a `bug` item on `b`, an `idea` item on `a`, an item of kind `chore` on `a`, and a `question` item on `b`
- WHEN the view is shown, its filter's `a` is tapped, and the view is drawn filtered to `a`
- THEN the columns read `Bugs`, `Deferred`, `Ideas` and `chore`, in that order, the filter lists `all`, `a` and `b`, the tap reports `a` to its caller, and filtered the `question` item is gone from Waiting on you and the `bug` item from `Bugs`

#### Needs
- model/parse-backlog

### Requirement: blockers-view <!-- base: fb7c056139d00530 -->
The implementation view SHALL have a switch between `In flight` and `Blockers`, which shows the `view` component's `blockers` model in its order: each root blocker with its state, its weight and the components it holds up, a component held up through others followed by ` via ` and those components joined by ` › `, the roots that wait on the human marked as waiting on you. Each held-up component, and a root that is a component, SHALL link to its entry in the In flight list; any other root is shown by its name. With no root it SHALL say nothing is blocked.

#### Scenario: blocker weight
- GIVEN components `b` and `c`, each with one pending requirement, which needs `a/x`, uncited on claimed component `a`, and component `d` whose only pending requirement is blocked by `backlog/q` of kind `question`
- WHEN the Blockers switch is shown
- THEN `a` is listed with 2 components held up, and `backlog/q` with 1, marked as waiting on you and listed first

#### Scenario: nothing blocked
- GIVEN a queue with no `blocked` components
- WHEN the Blockers switch is shown
- THEN it says nothing is blocked

#### Scenario: chain
- GIVEN a `blockers` model whose root `a`, a component, holds up `d` via `b` and `c`
- WHEN the Blockers switch is shown and `d` is tapped
- THEN `d` reads `d via b › c`, `a` links to its entry, and the tap asks its caller to show In flight at `d`'s entry

#### Serves
- capability/see-project-state

### Requirement: queue-states <!-- base: 340afeed3af802d5 -->
The implementation view SHALL label the queue states `ready`, `claimed` and `blocked` (`builds-in-subagents`, `needs-alone-order`), give the totals' states in the order `claimed`, `ready`, `blocked`, any other after them, and show any other state, `waiting` included, by its name with no label of its own; the graph SHALL ring such a state in the colour of a state the app doesn't know (`graph-view`). Neither SHALL read a component's `paths` from the `flight` model.

#### Scenario: no interactive or waiting state
- GIVEN a `flight` model whose totals hold one `blocked`, one `ready`, one `claimed` and one `waiting` component, the last `w`
- WHEN the implementation view and the graph are drawn
- THEN the totals read `1 claimed, 1 ready, 1 blocked, 1 waiting`, `w` shows `waiting` with no label of its own, and `w`'s ring is the colour of a state the app doesn't know

#### Serves
- capability/see-project-state
- architecture/component-reconciliation

### Requirement: review-intent-first <!-- base: 563a463a8006312c -->
The PR view SHALL show the PR's `## Intent` section first, above the per-node changes, next to the approve button; for a change with no `## Intent` section, its whole intent there; and for a branch with no change, `This change has no change.md.`

#### Scenario: intent first
- GIVEN a spec PR whose `change.md` has an `## Intent` section
- WHEN the reviewer opens the PR
- THEN the view shows that section first, above the per-node changes, next to the approve button

#### Scenario: no intent section
- GIVEN a review whose change has no `## Intent` section, and a review with no change
- WHEN each is shown
- THEN the first shows its whole intent first, above the per-node changes, and the second says `This change has no change.md.` there

#### Serves
- capability/approve-intent
- capability/follow-from-browser
- architecture/humans-review-intent

### Requirement: spec-branch-review <!-- base: 26ee0085371042a3 -->
The PR list SHALL list the `spec/*` branches the Worker's branch list gives, each by name with its page's number and title when it has a page, or say `No spec branches.` when it gives none, and SHALL link each to its review by the link its caller gives for the branch, routed by branch name as one encoded segment, whose Worker path keeps the name's slashes (`hash-routes`). The review's header SHALL show the branch's name; "needs review" when the review's top-level `review` is true, whether or not it has a `change`, and nothing for it otherwise; the writer and time of each approval the review lists, or "not approved" when it lists none; "checked" or "not checked"; a link to the page when there is one; the branch's components, what its change resolves, and `<branch> → <base>`; and SHALL show no labels. Under the header, above the review's components, it SHALL list each line of the review's `cuts` as the Worker gives it, under the heading `Cuts`, and show no such heading for a review with none, so a person approving in the app reads what the branch removes before they tap Approve (`removal-needs-a-yes`). A review with no component change SHALL say `No component changes.` The PR list SHALL list nothing but the spec branches: no lands to review.

#### Scenario: branch without a page
- GIVEN the branch list holds `spec/a` with page #7 "Add a" and `spec/b` with no page
- WHEN the PR list is shown and `spec/b`'s review is drawn
- THEN both are listed, `spec/a` with #7 and "Add a", `spec/b`'s review is routed at `#/o/a/prs/spec%2Fb` with the Worker path `branches/spec/b`, and its review shows no page link

#### Scenario: approved by
- GIVEN `spec/x`'s review lists an approval by `clunny/web` and says `checked` false
- WHEN its review is shown
- THEN the header names `clunny/web` with the approval's time and says "not checked"

#### Scenario: needs review without a change
- GIVEN `spec/arch`'s review with `review` true and no `change`, and `spec/plain`'s with `review` false and a `change`
- WHEN each review is shown
- THEN `spec/arch`'s header shows "needs review" and `spec/plain`'s doesn't

#### Scenario: a branch that cuts
- GIVEN `spec/x`'s review whose `cuts` hold `specs/n/SPEC.md: gate/cut: requirement n/r is gone`, and `spec/plain`'s with empty `cuts`
- WHEN each review is shown
- THEN `spec/x`'s shows `Cuts` with that line above its components, and `spec/plain`'s shows no `Cuts` heading

#### Scenario: empty
- GIVEN a branch list with no branch; and `spec/x`'s review against `main` changing no component, whose change resolves `crash`; and `spec/y`'s, changing `a` and `b`
- WHEN the list and each review are shown
- THEN the list says `No spec branches.`; `spec/x`'s header shows `resolves crash` and `spec/x → main`, and its document says `No component changes.`; and `spec/y`'s header names `a` and `b`

#### Serves
- capability/follow-from-browser
- capability/approve-intent

#### Needs
- web.worker.review/spec-branches
- web.worker.review/pr-review

### Requirement: approve-button <!-- base: 17d4c4226735a827 -->
Approve intent SHALL call its caller's approve once per tap and not again until the review is drawn anew. Drawn as accepted by its caller, or for a review whose `approvals` list one, it SHALL read Approved and be disabled, except in a signed repository (`signed-approvals`), reading an approval from the review's approvals, never from a label. It SHALL be disabled offline and for a review without a fingerprint.

#### Scenario: one request, then approved
- GIVEN a spec branch's review, online, with a fingerprint and no approval
- WHEN Approve is tapped twice in quick succession, and the review is drawn again as accepted
- THEN the approve callback was called once, the button was disabled after the first tap, and drawn again it reads Approved and is disabled

#### Scenario: disabled
- GIVEN a review with no fingerprint, a review that already shows an approval, and a review with a fingerprint shown offline
- WHEN each is shown
- THEN Approve is disabled in all three, and the one whose review lists an approval reads Approved

#### Serves
- capability/follow-from-browser
- capability/approve-intent
- architecture/approval-policy

### Requirement: pr-document <!-- base: 0e70b0ab6efc9c77 -->
The Document tab SHALL show, per touched component, every changed requirement, Contract, Design, frontmatter and asset the review gives, each with whether it was added, modified or removed, its text with word marks between before and after and a couple of dimmed lines of context from its file, and a link to its file in full; an HTML asset previewed in a sandboxed frame with no permissions; then an impact summary, giving the number of components touched, the requirements as `<n> added`, `<n> modified` and `<n> removed` for each that has any, or `none`, and the number of Contracts and of Designs changed; then each decision the review gives with what it says, the backlog edits, and a list of the changed files, each linking to it in full.

#### Scenario: modified requirement, asset and decision
- GIVEN a review that modifies requirement `n/r` by adding the word `quickly`, adds asset `assets/m.html` to `n` and adds decision `d`
- WHEN the Document tab is shown
- THEN `n/r` is marked Modified with `quickly` marked as inserted and its old text shown, `m.html` is marked Added and previewed in a frame whose `sandbox` attribute is empty, and `d` is listed as added with its text

#### Scenario: links to the files
- GIVEN a review whose changes touch the root component `clm`, component `n` and component `m.n`
- WHEN the Document tab is shown
- THEN their links open `specs/SPEC.md`, `specs/n/SPEC.md` and `specs/m/n/SPEC.md` in full

#### Scenario: impact
- GIVEN a review touching `n`, which adds two requirements, removes one and changes its Contract, and `m`, which changes nothing but its frontmatter; and a review that changes no requirement
- WHEN each Document tab is shown
- THEN the first's impact reads 2 components touched, `2 added, 1 removed`, 1 Contract and 0 Designs, and its files list links each document the review holds in full; and the second's requirements read `none`

#### Serves
- capability/follow-from-browser
- capability/approve-intent
- architecture/humans-review-intent

### Requirement: signed-approvals <!-- base: 9bdf6473a905ede9 -->
When a spec branch's review says `signed`, the PR view SHALL draw Approve disabled, never reading "Approved", with a note beside it, online or offline, that the repository counts only approvals signed with the reviewer's SSH key and the command `clm approve <branch>` for that branch; SHALL mark each approval in the header "signed" or "unsigned, doesn't count" from its `signed`; and SHALL say "no signed approval" when it lists approvals and none is signed. When the review says `signed` false or doesn't say, the view SHALL show neither the note nor the marks.

#### Serves
- capability/follow-from-browser
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

#### Scenario: unsigned repository
- GIVEN `spec/x`'s review with `signed` false, a fingerprint and no approvals, and another review with no `signed` field
- WHEN each PR view is drawn online
- THEN Approve is enabled and no note or approval mark shows

#### Scenario: signed offline
- GIVEN `spec/x`'s review with `signed` true and no approvals
- WHEN the PR view is drawn offline
- THEN the note shows `clm approve spec/x` and the header says "not approved"

### Requirement: propose-from-backlog <!-- base: 9531d23bc59b4a97 -->
The Backlog view SHALL offer New proposal: a form with a title and an intent, and no picker of components, that calls its caller's propose with the trimmed title and intent once per tap and not again while it runs; with no title it SHALL call nothing and say `Give it a title first.`. When the call answers that it went through the form SHALL close, and otherwise show under the form the reason it gives, or `Not sent: offline.` when it fails, keeping the form and its text. What was typed into an open New proposal form SHALL survive the redraws a refresh makes, until it is sent or cancelled, kept apart for each repository the snapshot names (`web.worker/snapshot-names-its-repo`). It SHALL list the implementation state's `proposals` under Proposed, between Waiting on you and the columns, each with its title, the person part of its proposer and its time and a note that the next build files it, and show no Proposed row before the implementation state arrives or when it lists none; those whose `mine` is true SHALL carry Withdraw, which calls its caller's withdraw with the proposal's `id` once per tap and, when that is refused, shows under the proposal the reason it gives, or `Not withdrawn: offline.` when it fails, and can be tapped again. Both SHALL be disabled offline.

#### Scenario: proposed
- GIVEN the Backlog of `o/a` online, and a propose callback that answers that it went through, and then an implementation state listing the proposal in `proposals` by `alice/web`
- WHEN the user opens New proposal, enters the title "Export CSV" and the intent "Let me export.", and submits twice in quick succession, and the view is drawn with that implementation state
- THEN the callback was called once, with "Export CSV" and "Let me export.", the form holds no picker of components, the form closed, and Proposed lists "Export CSV" with `alice` and the note

#### Scenario: title taken
- GIVEN a propose callback that answers "That title is taken: choose another."
- WHEN the user submits
- THEN the form stays open with its text, showing that reason

#### Scenario: withdraw
- GIVEN Proposed listing `p1` by `alice/web` with `mine` true and `p2` by `bob/web` with `mine` false
- WHEN the view is shown and `p1`'s Withdraw is tapped
- THEN `p2` shows no Withdraw, and the withdraw callback was called once, with `p1`

#### Scenario: refusals and empties
- GIVEN New proposal open with an empty title; Proposed listing `p1` with `mine` true and a withdraw callback that answers "Not withdrawn (500)."; and the view drawn before the implementation state arrives, and with an implementation state listing no proposal
- WHEN the proposal is submitted, and `p1`'s Withdraw is tapped twice, once after the refusal shows
- THEN the propose callback isn't called and the form says `Give it a title first.`; `p1` shows "Not withdrawn (500)." and the second tap calls withdraw again; and neither of the last two shows a Proposed row

#### Scenario: draft kept
- GIVEN New proposal open in `o/a`'s Backlog with the title "Export" typed
- WHEN the view is drawn again, then `o/b`'s Backlog is drawn, then the form is cancelled in `o/a`
- THEN the redraw shows the form open with "Export", `o/b`'s shows the New proposal button and no form, and after the cancel `o/a`'s shows no form

#### Serves
- capability/keep-a-backlog
- capability/follow-from-browser
- architecture/git-is-truth

#### Needs
- web.worker/proposals
- web.worker/snapshot-names-its-repo

### Requirement: comment-on-review <!-- base: 1279617c5ab0d49e -->
The PR view's Document tab SHALL carry a Comment control on the intent and on each requirement card, which opens a one-field form that calls its caller's comment with the target, null for the intent or the requirement's ID for a card, and the trimmed text, once per tap and not again while it runs; an empty text SHALL call nothing and say `Write something first.`. When the call answers that it went through the form SHALL close, and otherwise show under the form the reason it gives, or `Not sent: offline.` when it fails, keeping the form and its text. What was typed into an open Comment or Request changes form SHALL survive the redraws a refresh makes, until it is sent or cancelled, kept apart for each review its caller names, so another repository's branch of the same name has its own. It SHALL draw each entry of the review's `comments` under its target, the intent for a null target or for a requirement the review shows no card for, there marked `on <id>`, with the person part of its writer and its time, marked "on an earlier version" when `current` is false, and each entry of `change_requests` whose `held` is false under the intent the same way, marked "since approved" when `current` is true. A review without the two fields SHALL be read as having none. Comment SHALL be disabled offline and when the review has no fingerprint.

#### Scenario: comment on a requirement
- GIVEN `spec/x`'s review with fingerprint `f1`, which modifies `n/r`, a comment callback that answers that it went through, and then the review with a comment on `n/r` by `alice/web` at `f1`
- WHEN the reviewer taps Comment on `n/r`'s card, enters "Name the rule" and submits, and the view is drawn with that review
- THEN the callback was called once, with `n/r` and "Name the rule", and `n/r`'s card shows the comment by `alice`

#### Scenario: earlier version
- GIVEN a review with fingerprint `f2` whose `comments` holds one at `f1` with `current` false and target `null`, and whose `change_requests` holds one at `f2` with `current` true and `held` false
- WHEN the Document tab is shown
- THEN the comment is drawn under the intent, marked "on an earlier version", and the request under it, marked "since approved"

#### Scenario: no card for it
- GIVEN a review whose `comments` holds a current comment on `n/gone`, a requirement the review shows no card for
- WHEN the Document tab is shown
- THEN the comment is drawn under the intent, marked `on n/gone`

#### Scenario: refused
- GIVEN a comment callback that answers "Not sent: the Worker refused the text.", and, apart, one that fails
- WHEN a comment is submitted in each, and an empty one is submitted
- THEN the first shows that reason under the form and the second `Not sent: offline.`, each keeping the form and its text; and the empty one calls nothing and says `Write something first.`

#### Scenario: draft kept
- GIVEN `spec/x`'s review drawn for scope `/api/repos/o/a/branches/spec/x`, with "Name it" typed into the intent's Comment form and "Split" into Request changes' form
- WHEN the review is drawn again for that scope, then drawn for the scope `/api/repos/o/b/branches/spec/x`
- THEN the first redraw shows both forms open with their text, and the second shows neither form open

#### Serves
- capability/approve-intent
- capability/follow-from-browser
- architecture/humans-review-intent
- architecture/approval-policy

#### Needs
- web.worker.review/review-feedback

### Requirement: request-changes <!-- base: 8ce19ec86a611bff -->
The PR view SHALL offer Request changes in the Approve bar, beside Approve, which opens a one-field form that calls its caller's request with the trimmed text once per tap and not again while it runs, answering and keeping its draft as a comment's form does (`comment-on-review`). The header SHALL show each entry of `change_requests` whose `held` is true as changes requested by the person part of its writer, with its time and text, and say the branch won't land until the author pushes a change or the requester approves. Request changes SHALL be disabled offline and when the review has no fingerprint, and enabled in a signed repository, after an approval and after an earlier request.

#### Scenario: requested
- GIVEN `spec/x`'s review with fingerprint `f1` and `signed` true, online, a request callback that answers that it went through, and then the review with a held request by `alice/web`
- WHEN the reviewer taps Request changes, enters "Split it" and submits twice in quick succession, and the view is drawn with that review
- THEN the callback was called once, with "Split it", and the header says changes requested by `alice` with "Split it"

#### Scenario: disabled
- GIVEN a review with no fingerprint, and a review with a fingerprint shown offline
- WHEN each is shown
- THEN Request changes is disabled in both

#### Serves
- capability/approve-intent
- capability/follow-from-browser
- architecture/humans-review-intent
- architecture/approval-policy

#### Needs
- web.worker.review/review-feedback

### Requirement: capabilities-list <!-- base: fde4e68b1310910c -->
The Capabilities view SHALL list every capability of the Worker's report in its order, marking each with an unsettled flag ⚑, with a "⚑ only" filter that shows only those. It SHALL draw both a table, of the capability's slug and statement, its cost (`≈$`, work tokens and reviews), its unsettled workarounds, "Live without?" from its `overlap`, and its latest decision, and one card per capability with the same facts and a bar for its share of the costliest; which of them shows is `web`'s (`web/cost-views-in-the-browser`). Choosing a capability SHALL open its page. Dollars SHALL read to the cent under $10 and to the tenth from $10; tokens as a whole number under a thousand, then in thousands as `<n>k`, then in millions to the tenth as `<n>M`; a capability with no decision `never decided`; "Live without?" `all its rules only here`, or `<k> of <n> rules only here; also <id> (<k>)` for each entry it shares rules with; and the view SHALL note the unattributed and architecture-only costs. With no capability it SHALL say `No capabilities: CAPABILITIES.md holds none.`, and with none shown under "⚑ only" `No capability is flagged.`

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser

#### Needs
- web.worker/capabilities

#### Scenario: ranked and flagged
- GIVEN a report listing `b` ($9) then `a` ($2), with `a` flagged
- WHEN the view is shown
- THEN `b` comes before `a`, only `a` is marked ⚑, and with "⚑ only" on only `a` is listed

#### Scenario: figures and empties
- GIVEN a report whose `b` costs $9.00 and 900,000 work tokens, shares one of its two rules with `other`, and was never decided, whose `c` costs $12.34 and 1,300,000 tokens with all its rules its own, and whose unattributed cost is $1 and architecture-only cost $2; a report with no capability; and one whose capabilities none is flagged
- WHEN each is shown, the last with "⚑ only" on
- THEN `b` reads `≈$9.00`, `900k tok`, `1 of 2 rules only here; also other (1)` and `never decided`, `c` reads `≈$12.3` and `1.3M tok` and `all its rules only here`, and the note gives `≈$1.00` and `≈$2.00`; the second says `No capabilities: CAPABILITIES.md holds none.`; and the third `No capability is flagged.`

### Requirement: capability-page <!-- base: ebf732828d2e8d72 -->
A capability's page SHALL show, in this order: its entry in full, the report's `text` rendered as Markdown (`repo-markdown`), in place of its statement and why we need it; what it costs; its workarounds, each a `<details>` whose summary is its title and whose body is its text, can we live without it from its `overlap`, its rules collapsed, and its decisions, with a line saying to run `clm capabilities <slug>` for its commits, questions answered, blocking and size, which the Worker's report doesn't hold. The page of a slug the report doesn't hold SHALL say `The report holds no capability <slug>.`

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser

#### Needs
- web.worker/capabilities
- effort/entry-text

#### Scenario: sections
- GIVEN capability `x` whose `text` holds `` `clm land` `` and a `#### Scenario: s` block, with an unsettled flag titled "Judge ignores pipes" whose text is "Builds pipe output."
- WHEN its page is shown
- THEN the sections come in that order, the entry shows `clm land` as code and the scenario `s`, the workaround's summary reads "Judge ignores pipes" and its body "Builds pipe output.", and the page names `clm capabilities x`

#### Scenario: unknown slug
- GIVEN a report that holds no capability `nope`
- WHEN `nope`'s page is shown
- THEN it says `The report holds no capability nope.`

### Requirement: decide-buttons <!-- base: c1039d9e21ebaab8 -->
A capability's page SHALL offer a value (high, medium or low), the chosen one pressed, and Keep, Cut, Redesign and Accept as is. A tap SHALL post `{"version", "value", "decision"}` to the Worker's decide route with the capability's version as shown, through the `post` its caller gives, one request per tap and none while one is in flight, a page drawn again meanwhile included, and then load the report again through its caller: on 200 the page SHALL show the new decision, and on 409 say the capability changed since it was shown. On a 422 whose `error` isn't `signed` it SHALL say `Not decided (422): the Worker refused the <error>.` and load the report again; on any other failure say `Not decided (<status>).` and load nothing; when the post can't be sent, say `Offline: decide when back online.`; when the session has ended, call its caller's sign-out and say `Signed out. Sign in again to decide.`; and when the report can't be loaded again, add `Its report could not be loaded again: Reload.` The buttons SHALL be disabled offline, before a value is chosen and when the report's `signed` is true; when it is, a note SHALL give `clm capabilities decide <slug> --value high|medium|low --decision keep|cut|redesign|accept`, as it SHALL after a 422 whose `error` is `signed`, which gives the Worker's `command` in its place, the page then showing that one command only.

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
- GIVEN the page drawn at version `v`, and a Worker that answers the decision with 409
- WHEN a decision is tapped
- THEN the page says the capability changed since it was shown, and the report was asked for again

#### Scenario: disabled
- GIVEN a page offline, a page with no value chosen, and a page whose report has `signed` true
- WHEN each is shown
- THEN the decision buttons are disabled in all three, and the signed one's note gives `clm capabilities decide x --value high|medium|low --decision keep|cut|redesign|accept`

#### Scenario: refused
- GIVEN a Worker that answers a decision 422 with `error` `capability`; one that answers 502; a post that can't be sent; a session that has ended; and a Worker that answers 409 and then fails the report
- WHEN a decision is tapped in each
- THEN the first says `Not decided (422): the Worker refused the capability.` and the report was asked for again; the second says `Not decided (502).` and the report wasn't; the third says `Offline: decide when back online.`; the fourth called the sign-out once and says `Signed out. Sign in again to decide.`; and the fifth ends with `Its report could not be loaded again: Reload.`

#### Scenario: in flight
- GIVEN a page with value low chosen, drawn while a decision from it is in flight; and a signed report's page, after a 422 `signed` whose `command` is `clm capabilities decide x --value low --decision cut`
- WHEN each is shown, and in the first `medium` is tapped
- THEN the first's decision buttons are disabled, `low` is pressed, and the tap reports `medium` to its caller; and the second shows that command once and no other

### Requirement: architecture-list <!-- base: d644a9d4abaf2702 -->
The Architecture view SHALL list every entry of the Worker's architecture report in its order, marking each flagged one ⚑, with a "⚑ only" filter that shows only those, and a note that an entry's cost is its enforcing rules' share, counted apart from the capabilities'. It SHALL draw both a table, of the entry's slug and statement, its proof (`held <c>/<n>`, amber when `c` is less than `n`, ` + untested` after it when it also holds `#### Untested`, or `untested` alone; `held 0/0` for an entry with neither a rule nor `#### Untested`), its cost (`≈$`, work tokens and reviews), its unsettled workarounds, "Live without?" from its `overlap`, and its latest decision, and one card per entry with the same facts; which of them shows is `web`'s (`web/cost-views-in-the-browser`). While the report's `indexing` is true, or while a rule's state is unknown, the proof SHALL read `? / <n>`, and while it is indexing the note SHALL say coverage is still indexing. Choosing an entry SHALL open its page.

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser

#### Needs
- web.worker/architecture

#### Scenario: ranked, held and flagged
- GIVEN a report listing `e` ($5, flagged, enforced by two rules of which one is current) then `g` ($0, holding only `#### Untested`)
- WHEN the view is shown
- THEN `e` comes before `g`, `e` is marked ⚑ and shows `held 1/2` in amber, `g` shows `untested`, and with "⚑ only" on only `e` is listed

#### Scenario: indexing
- GIVEN a report whose `indexing` is true
- WHEN the view is shown
- THEN `e`'s proof reads `? / 2` and the note says coverage is still indexing

#### Scenario: unknown and empty proofs
- GIVEN a report not indexing, whose `e` has two rules, one with `current` null, and whose `z` has no rule and no `#### Untested`
- WHEN the view is shown
- THEN `e`'s proof reads `? / 2` and `z`'s `held 0/0`, not amber

### Requirement: architecture-page <!-- base: b9446399e80b2cce -->
An architectural constraint's page SHALL show, in this order: its entry in full, the report's `text` rendered as Markdown (`repo-markdown`), in place of its statement and its `#### Untested` text; whether it is held, each enforcing rule with its state (current in green, pending in amber, unknown while the report is indexing or while the rule's state isn't known), or `No rule enforces it.` for an entry with none; what it costs, saying it is counted apart from the capabilities'; its workarounds, each a `<details>` as on a capability's page; can we live without it from its `overlap`; and its decisions; then the decision buttons (`architecture-decide-buttons`).

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser

#### Needs
- web.worker/architecture
- effort/entry-text

#### Scenario: sections
- GIVEN entry `e`, whose `text` holds its `#### Enforced by` and `#### Untested` blocks, enforced by `a/x`, current, and `a/y`, pending, with the untested text "Partly judgement." and a flag titled "Gate misfires"
- WHEN its page is shown
- THEN the sections come in that order, the entry holds "Partly judgement.", `a/x` is drawn current and `a/y` pending, and the workaround's summary reads "Gate misfires"

#### Scenario: no rule
- GIVEN entry `u`, enforced by no rule
- WHEN its page is shown
- THEN it says `No rule enforces it.`

### Requirement: architecture-decide-buttons <!-- base: 41e20cf538be68f9 -->
An architectural constraint's page SHALL offer the value and decision buttons `decide-buttons` describes, posting `{"version", "value", "decision"}` to the Worker's architecture decide route with the entry's version as shown, one request per tap and none while one is in flight, then loading the architecture report again through its caller and answering each outcome as a capability's buttons do: on 200 the page SHALL show the new decision, and on 409 say the entry changed since it was shown. The buttons SHALL be disabled offline, before a value is chosen and when the report's `signed` is true; then, and after a 422 whose `error` is `signed`, a note SHALL give `clm architecture decide <slug> --value high|medium|low --decision keep|cut|redesign|accept`, or the Worker's `command`.

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

### Requirement: says-architectural-constraint <!-- base: c93b2f7f8b240a57 -->
The Architecture view SHALL call an entry of `ARCHITECTURE.md` an architectural constraint: its table's first column SHALL be headed `Constraint`. With no entries the Architecture view SHALL say `No architectural constraints: ARCHITECTURE.md holds none.`; with entries and none shown under "⚑ only", `No architectural constraint is flagged.`; and the page of a slug the report doesn't hold, `The report holds no architectural constraint <slug>.`

#### Scenario: an empty report
- GIVEN an architecture report with no entries
- WHEN the Architecture view is shown
- THEN it says `No architectural constraints: ARCHITECTURE.md holds none.`

#### Scenario: the list's heading
- GIVEN an architecture report with entries, none flagged, and no entry `nope`
- WHEN the Architecture view is shown, then with "⚑ only" on, and `nope`'s page is shown
- THEN the table's first column is headed `Constraint`, the filtered view says `No architectural constraint is flagged.`, and the page says `The report holds no architectural constraint nope.`

#### Serves
- architecture/plain-terms
- capability/follow-from-browser

### Requirement: says-component <!-- base: 88d3bff9568a4281 -->
The views SHALL say component and components where their text names one, never the old words `component-words` defines: the Implementation view's `Components` section, `Pending components` stat and `Nothing is pending: every component is up to its spec.`, the backlog's `Component` filter and its `<n> items across <m> components`, the PR view's `Components touched` and `No component changes.`, and the capability and architecture pages' and the Capabilities list's notes that a cost is split by the share of a component's rules.

#### Scenario: the Implementation view
- GIVEN a snapshot with two pending components
- WHEN the Implementation view renders
- THEN it shows the heading `Components` and the stat `Pending components`, and no shown text holds an old word

#### Scenario: the other views
- GIVEN a backlog of 3 items on 2 components, a review touching 2 components and one changing none, a capabilities report and an architecture report
- WHEN the Backlog, each review, the Capabilities list, a capability's page and a constraint's page are drawn
- THEN the Backlog shows the filter `Component` and `3 items across 2 components`, the reviews show `Components touched` and `No component changes.`, each cost note says a cost is split by the share of a component's rules, and no shown text holds an old word

#### Serves
- architecture/plain-terms

### Requirement: says-new-state-words <!-- base: 1dbae2a08aac704d -->
The app SHALL show a component's status as the `spec` model gives it and map no other word to `unbuilt`: the model writes `unbuilt` and reads the old word only from files (`renamed-words-still-load`), so it never hands the app the old word.

#### Scenario: an unbuilt component
- GIVEN a snapshot whose `spec` model gives component `b` the status `unbuilt`
- WHEN the inspector opens on `b`
- THEN its header names the status `unbuilt`

#### Serves
- architecture/plain-terms

### Requirement: zoom-at-pointer <!-- base: 7f4f9eca6ac548fe -->
Zooming the graph SHALL keep the point under the pointer where it is on screen: the wheel zooms around the pointer, a pinch around the midpoint between the two pointers, and the zoom buttons around the canvas's centre. The zoom SHALL stay between 0.3 and 3 times. Fit SHALL scale the graph to the canvas's width, never above 1 times, centred across it, and a graph given no pan and zoom SHALL open fitted. A drag on the canvas, off the cards and boxes, SHALL pan it.

#### Scenario: wheel over a card
- GIVEN the spec graph with node `b`'s card under the pointer
- WHEN the person zooms in with the wheel
- THEN the point of `b`'s card under the pointer is still under it, within a pixel

#### Scenario: buttons
- GIVEN the spec graph, with the point at the canvas's centre on node `a`'s card
- WHEN the person taps Zoom in
- THEN that point of `a`'s card is still at the canvas's centre

#### Scenario: fit and pan
- GIVEN a graph whose stage is twice as wide as its canvas, and one half as wide
- WHEN each is shown with no pan and zoom given, the first is zoomed in and out far past each end, then Fit is tapped, and the canvas is dragged 30 px right
- THEN the first opens at 0.5 times, its zoom stops at 3 and at 0.3 times, Fit brings it back to 0.5, and the drag moves it 30 px right; and the second opens at 1 times, centred across its canvas

#### Serves
- capability/follow-from-browser
- architecture/ui-quality

### Requirement: home-waiting-on-you <!-- base: 46bb33dba18c6f15 -->
The Home view SHALL list, in this order, each section left out when empty: **Review**, each spec branch whose review has `review` true and no approval in `approvals`, by name, linking to its review; **Answer**, each `question` item with `choices` and each `human-task` item, by title, linking to its page, with its answer buttons (`answer-buttons`); **Flagged**, each capability and architectural constraint whose report entry has `flagged` true, linking to its page; and **Answer in a session**, each other `question` item, linking to its page and saying it is answered in a `/clm:triage` session. Review counts only the reviews that have arrived. A Home with every section empty SHALL say nothing waits on the person. Its crumb SHALL name the repository as the snapshot names it, `<owner>/<repo>` (`web.worker/snapshot-names-its-repo`). The number of entries in Review and Answer SHALL be the count it gives its caller for Home's badge (`web/home-first`).

#### Scenario: what waits
- GIVEN repo `o/a` with spec branches `spec/x`, whose review needs review and has no approval, and `spec/y`, whose review needs review and has one; question `q` with choices `yes` and `no`; question `r` with none; `human-task` `h`; and a capabilities report flagging `c`
- WHEN Home is shown
- THEN Review lists `spec/x` only, Answer lists `q` and `h`, each with its buttons, Flagged lists `c`, Answer in a session lists `r`, and Home's badge is 3

#### Scenario: nothing waits
- GIVEN a repo with no spec branch needing review, no `question` or `human-task` item and nothing flagged
- WHEN Home is shown
- THEN it says nothing waits on the person and lists no section

#### Scenario: the crumb
- GIVEN a snapshot of `o/a`
- WHEN Home is shown
- THEN its crumb reads `o/a`

#### Serves
- capability/follow-from-browser

#### Needs
- replica/snapshot-backlog-text
- web.worker/snapshot-names-its-repo

### Requirement: answer-buttons <!-- base: c08a803139884b6f -->
A `question` item with `choices` SHALL offer one button per choice, in their order, and a `human-task` item one button, Done, which sends `done`; no other item SHALL offer any, a `question` with no `choices` and an item of another kind that has `choices` included. A tap SHALL call the item's answer callback once with the item's slug and the choice's value, and not again while it runs, and SHALL show under the buttons the reason the callback gives, or `Not sent: offline.` when it fails, keeping the buttons enabled. The button of an answer the person gave (an answer to the item whose `mine` is true) SHALL be drawn chosen, and no button for another person's answer. The buttons SHALL be disabled offline.

#### Scenario: answered
- GIVEN item `q` with choices `yes` and `no`, online, and an answer callback that answers that it went through, and then an implementation state listing the answer `yes` by `alice/web` with `mine` true, and `no` by `bob/web` with `mine` false
- WHEN `yes` is tapped twice in quick succession, and the buttons are drawn with that state
- THEN the callback was called once, with `q` and `yes`, and `yes` is drawn chosen and `no` isn't

#### Scenario: done
- GIVEN `human-task` item `h`
- WHEN its page is shown and Done is tapped
- THEN the answer callback was called once, with `h` and `done`

#### Scenario: offline
- GIVEN item `q` with choices, offline
- WHEN its page is shown
- THEN its buttons are disabled

#### Serves
- capability/follow-from-browser
- architecture/git-is-truth

#### Needs
- web.worker/answers

### Requirement: inspector-overlay <!-- base: ca741eb05ed89631 -->
The component inspector SHALL call its caller's close on its × button and on Escape while it is on the page, and not after it has left the page, and its backlog items SHALL link to their pages by the links its caller gives, drawn as plain text without them.

#### Scenario: an item from the inspector
- GIVEN component `a` with backlog item `dark-mode`, and item links of the form `#/o/a/backlog/<slug>`
- WHEN `a`'s inspector is shown
- THEN `dark-mode` links to `#/o/a/backlog/dark-mode`

#### Scenario: closing
- GIVEN `a`'s inspector on the page, and another inspector drawn and then taken off the page
- WHEN its × is tapped, then Escape is pressed
- THEN `a`'s close was called once for the × and once for Escape, and the other inspector's not at all

#### Serves
- capability/see-project-state

### Requirement: repo-markdown <!-- base: 53adc355eb110c11 -->
Every text the app shows from the repository that the Worker gives as Markdown (a component's contract, a change's intent, a requirement's text, a backlog item's body, and a capability's or constraint's statement and `text`) SHALL be rendered as Markdown and sanitised before it reaches the page, in the lists and on the pages alike: script elements, event-handler attributes and `javascript:` links SHALL be dropped.

#### Scenario: code in a statement
- GIVEN a capabilities report whose capability `x` has the statement ``Run `clm land` <img src=x onerror=alert(1)>``
- WHEN the list and `x`'s page are shown
- THEN each shows `clm land` in a `code` element, no backtick, and no `onerror` attribute

#### Scenario: what sanitising drops
- GIVEN the Markdown `**bold** <script>alert(1)</script> [x](javascript:alert(1)) <a href="https://e" onclick="alert(1)">e</a>`
- WHEN it is rendered
- THEN `bold` is in a `strong` element, and no `script` element, `onclick` attribute or `javascript:` link is left

#### Serves
- capability/follow-from-browser
- capability/see-project-state

### Requirement: worker-requests <!-- base: 936c9e889d169db8 -->
The views' requests to the Worker SHALL go through one module. A request answered 401 SHALL be sent once more, since a parallel request may have refreshed the session, and a second 401 SHALL report the person signed out; a `GET` that ends so SHALL also forget every answer kept for offline use. A successful `GET`'s answer SHALL be kept for offline use, unless its caller asks for it not to be, and no failed one. When the network fails, a `GET` that isn't a background refresh SHALL return the answer kept for that path with the time it was fetched, or report offline when none is kept. A `GET` answered with a 5xx other than a 503 whose body is `{"preparing": <share>}` SHALL do the same, returning the kept answer with its time when it isn't a background refresh and one is kept, and otherwise fail carrying the status; a 503 `{"preparing": <share>}` SHALL fail carrying the status and the share. A `GET` made with `fresh` SHALL append `fresh=<delivery>` to its query, after `&` when the path already has one, and keep its answer under the path without it. Any other `GET` the Worker refuses SHALL fail carrying the HTTP status. A `POST` or `DELETE` SHALL send its body as JSON with `content-type: application/json`.

#### Scenario: one 401, then answered
- GIVEN a Worker that answers a `GET` 401 and then 200
- WHEN the request is made
- THEN it was sent twice and returns the second answer, with no fetch time

#### Scenario: signed out
- GIVEN an answer kept for `/api/repos/o/a/snapshot`, and a Worker that answers every request 401
- WHEN a `GET` is made
- THEN it reports signed out, and no answer is kept

#### Scenario: offline
- GIVEN an answer for `/api/repos/o/a/snapshot` fetched at time `t`, and a network that then fails
- WHEN `/api/repos/o/a/snapshot` is asked, and, with nothing kept for it, `/api/repos/o/b/snapshot`
- THEN the first returns the kept answer with `t`, and the second reports offline

#### Scenario: offline copy
- GIVEN a notice with delivery `d1`
- WHEN the refresh it starts asks for `/api/repos/o/a/implementation`
- THEN the request goes to `/api/repos/o/a/implementation?fresh=d1`, and the copy kept for offline use is stored under `/api/repos/o/a/implementation`

#### Scenario: a body
- GIVEN the fingerprint `f1`
- WHEN it is posted to `/api/repos/o/a/branches/spec/x/approve`, and an endpoint is sent with `DELETE` to `/api/repos/o/a/push`
- THEN each request carries its method and its body as JSON, the post with `content-type: application/json`

#### Scenario: a failure
- GIVEN a Worker that answers a `GET` 404
- WHEN the request is made
- THEN it fails carrying the status 404, and nothing is kept

#### Scenario: busy Worker
- GIVEN an answer kept for `/p`, and a Worker that answers 502; then one that answers 503 `{"preparing": 0.4}`; and a path `/q?x=1` asked with `fresh` `d1`
- WHEN `/p` is asked, and asked as a background refresh, while the Worker answers 502, then asked while it answers 503, and `/q?x=1` is asked
- THEN the first returns the kept answer, the background refresh fails carrying 502, the third fails carrying 503 and the share 0.4, and the last goes to `/q?x=1&fresh=d1`

#### Serves
- capability/follow-from-browser

## ADDED Requirements

### Requirement: inspector
The component inspector SHALL show, for the selected component: its ID, with its status and `<current>/<total>` as the `spec` model gives them, or `indexing <n>%` while the snapshot's coverage is indexing; its title; its dependencies; its owners, as `Owned by <owners>`; each open spec branch whose review its caller has given that changes it, by name, with each element it changes and whether it is added, modified or removed; its contract as Markdown (`repo-markdown`); each change that touches it, with its status or `complete`, and its requirements current of total, or `coverage indexing` while the change's count isn't known; its work in flight (`inspector-pending`), reading `Loading work in flight…` until the implementation state arrives; and its backlog items, each by slug, kind and title. A section with nothing to show SHALL say so: `No open PR changes it.`, `Not in this snapshot.` for a component the snapshot's components lack, `No change touches it.` and `None.`

#### Scenario: what it shows
- GIVEN component `n`, `unbuilt` with 1 of 2 current, titled "N", depending on `a`, owned by `@x`, with contract `**Calls**`, changed by `spec/x`'s review (its Contract modified, its Design added, `n/r` added) and not by `spec/y`'s, touched by change `c1` (applied, 1 of 2 current) and change `c2` (count unknown), with backlog item `later`; and component `m`, which no review, change or item touches, absent from the snapshot's components
- WHEN `n`'s inspector is shown before the implementation state arrives, and `m`'s after it
- THEN `n`'s reads `unbuilt · 1/2`, "N", `a`, `Owned by @x`, `spec/x` with its Contract modified, its Design added and `n/r` added, and no `spec/y`, `Calls` in bold, `c1` with `1/2 requirements current` and `c2` with `coverage indexing`, `Loading work in flight…`, and `later`; and `m`'s says `No open PR changes it.`, `Not in this snapshot.`, `No change touches it.` and `None.`

#### Serves
- capability/see-project-state

#### Needs
- view/spec-view

### Requirement: hash-routes
The views' routes SHALL read as `#/<owner>/<repo>/<view>`, where the view is `graph`, `prs`, `build`, `backlog`, `capabilities` or `architecture`, and any other view, or none, reads as `graph`. Under `prs`, a branch given as one encoded segment SHALL name that branch's review when it starts `spec/`, followed by `graph` for its Graph tab or `full/<path>` for a file in full, the review's Document tab otherwise; any other branch reads as the PR list. `capabilities/<slug>`, `architecture/<slug>` and `backlog/<slug>` SHALL name a capability's, an architectural constraint's and a backlog item's page. A segment whose escape is malformed SHALL be read as written. The link the views make for a route SHALL read back as that route, and a review's Worker path SHALL be `branches/<branch>`, the name's slashes kept.

#### Scenario: routes
- GIVEN the routes `#/o/a/prs/spec%2Fx/full/specs/n/SPEC.md`, `#/o/a/prs/spec%2Fx/graph`, `#/o/a/nope`, `#/o/a/prs/main`, `#/o/a/backlog/later` and `#/o/a/capabilities/%E0%A4`
- WHEN each is read, and the links made for each route read are read back
- THEN the first names `o`, `a`, the review of `spec/x` and the file `specs/n/SPEC.md`, the second `spec/x`'s Graph tab, the third the graph, the fourth the PR list, the fifth `later`'s page, and the sixth the capability `%E0%A4`; each link reads back as its route; and `spec/x`'s Worker path is `branches/spec/x`

#### Serves
- capability/follow-from-browser
