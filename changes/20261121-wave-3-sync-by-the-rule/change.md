---
status: proposed
---

## Intent

Wave 3 of phase 5 of `backlog/a-passing-check-proves-less-than-the-spec-claims`, its second branch of three: `sync` and `sync.regenerate` are each brought, in one pass and one delta, to the rule for a requirement (`requirements-state-what-the-contract-shows`), with a one-phrase delta of `rename`'s Design, which named `sync.regenerate`'s `other-delta-merge` as a requirement where it is a decision. The wave is for a spec that is true and proven, not a smaller one. `20261121-wave-3-land-by-the-rule`, on `main`, held `gate`, `land`, `land.approval` and `host`; `20261121-wave-3-verify-by-the-rule` holds `verify` and `ci`; no clause moves between the three.

This branch lands after `20261121-wave-3-land-by-the-rule`, which has: `sync/build-branches` Needs `gate/evaluate-at`, `gate/own-files` and `gate/light`, the gate's answer that carries a branch's kind, `light` among them, and the component it reads from the files.

## Problem

Read against the rule and the code on 2026-11-21, by one reader per component, then by one drafter per component that checked each entry against the code, with a builder check, a correction round, a second-round check and an architecture audit:

- **Both Contracts were wrong or incomplete.** `sync`'s left out `RebaseArgs` and `AutosyncArgs`, which the root flattens into its parser; `sync.regenerate`'s left out `Tree::path` and `Tree::merged`, on which `sync`'s mirror relies, the refusals its calls return and where the lines of a take-back are returned.
- **Many clauses the code shows had no requirement**: what `clm rebase` does on a branch of no build name, with a rebase or `git am` begun by hand, with an archive place taken, with a held `index.lock` at each commit it makes; what `clm autosync` does with no terminal to prompt on; what `regenerated` is given and returns; and what both command calls do with `--help`.
- **Sentences of both Designs stated exits, and some contradicted the code**: `sync`'s said the `clm` crate prints a refusal's message and `clm-sync` prints nothing, where the crate's command call prints it, and `sync/shared-plumbing` said the root maps the error through one helper and chooses the outcome line from the exit code, where the crate's call writes it.
- **Bugs on `main`, each mended by its component's build, worst first:**
  - `sync.regenerate`: a merge into another change's delta writes over the working-tree file and commits it, so an uncommitted edit to that delta is lost without a word. An ID `HEAD` doesn't hold panics, and an `onto` that is no commit gives a tree with no `specs/` and no error.
  - `sync.regenerate` and `sync`: the trap the plan item records as "A trap met on the first closing branch". A `Merge x into o` commit, or a hand resolution of one, leaves `o`'s delta on a base only the branch's tree had; once a rebase drops that tree, or the branch takes back its own edit, `o` is stale and the report can only say to compare by hand. `clm rebase` merges other changes' deltas only when it regenerated a change, so a branch that no longer applies one never mends it.
  - `sync`: while a rebase someone else started is stopped, `clm rebase` says `HEAD` is detached and to check out a branch. With an archive place taken it archives and activates nothing at all. `clm autosync`'s command call ignores the `GIT_SSH_COMMAND` its lookup gives. Five places decide from the branch's name alone whether a branch is a build's and which component it has.
  - `sync`: `--help` through the crate's own command calls prints a code comment as its first line.
- **Two decisions no longer matched**, as the architecture audit found: `archive-on-sync` said `clm rebase` archives on any `impl/*` branch, and `autosync-policy`'s "a turn that changed nothing pushes nothing" read as if a clean branch were never pushed.

## Outcome

