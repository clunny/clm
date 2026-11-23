---
status: proposed
resolves: [a-worktree-the-finish-keeps-for-a-held-lock-is-told-to-no-one, the-build-loop-has-no-way-to-hand-its-runs-the-human-s-notes-for]
---

## Intent

Wave 4 of phase 5 of `backlog/a-passing-check-proves-less-than-the-spec-claims.md`, its branch B of two: `orchestrator.loop`, `orchestrator.checkpoint`, `view`, `view.repo` and `replica` are each brought, in one pass and one delta, to the rule for a requirement (`requirements-state-what-the-contract-shows`), with a small delta of `check.coverage` for the other side of a move into `view`. The wave is for a spec that is true and proven, not a smaller one. Branch A, `20261122-wave-4-orchestrator-by-the-rule`, holds `orchestrator`, `orchestrator.run` and `orchestrator.triage`; no clause moves between the two branches.

It also takes in two items the wave inherits. `orchestrator.loop` passes the `kept` lines of a finish to the session, as the human answered the kept-lines question (`a-worktree-the-finish-keeps-for-a-held-lock-is-told-to-no-one`) on 2026-11-22; and `clm build start --notes <file>` hands the human's notes to every run the loop launches (`notes-reach-every-launch`, for `the-build-loop-has-no-way-to-hand-its-runs-the-human-s-notes-for`).

## What the pass found

Read against the rule and the code by one reader per component, then by one drafter per component that checked each entry against the code, with a builder check and a correction round:

- **Contracts that didn't match the code.** `orchestrator.loop` offered no call that reads the loop file and says why it can't; `view`'s claimed a holder no model holds and that every model takes what `queue_from_refs` takes; `view.repo`'s left out the error stream its calls take and the budgets they read; `replica`'s Design named a rule for its files that lives in `web.worker.platform` and an archive it keeps that it doesn't.
- **Clauses the code shows with no requirement**: what `clm build` does when a command it runs fails, when `launched` names a run that has finished, when a take-over's held branch leaves `origin`; what `clm wrap-up` does with its own worktree, a main checkout that can't reach `main`, a build worktree gone from disk, a failed ledger audit; what `clm view` does with a commit it can't read, a removed unpruned worktree, a `--width` above 65535, a `main` it can't compare; which files the web replica fetches, and in which order (`replica/files-it-fetches`, new).
- **Bugs on `main`, each mended by its component's build:**
  - `orchestrator.loop`: a `kept` line of a finish reaches no one; a new draft refused at launch is reported with an empty name; a no-task `landed` return clears the run's resume flag before it refuses; a build whose escalation failed to land is launched again after a new session's `start`; the held orphan's wake line names no run.
  - `orchestrator.checkpoint`: a wrap-up that fails after deleting branches says nothing of what it deleted; `--help` through the crate prints a code comment as its first line; a comment says it exits 1 when it refused, which it never does.
  - `view`: an empty `components` section prints no `none`; a cycle of needs vanishes from the blockers view with everything waiting on it; the SVG passes characters XML forbids; the spec model writes the retired word `specd`.
  - `view.repo`: an unreadable loop file shows as `no loop has run on this clone`; `clm view spec --at` sizes a non-UTF-8 file differently from `clm view spec`; a help whose write fails exits 0.
  - `replica`: a record time with an offset reads as UTC, and trailing text is accepted; its snapshot copies the status word rule instead of taking `clm status`'s.
  - `check.coverage`: `clm status` writes `specd` for an unbuilt component.
- **Needs against `depends_on`**: `view`'s seven Needs on `plugin.panel` and `plugin.hooks`, and the loop's seven `view/queue-view` Needs on requirements that state nothing of the queue model, ordered nothing and go.
- **One decision no longer matched**: `remote-plumbing-scan` listed `view` among the crates that scan their source, which this change cuts.
- **Found and filed, outside the wave**: a draft's `left <slug>` lines never reach the loop's list (`backlog/a-draft-s-left-items-never-reach-the-loop-s-list`); rules held in several crates (`backlog/whether-a-propose-claim-is-done-is-decided-in-two-crates`, `backlog/clm-view-at-keeps-its-own-copies-of-clm-size-s-binary-probe-and-its-choice-of-files`, `backlog/the-rule-for-which-files-the-web-replica-fetches-is-held-three-times`, `backlog/the-proposal-slug-rule-is-copied-in-three-crates`, `backlog/rfc-3339-times-are-parsed-by-three-copies-of-one-function`); the replica's snapshot leaves out binary assets (`backlog/the-replica-s-snapshot-leaves-out-a-component-s-binary-assets`); and the `build` skill's sentence for `--notes` (`backlog/the-build-skill-has-no-sentence-for-the-human-s-notes`).

