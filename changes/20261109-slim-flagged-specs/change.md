---
status: applied
from: backlog/specs-restate-their-requirements-and-narrate-history
resolves: [specs-restate-their-requirements-and-narrate-history]
---

## Intent

Slim the second batch of the components `clm health` flagged on 2026-11-09: `capability` (design-heavy), `orchestrator.checkpoint` (history-words), and the outliers `plugin.panel`, `sync.regenerate`, `view`, `view.repo`, `web.client-live` and `web.worker.platform`, plus the `check.coverage` bug `pending-report-tests-prove-text-it-no-longer-states`. Each Design is cut down to its reasons where it restated its requirements, Contract text that repeats a requirement goes, requirements that duplicate another are folded in, scenarios another fully covers are dropped or merged, and requirements bundling several rules are split. Behaviour that only a Design or the code states is written into a requirement and kept, never dropped unseen. This finishes `backlog/specs-restate-their-requirements-and-narrate-history`: the first batch, `20261109-slim-restated-specs`, did `gate.fingerprint`, `orchestrator.checkpoint`, `plugin.hooks.guard` and `check.coverage`, and `clm health` no longer flags the item's other components (`ci`, `plugin`, `vcs`).

## Problem

A `spec-slimmer` pass per flagged component found the same causes as the first batch: Designs and Contracts restating their requirements almost word for word (`capability` has a 2820-token Design over no requirements; `plugin.panel`'s Design repeats `build-rows`' five-line rule verbatim; `view` defines the checkout line in four places), requirements that repeat another (`web.worker.platform/code-key-as-ledger` and `code-key-doc-only`; `view/view-builds-for-wasm` and `view-library`; `plugin.panel/manifest-owner` and `panel-package`), Design text that narrates one build's migration (`web.worker.platform`'s `tests/sign_in.rs` sentence, `capability`'s "It is built once `web/site-served-directly` is current", `view.repo`'s "a session whose checkout is behind…"), and one requirement carrying most of a component's scenarios (`plugin.panel/build-rows`, 18). Some behaviour is stated only in a Design or only in code: `web.client-live`'s push-subscription renewal, and `push-sw.ts`'s routing of an off-origin URL and its fallback when navigation fails.

## Approaches

- **A. Cut and merge only.** Remove restatements and narration, fold duplicates, drop covered scenarios, keep every requirement whole. Least rebuild, but `build-rows` keeps its `many-scenarios` flag and bundled rules keep churning together.
- **B. A, plus split the requirements that bundle several rules (chosen).** As the first batch did. A split moves text rather than removing it and re-cites those rules' tests, but each rule then changes, and churns, on its own. The human chose B.

Two further calls, the human's: requirement and scenario cuts are in scope, as in the first batch, not only Design and Contract text; and behaviour only a Design or the code states is written into a requirement and kept, not deleted. `view.repo/view-at-rev` keeps `clm view spec --at`'s size report, since the CLM panel's Spec tab runs `clm view spec --at origin/main` and shows its oversize marks from it.

Each split keeps the existing ID on the core rule and adds the carved-out parts under new IDs, so every reference from other components and their `#### Needs` stays valid.

## Outcome

**`capability`** (Contract and Design only; it holds no requirement):
- Design: Order shrinks to the one fact that ordering is the queue's, citing `check.coverage/pending-report` and `orchestrator.run/capability-waits-for-changed-rule`; Selection to the fact that the two npm packages aren't this component's dependencies, so `ci-check.sh` runs them itself; "A rule that changes what a test sees" to two sentences and its citations, without the `web/app-at-app-path` example; the `learn-clm` bullet loses "It is built once `web/site-served-directly` is current…"; "How many" goes. The web harness, panel harness, package mechanics and per-capability fixture paragraphs shrink to the boundary rules (phone-sized browser on the built app, GitHub faked and an unanswered request failing the test; the panel reached only through `plugin-panel/hooks/hooks.json`, running the real `clm` and `git`; no import of `web/`, `plugin-panel/` or `crates/`; fixtures in the package).
- Contract: drops the clause repeating `model/reserved-ids`.