- Each of the two has one delta: Contract and Design made right against the code, requirements reworded to what someone outside the component observes, the unstated clauses stated with one scenario each, and the cuts below. `rename`'s Design names `sync.regenerate/stale-delta-merge` and the decision `other-delta-merge`.
- Where the code falls short of a requirement's plain meaning the requirement keeps its meaning, gains a scenario for the failing case, and its build mends the code.
- `sync/build-branches` decides which branches are builds' and their component from the gate's one answer for the branch against `origin/main`, read once per run, and `gate` joins `sync`'s `depends_on`.
- `sync.regenerate/stale-delta-merge` takes back, per element, a merge whose base no tree has before it merges again, finding the commit by the base the element records and never by its message, and judging each element when its change is reached in order. `sync/reapply` runs it whenever a commit since `origin/main` changes the `deltas/` of a live `proposed` or `approved` change, one the branch itself adds included, whether or not the run regenerated a change.
- The guard the deleted scan of `sync/git-through-layer` held, that `clm rebase`'s commits wait out a held `index.lock`, is kept as behaviour and proven at the commit itself: `sync/rebase` states the retry, and its scenario "lock met at every commit it makes" puts first on `PATH` a `git` that refuses the first `commit` it is given with a message naming `index.lock`, so each commit the run makes (`Archive`, `Activate`, `File`, `Re-apply`, `Merge`, `Take back` and `Rename`) is shown to be run again. `sync.regenerate/retry-index-lock`'s "merge commit waits out a lock" meets the lock at its `Merge` commit the same way.
- `agents-md-spec-path` merges into `sync/autosync`, which takes its scenario "uncommitted instructions" under its name and its `#### Serves`.
- The decision `other-delta-merge` gains the paragraph on taking a merge back and what stays a hand job; `archive-on-sync` and `autosync-policy` each gain a paragraph that meets what `sync` now states.
- `backlog/dead-decision-names-in-code-comments-and-skills` and `backlog/ownership-reads-that-bypass-owner` lose `sync`'s lines, which `sync`'s delta now covers. `backlog/a-live-change-and-an-archived-one-with-the-same-id-load-as-the-archived-one-alone` records a fault in `model` the pass met.

## Constraints

- A build changes only its own component's files (`own-files-at-the-gate`). No build of this branch needs a file outside its component.
- Build order is by `#### Needs`: `gate`'s build of `20261121-wave-3-land-by-the-rule` first, then `sync.regenerate`, then `sync`.
- `clm_sync::autosync(root)` keeps its signature and its reading of the process's environment, which `plugin.hooks` calls; only the command call honours its lookup. A hook that passes its lookup is wave 5's, expand then contract.
- The take-back's trigger covers a change the branch itself adds, so most drafting spec branches, whose commits touch their own change's `deltas/`, now run it on each `clm rebase`: one extraction of the tree per run, and on such a branch `main`'s own stale deltas of other changes are merged (`Merge main into <change>`) or stop the run with their report, as on a branch that regenerates.
- `crates/clm/src/main.rs` keeps its own `about` strings and the label of `clm rebase`'s settings line (`backlog/clm-rebase-labels-its-settings-failure-with-clm-health-s-rule`): both are the root's, in wave 7.
- `clm land`'s derived-specs check passes `merge = false` and reads its IDs from the checkout it gives, so neither new refusal of `regenerated` reaches it.
- Both components were over the spec budget before this change and `sync` grows, as "Settled on 2026-11-20" in the plan item expects of every wave. No seam is proposed here (`slim-components`).
- The names other components and the human layers use stay: every requirement of `sync` and `sync.regenerate` another component needs.

## What the requirements no longer state

Under the yes given in wave 1 (`requirements-state-what-the-contract-shows`) for the copied sentence and scenario of `git-through-layer`:
- `sync/git-through-layer`, whole, its one SHALL sentence being the copied one and an absence, with its scenario "covered by the scan". Its scenario "lock met at the commit" moves under its name to `sync/rebase`, where the `index.lock` retry is a clause of its own; the scan's check that commits go through the retried calls is kept as that behaviour (above), not cut.
- `sync.regenerate/git-through-layer`: its first sentence and its scenario "covered by the scan". The wrapper the sentence named is stated as behaviour in `retry-index-lock`, with the new scenario "merge commit waits out a lock", which meets the lock at the commit itself.

Under the yes given for the `shared-plumbing` scans of `host`, `land` and `sync` (wave 3's opening, after wave 2's yes for `ledger` and `init`):
- `sync/shared-plumbing`: its scan sentence and its scenario "no copy of the remote's name".

Sentences that only say where a test sits, cut with no separate yes (`cuts-are-what-a-program-lists`):
- `sync/commands-in-crate`: "Every test of `sync`'s requirements that sat in the top-level test files of `crates/clm/tests/` SHALL sit in `sync`'s own files…".