## Cuts in plain words

The gate lists 51 cut lines: 36 on the specs and 15 tests in `deleted-tests.txt`. None is under an earlier yes; each is *proposed*, and most lose nothing.

`orchestrator.loop` (9):
- `build-wait` "a stop request", `loop-start` "adopting a lapsed loop", `plan-snapshot` "no loop has run", `repeated-stops-escalate` "escalated", `stop-causes` "a full disk cleared" and `take-over-holds-moving-orphans` "a branch still moving" change their outcome by gaining cases the code already has (a drain with nothing running wakes at once, a returned run is dropped on `start`, `read` and `try_read` beside `stored_plan`, an escalation already on `origin/main` not pushed again, `cleared the-moon` exits 1, a held branch deleted on `origin` ends the hold). Lost: nothing.
- `plan-archive` "complete change": a typo (`gives,,`). Lost: nothing.
- `step-actions` "launched": its GIVEN and THEN grow: no fetch, and an unlisted or `returned` run is refused. The `returned` refusal is a change of behaviour, below.
- `runs-the-clm-now-at-its-path` states 2 sentences where it stated 3: the sentence that the path's resolution is a private function so a unit test proves it goes. Lost: a rule on how code and tests are laid out, no behaviour (the three scenarios keep it). Saves about 70 words. No test goes.

`orchestrator.checkpoint` (1):
- `wrap-up-fetch-retries-race` "a wrap-up whose fetch keeps failing": it now pins the line to standard error, starting `clm wrap-up: `, and the exit to 2. Lost: nothing.

`view` (9):
- `queue-view`'s scenario "wasm" is gone: it ran the same build as `view-library`'s "wasm build", whose statement covers the queue model. Lost: nothing not still stated and proven. Saves a scenario.
- `flight-view` "removed requirement still cited" loses "and `clm status`'s row for `d` has no `complete` cell", which is `check.coverage/status-report`'s own scenario. Lost: a second check of the same row.
- `blockers-root-lines` "root lines", `flight-view` "claimed node", `spec-view` "nested nodes" (`unbuilt` added), `queue-view` "chart sections" and "waiting on you highlighted", `view-command` "svg" and "at a commit": widened only. Lost: nothing.

`view.repo` (8):
- `view.repo/shared-plumbing` is gone, with its scan test (*proposed*, the session's choice). It forbade `"origin"`, `"origin/`, `remotes/origin`, `SystemTime` and `fn now(` in the text of `clm-view-repo`'s and `clm-view`'s `src/`. Lost: a program's guarantee about two crates' source text; `clm-view` is left with no scan. Kept: that the commands compare the checkout with `refs/remotes/origin/main`, stated by `reads-the-repository` and proven by a test that moves there. Saves a requirement, a test and the churn the scan causes on each refactor. With it the decision `shared-plumbing-in-git` is amended (*proposed*, read with the cut): `view` and `view.repo` hold no scan any more, that `REMOTE` and `now()` come from `clm-git` stays a principle the decision holds with no program to check it in those crates, and `vcs/one-git-layer` remains the only check of how a crate reaches git.
- `commands-in-crate` "a command called in a test" (states what its test asserts), "arguments it can't parse" ("test's" becomes "caller's") and "text from the crate below" (names the requirement, not the crate): reworded. Lost: nothing.
- `view-at-rev` "fetched from another worktree" (a removed, unpruned worktree counts) and "no such commit" (`--at=--all`), `view-queue` "no loop yet" (no git repository) and "the loop's plan" (the stderr line): gain cases. Lost: nothing.

`replica` (9):
- `architecture-report` and `capabilities-report` "the report clm gives": a function's name becomes the requirement's. Lost: nothing.
- `batched-recompute` "three pushes, one recompute" and "a claim lapses", `implementation-ledger` "checked page" (its THEN maps `impl/b` to 6, the lower open pull request, not 8), `snapshot-matches-clm` "fixture repo", `stores-in-chunks` "a large repository", `validate-writes` "a taken slug": gain cases. Lost: nothing.
- `host-neutral` "dependencies" reads the dependency tree's normal and build edges, where a test read `Cargo.toml`. Lost: the ban on `worker`, `clm-web-platform`, `clm-host` and `clm-git` as dev-dependencies of `clm-replica`, which would reach only its tests. Saves a scan of a file's text and the `toml` dev-dependency.

Tests deleted (15, `deleted-tests.txt`):
- `clm-coverage`'s `tests/status.rs::scenario_nothing_in_flight`: the scenario moved to `view/status-report`, whose own `scenario_nothing_in_flight` proves it, and the report's text form it reads goes. Lost: nothing.
- `clm-checkpoint`'s `a_refusal_is_on_the_error_stream_with_1` and `violations_are_on_the_output_stream_with_1`: they drive branches of the report no run reaches (`clm wrap-up` never refuses, and reports `main`'s violations as lines). Lost: checks of outcomes that can't happen. Saves the tests and the dead code behind them.
- Ten tests of `crates/clm-view/tests/view.rs`, each a twin of a `views.rs` test of the same name that asserts as much or more. Lost: these ten views read from files on disk rather than built in memory, which `model`, `check.coverage` and `view.repo`'s tests cover. Saves about 260 lines.
- `clm-view-repo`'s `shared.rs::scenario_no_copy_of_the_remotes_name_or_the_clock`, with `shared-plumbing`.
- `clm-replica`'s `host_neutral.rs::manifest_names_no_host_crate`, with the dev-dependency ban.