**`orchestrator.checkpoint`**:
- Design: the line `clm health` flags reads "a build place with no branches reads as not yet claimed"; the Removals, Unsafe work and Review mark paragraphs keep their reasons and drop the sentences restating `checkpoint`, `checkpoint-reports-unsafe-work` and `checkpoint-review-mark`.
- `checkpoint-frees-places` drops "Neither line SHALL make the wrap-up incomplete", which `checkpoint-reports-unsafe-work` states. It keeps its exit-2 sentence: a sweep error is no git failure, so `checkpoint`'s rule doesn't cover it. Its scenarios stay; the first batch's scenario choices stand.

**`check.coverage`** (the bug):
- `pending-report` states again that an obsolete marker whose component is missing goes to the component `owner` gives its file, however the component went, which the code does and `a_node_gone_without_a_removal_delta_moves_too` proves.
- The bug's other half, re-citing `paths_match_as_git_glob_pathspecs_do` to `check/path-ownership`, stays filed: the test lies in `check`'s `paths`, and a `check.coverage` build may re-cite it only to its own requirement (`path-ownership`). The item is narrowed to that, on `check`, for a `clm claim check --fix`.

**`plugin.panel`**:
- `build-rows` splits into `build-rows` (which runs are a component's builds and what a row shows), `build-row-lifecycle` (waiting, done, failed with `Dismiss`, cleared and resumed) and `build-row-cost` (tokens, cost and the per-second tick). It drops "`/clm:view flight` in the chat SHALL answer with `clm`'s output alone" and the `chat unchanged` scenario, which `view-command-in-chat` states. Scenarios merge: `launcher prompt` with `quoted first step`; `finished build`, `failed build dismissed` and `cleared` into one for what is drawn and one for dismissing; `waiting ends` with `status lags the answer`. 13 scenarios remain across the three: `round`, `not a build` and `failed child of a finished build` each prove a rule no other does.
- "A press SHALL NOT rerun a `clm` command whose output the pane already holds for this generation" is stated once, in `views-panel`, and leaves `component-inspector`, `blocker-chains` and `inspector-detail`, with `press reruns nothing`.
- `manifest-owner` folds into `panel-package` as one clause.
- `component-inspector` merges `lines and model disagree` and `node gone` into `no inspector`, and folds `no waiting field` into `select a node`.
- Design: the Build rows, Inspector, Capabilities, panel intro and Type-check paragraphs keep only what the requirements don't say (the hook-event fields, the 200-run cap, reconciliation, why the tests type-check through the `typescript` API); the "In open PRs" exclusion goes.
- Kept as they are: `type-check` (`ci` names it), `component-heading` (`view`'s `#### Needs` name it) and `superseded-draw`.

**`sync.regenerate`**:
- Which bases count, and when a base is unknown, is stated once, in `stale-delta-diff`; `regenerated-tree` and `stale-delta-merge` name it instead of restating it.
- Scenarios merge: `regenerated-tree`'s "stale delta without merging" into "a change stale against main goes before the one built on it". `stale-delta-merge` writes in its own text that `other_deltas` stops on any refusal, which only the Design said.
- Design: Order, Stages, Merge, Other deltas and The tree keep the mechanics the requirements don't state (the `git diff --no-index`/`merge-file` invocations, the text trimming, the extract-and-move, why violations are filtered); the Tests paragraph keeps its first sentence; "The waits are fixed, with no override" and "should a third, it moves to `clm-git`" go. Contract bullets shrink to one line each.

**`view`**:
- The checkout line is defined once, in `status-report`; `view-command`, the Contract and the Design point at it.
- `view-builds-for-wasm` folds into `view-library`.
- `says-component` drops its test-mechanism sentence; `component-detail` drops the rationale for always writing its fields.
- Scenarios dropped: `width` in `view-command` (each line requirement asserts the width) and `blockers-view`'s `nothing blocked` (`blockers-root-lines`' scenario covers it). Merged: `checkout named` and `no origin` into `checkout behind`, renamed `checkout line`; `checkout current` into `checkout behind on the chart`; `spec lines at a commit` into `spec lines`, which the panel reads by position after the `--at` line; `nothing in flight` into `summary`; `budget accepted` into `oversized node`; `charts unchanged` and `nothing touches it` into `node detail`. `claims counted` stays: no other scenario checks the summary's `claims` line. 55 scenarios become 45.
- Design: Checkout, Models, Component detail, Oversized components and Charts lose their restatements; Size budget keeps one sentence naming the shared shell.

**`view.repo`**:
- `pending-names-its-commit` names `view-at-rev`'s first line and reading instead of restating them; `up to date` folds into `behind origin/main`. Its `no such commit` stays: it proves the exit of `clm pending`, a command `view-at-rev` doesn't cover.
- `view-at-rev`'s `detached, never fetched` folds into `header`. The size report stays.
- Design: the Pending paragraph's history and restatements go; At a commit keeps the in-memory calls, the budget and `.gitattributes` sources and where the `FETCH_HEAD` files are; the dependency sentence goes.

**`web.client-live`**:
- `notify-switch` splits: `notify-switch` keeps when the switch shows, its state and turning it on and off; new `push-renewal` states what the Design alone did: the app remembers the endpoint it posted per repo, moves every repo whose switch was on to the browser's new endpoint when the subscription changes, and unsubscribes the browser on turning a switch off only when no other repo's switch is on.
- `push-shown` adds what only `push-sw.ts` did: a URL on another origin opens the app's root instead, and a window that can't be navigated gets a new one opened.
- Scenarios: `notify-switch`'s two no-key cases become one.
- Design: The socket, Notifications and Push handlers lose their restatements; one copy of the import-cycle rule stays. `live-updates`' timings stay: they are what the tests observe.
- `notify-switch` also states what only the per-repo last-state cache (`states`) did: a switch drawn again shows its known state at once and keeps it when the ask fails (`ui-quality`), with a scenario. The build drops the shared-request map (`asking`), which no requirement needs.

**`web.worker.platform`** and **`web.worker`**:
- `code-key-as-ledger` folds into `code-key-doc-only`; `code-key-doc-only` drops "`keys.rs` SHALL hold no list of doc-only paths of its own" and the `no copy of the list` scenario, since the first scenario proves a change to `ledger`'s list reaches the key.
- `asset-responses` keeps the trait, the provided `asset` and the fake; the adapter's clauses (passing the request's method, path, query and headers, the 503 and 502 answers, caching per isolate only `/_clm/*`) move to a new `web.worker/asset-adapter`, the component that owns `worker.rs`. `asset-responses`' `a missing path` folds into `a binary asset`. `asset-adapter` writes in what only `worker.rs` did: `cookie` and `authorization` don't reach the binding, an unreadable body answers 502, and only a `GET` with no query is kept. Since `worker.rs` builds only for `wasm32` and no test reached those clauses, `web.worker`'s Design moves what the adapter decides (what reaches the binding, the 503 and 502, what it keeps) into code built for the host over a binding `worker.rs` supplies from `ASSETS` and tests fake; `worker.rs` keeps only the binding's calls.
- Design: the Assets paragraph keeps the HEAD/304/206 reason and "the platform answers no path specially"; the `path-ownership` migration sentences naming `tests/sign_in.rs` and `tests/push.rs` go; Fresh keeps its reason and the two accepted races; Object reads keeps one sentence; "One way down" stays in one place. The Contract's collaborator endpoint and cache times move to the Design.
- The build removes `fake.rs`'s `write_checkout`, `version_of`, `citing` and `compare_status`, which nothing calls, and any `fake` dependency that leaves unused (`no-dead-code`).

The builds afterwards delete the tests of removed requirements' and dropped scenarios' own that no rule keeps, and re-cite the rest at the new versions and IDs.

## Constraints

- No behaviour changes: every scenario dropped is covered by one kept, every rule removed is stated by another, and behaviour only a Design or code stated is written into a requirement, not dropped.
- No `ARCHITECTURE.md` or `CAPABILITIES.md` entry names a requirement this change removes, so neither changes.
- Left alone on purpose: `view.repo/shared-plumbing` and `sync.regenerate/git-through-layer`, the per-crate scans `remote-plumbing-scan`, `shared-plumbing-in-git` and `sync-regenerate-split` chose; `check.coverage/state-names` (the glossary test and three components rely on it), `plugin.panel/type-check` (`ci` names it), `plugin.panel/component-heading` (`view` needs it), the first batch's scenario choices in `orchestrator.checkpoint` and `check.coverage`, `web.client-live/live-updates`' timings, and `view.repo/view-at-rev`'s size report.