New proposals, each read by the human:
- `sync/shared-plumbing`, the rest of it, so that the requirement goes whole, as `ledger`'s and `init`'s did in wave 2: the sentence that `clm-sync`'s `Error` is `clm-git`'s `Failure` and its scenario "the shared error" (no caller reads the type); the sentence that the root maps the error through one helper `failed` and that `clm rebase` chooses its outcome line from the exit code, false today, and its scenario "one mapping in the dispatcher", a scan of the root's `main.rs`; and "`clm rebase` and `clm autosync` SHALL keep their exit codes, messages and outcome line", which `rebase`, `outcome-line` and `autosync` now state outright. Its scenario "refused sync keeps its output" moves under its name to `rebase`, with a detached-`HEAD` case. Its Needs of `vcs/remote-name` and `vcs/failure` go with it.
- `sync/extract-through-layer`: the scenario "no extract of its own", a scan for `fn extract(` and `--ignore-skip-worktree-bits`, and two of its three SHALL sentences, one naming calls the Contract doesn't offer and "Regenerating SHALL load one tree…", which restates `sync.regenerate/regenerated-tree`. What the scan stood for, no hook outside the two worktrees, stays stated and proven by the two hook scenarios.
- `sync/warnings-name-the-command`: the scenario "a skipped proposal's warning", which states what `clm_sync::sync`'s report holds, a call no Contract offers and nothing outside the crate calls. The prefix `clm rebase` prints stays proven through the command call.
- `sync/agents-md-spec-path`, whole, merged into `autosync`, its scenario moving under its name. Not counted by the gate.
- `sync`'s Design sentences that stated exits; requirements now state each.
- `sync.regenerate/git-through-layer`, the rest of it, so that the requirement goes whole: "No file under `crates/clm-regenerate/src` SHALL hold `fn extract(` or `--ignore-skip-worktree-bits`…", whose behaviour `regenerated-tree` states (with `merge` off nothing in `root` changes); "Its `Error` SHALL be `clm-git`'s `Failure`, and no file … SHALL hold a string literal that begins `"origin`", whose type half the Contract keeps and whose scan is not under the `shared-plumbing` yes, which named `host`, `land` and `sync`; and the sentence that no file under `crates/clm-sync/src` defines four functions or declares four modules, with its scenario "no copy left in clm-sync", a name scan of another crate's source that a renamed copy passes.

## Narrowings the human reads

- `sync/outcome-line`: the outcome line is printed for every run whose arguments it parses, where it said always: `--help` prints the help alone, and a usage error leaves the output stream empty.
- `sync/archive-complete`: it deletes only a tracked `backlog/<slug>.md` that `resolves` names, and a marker in an untracked file doesn't count toward completeness.
- `sync/autosync`: it pushes `HEAD` itself when the spec paths hold nothing uncommitted, where it said a commit whose parent is `HEAD`, which its own test and the Design contradicted; `AGENTS.md` and `CLAUDE.md` are the CLM root's, where `agents-md-spec-path` said the top of the repository's; and the prompts it avoids are those git and ssh give on a terminal, not a credential helper's own window.
- `sync.regenerate/regenerated-tree`: "with its stages" goes, since no Contract call shows the stages; `stale-delta-diff` states what they do.

Widened, each a judgement the human reads:
- `sync/build-branches`: what ran on `impl/*` alone runs on every branch the gate reads as code, light or mixed, whatever its name, and not on an `impl/*` branch that needs review.
- `sync/archive-complete`: with an archive place taken it archives the other complete changes and activates components, leaving only that change.
- `sync/reapply` and `sync.regenerate/stale-delta-merge`: the merge of other changes' deltas judges every live `proposed` or `approved` change, the branch's own unapplied ones included, and runs on a run that regenerated nothing whenever a commit since `origin/main` touched a live `proposed` or `approved` change's `deltas/`.

## Scenarios whose outcome changes under their name

- `sync/archive-complete` "node activated": it also prints `activated a`, which its test already asserts.
- `sync/rebase` "edit set aside during a conflict": its conflict moves from `src/lib.rs` to `docs/guide.md`, because a branch that commits code is now a build's and strict about uncommitted edits; the case it shows is unchanged.
- `sync/judged-by-check` "committed fault, uncommitted fix": its words stay, but its GIVEN moves the citing file from a commit on `spec/x` to the merge base and `origin/main`; the case it used to show, a `spec/x` branch that commits code, now prints `uncommitted src/lib.rs`, for the same reason as "edit set aside during a conflict". The gate doesn't list it.
- `sync.regenerate/regenerated-tree` "change built on another sorts first": it loses its outcome on the stages, which no call shows and no test asserted.
- `sync.regenerate/regenerated-tree` "human layers in the tree": reworded only, the graph `clm_model::load` reads and `clm-apply`'s `validate` becoming the tree loaded as `clm check` loads it and validating `y` against it, names the Contract doesn't offer; the outcome is the same.

## Open questions

None that holds the build. The human reads the new cuts above, the narrowings and the changed outcomes. Left by this branch: `backlog/the-root-keeps-a-test-of-sync-s-skipped-proposal-warning` and `backlog/clm-rebase-labels-its-settings-failure-with-clm-health-s-rule`, the root's, for after the wave and wave 7.