Cuts the gate doesn't count:
- `view/says-component`, whole, *under wave 1's yes* for the `says-component` scans: its message `<n> components pending` and its scenario "the flight summary" move to `flight-view` by name; `clm/retired-terms` still checks the old words. Its Needs and Serves go with it.
- Sentences that only place tests, *under the earlier yes* (`cuts-are-what-a-program-lists`): `orchestrator.loop/step-actions`' capability-test sentence, `orchestrator.checkpoint/commands-in-crate`'s, `view/view-library`'s, and `view.repo`'s in `commands-in-crate`, `reads-the-repository` and `arguments-and-exit`, with `view.repo`'s Design paragraph "Tests".
- *Proposed*: `replica/preparing`'s "and the architecture view SHALL say it is indexing" (`architecture-report` states it); `replica/implementation-ledger`'s "or `spec-approved` labels" (no input carries a label); and `replica`'s Design sentences that were false or a finished build step.
- The library's SVG of the `queue` view (*proposed*, `no-dead-code`): nothing asks for it, no requirement states it and the Contract's "It has no `svg` form" stands. No code goes: the arm is shared with `flight` and `blockers`.

## What the requirements no longer state

Under wave 1's yes (`requirements-state-what-the-contract-shows`):
- `view/says-component`, whole.

Under the earlier yes for sentences that place tests:
- `orchestrator.loop/step-actions`: "The build of this requirement updates the capability test `capability/build-from-spec` …".
- `orchestrator.checkpoint/commands-in-crate`, `view.repo/commands-in-crate`, `view.repo/reads-the-repository`, `view.repo/arguments-and-exit`: "Every test … that sat in the top-level test files of `crates/clm/tests/` SHALL sit in … own files …".
- `view/view-library`: "Every test of `view`'s requirements SHALL sit in `view`'s own files and call `clm-view` …".

New proposals, each read by the human:
- `orchestrator.loop/runs-the-clm-now-at-its-path`: "The resolution SHALL be a private function of the path and of which files exist, so a test proves it without replacing a running binary …".
- `view.repo/shared-plumbing`, whole: the remote's name from `clm-git`'s `REMOTE` and the time from its `now()`, the scan of two crates' `src/`, and "Text it renders for a person that names `origin` … SHALL keep its wording", which `view/status-report` states.
- `view/queue-view`'s scenario "wasm" and `view/flight-view`'s clause on `clm status`'s row.
- `replica/host-neutral` as it reached dev-dependencies; `replica/preparing`'s indexing clause; `replica/implementation-ledger`'s labels.
- `view`'s Contract sentences that were false ("who holds its claim and for how long"; every model "taking what `queue_from_refs` takes").

Moved, not cut: `check.coverage/status-report`'s "A section with no rows has the single row `none`." and its scenario "nothing in flight" go to `view/status-report`, which lays `clm status` out; `check.coverage`'s Design paragraph "Status text" goes, and a new Design paragraph, "Status", says the report carries no text form, since `view/status-report` writes it; so the build removes `impl Display for StatusReport`, which nothing prints (`no-dead-code`).

Reworded to the behaviour they guarded, no SHALL lost (named so the human sees them): `orchestrator.loop`'s `plan-snapshot` (only the loop's writer creates `<git common dir>/clm/` becomes: `stored_plan`, `read` and `try_read` create and write nothing), `launched-refuses-an-earlier-task`, `runs-the-clm-now-at-its-path`, `release` and `repeated-stops-escalate`; `orchestrator.checkpoint`'s `checkpoint-frees-places` (a scan of calls becomes: a place written while the sweep waits is judged as written), `checkpoint-reports-unsafe-work`, `wrap-up-fetch-retries-race`, `wrap-up-delete-loses-race` and `commands-in-crate`; `view.repo`'s five; `replica`'s five that named a function. Each delta's Design loses the sentences a requirement now states. Two drops of wording are named here: `orchestrator.loop/release` no longer says it renames the branch "with `git branch -m`" ("which then has the renamed branch checked out" says what it was for), and `orchestrator.checkpoint`'s Design loses "A branch merged outside `clm land` is removed like any other landed branch, and `clm ledger verify` reports it as `ledger/unrecorded-land`", which `checkpoint` and its scenario "unrecorded land" state.

## Narrowings

Where requirement and code disagreed and the requirement now takes the code:
- `orchestrator.loop/step-actions`: "Every `clm build` command SHALL renew the lease" becomes "every command that succeeds", and the step that prints `end` ends the lease so the next `start` takes the loop at once. A refused command proves nothing of the holder, and `launched-refuses-an-earlier-task` already said its refusal leaves the lease unchanged.
- `orchestrator.loop/repeated-stops-escalate`: "launching it no more" becomes "while its question item is on `origin/main` or waits in the loop file to land", since the item tells the human to delete it once answered.
- `orchestrator.loop/return-rules`: a return with `--cause running` and no `landed <sha>` line leaves the run running, counts no stop and finishes nothing, where it said "otherwise finish the run"; finishing would free the place and remove a worktree under a live task.
- `orchestrator.checkpoint/checkpoint`: it deletes only local branches no worktree it leaves has checked out, and removes or switches only a checkout with no uncommitted changes or untracked files that aren't ignored (the safe side; it said "no uncommitted changes").
- `replica/snapshot-matches-clm`: each component's **text** assets hash as a checkout's; binary assets wait for `backlog/the-replica-s-snapshot-leaves-out-a-component-s-binary-assets`.

Widened, each a judgement the human reads:
- `view.repo/arguments-and-exit` states the `--width` bound, 1 to 65535, where it said only that `0` is refused.
- `replica/implementation-ledger`: two open pull requests on one branch give the lower number's page; a nested `impl/…` branch makes a page.

Where the code falls short of what the requirement means, the requirement keeps its meaning, gains a scenario, and its build mends the code: `view/status-report`'s `none` under an empty section, `view/view-command`'s SVG escaping, `view.repo/view-at-rev`'s non-UTF-8 sizing, `replica/implementation-ledger`'s record times, and the loop's escalation after a new session's `start`.

## Changes of behaviour the human reads

- **The loop says the `kept` lines** (the human's answer of 2026-11-22): a `--returned` build finish's lines starting `kept ` reach the session in a `say`; a plain finish's (`clm build` finishing a build whose claim is gone) don't, since a later finish removes that worktree.
- **A draft's finish's `kept` lines too** (recommended and written): `clm draft --finish` never keeps a worktree for a held lock, so its `kept` lines are of a landed draft's worktree no later finish removes; the loop says them wherever it runs it. The other way is to leave drafts out of the `return-rules` sentence and its scenario's last case.
- **`clm build launched` refuses a run listed as `returned`** with exit 1, changing nothing, where it recorded it `running` and could hold its place for good; a `stopped` run is still recorded.
- **`clm build start --notes <file>`** writes the notes beside the loop file, and every prompt the loop writes until the loop ends carries them under "The human's notes for this loop" (`notes-reach-every-launch`); an unreadable file is refused before anything changes.
- **The loop's three narrowings above**: the lease renewed only by a command that succeeds and ended by `end`; an escalated run launched again once its item is gone; a `--cause running` return without a `landed` line left running.
- **`clm wrap-up` prints what it did before an error**: on any failure (git, the ledger's audit, the places sweep) it prints the `removed`, `left`, `freed` and `place` lines of what it had done, `origin` deletes of a partly failed push included, then `clm wrap-up: <why>` and exit 2, where it printed only the error. The loop reads no output of a wrap-up that exits 2, so those lines don't reach a loop's session; the next wrap-up reports what is still left.
- **A cycle of needs in the blockers view**: the cycle's components become one root, named by the lowest of their IDs, state `blocked`, holding up the rest of the cycle and everything waiting on it, where today they vanish. `clm view` never shows such a graph, but the web app does.
- **`unbuilt` replaces the retired `specd`** as an unbuilt component's status in `clm status`, the spec chart and model, and the replica's snapshot. Nothing in `web/` or the panel reads the value `specd`.
- **An unreadable loop file**: `clm view queue` says `loop file not read: <why>` on standard error and gives `loop file not read` as the reason, where it said `no loop has run on this clone`; with no git repository at all it still says the latter. The loop offers `try_read(root)` for it, an expand: `read` keeps its result.
- **`clm view` under `--at`** sizes a non-UTF-8 text file as `clm size` does, so `clm view spec` and `clm view spec --at` agree at the same commit.
- **What the web replica keeps of the change archive**, stated as it is: `replica`'s Design said the archive is kept as `changes/archive/<id>/change.md` only, but the replica keeps every file under `changes/`, the archive's deltas included, which the capabilities report and the coverage's moves read. `replica/files-it-fetches` now states it; nothing changes in what the replica does.
- **A record time with an offset** counts at the instant it names in the web replica, and a time with trailing text is left out.

## Order of the builds

Not a strict sequence. `#### Needs` hold single requirements, as `clm pending` shows after `clm apply`, and `clm next` offers a component once any of its pending requirements is free, so most start at once and pay a later build for the ones held:
- **Start at once:** `replica`, `view`, `orchestrator.loop`, `view.repo` and `orchestrator.checkpoint`.
- **Waits whole:** `check.coverage`, whose one pending requirement, `status-report`, Needs `replica/snapshot-matches-clm`: the snapshot takes its status word from `clm status`'s rows before that word changes from `specd` to `unbuilt`.
- **A later build for what is held:** `replica` 1 (`implementation-models` Needs `view/blockers-view`); `view` 1 (`status-report` Needs `check.coverage/status-report`, so it follows `replica`, then `check.coverage`); `orchestrator.loop` 5 (`plan-fold`, `plan-snapshot`, `repeated-stops-escalate`, `stop-causes` and `take-over-holds-moving-orphans` Need `view/queue-view`; six more that Need it stay held by the plan item); `view.repo` 2 (`view-at-rev` Needs `view/view-command`; `view-queue` Needs `orchestrator.loop/plan-snapshot`, which offers `try_read`, so it follows the loop's second build).

Across the A/B line, neither branch needs the other to land first. Branch A's `orchestrator` Needs `orchestrator.loop/build-wait`. This branch's requirements Need four that branch A modifies: the loop's `return-rules` Needs `orchestrator.run/run-finish` and `loop-start` Needs `orchestrator.triage/draft-resume`, and `replica`'s Need `orchestrator/queue-from-refs` and `orchestrator/taken-from-refs`. These Needs are unchanged from `main`, but once both branches have landed, each of those requirements waits for A's build of the one it Needs.

## Sessions told

- `plugin/skills/build/SKILL.md` gains a sentence for `--notes` once `notes-reach-every-launch` is built: `backlog/the-build-skill-has-no-sentence-for-the-human-s-notes`, `kind: deferred` on `plugin`.
- `docs/decisions/remote-plumbing-scan.md` records that `view` scans no longer, and `docs/decisions/shared-plumbing-in-git.md` that `view` and `view.repo` hold no scan while the rule stays a principle the decision holds.
- `backlog/help-through-a-crate-s-own-command-call-is-not-the-help-the-binary-prints` leaves `orchestrator.checkpoint`, whose help this change states, and keeps `config`.
- No edit to `AGENTS.md`, `docs/` guide pages or prompts.
