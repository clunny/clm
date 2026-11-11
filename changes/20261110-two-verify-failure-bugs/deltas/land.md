---
component: land
base: { contract: "a3c6656493e8961b" }
---

## MODIFIED Contract
Landing a branch on `main`, on the machine that runs the command (`land-locally`). `clm land [<branch>]` runs the gate against `origin/main`, merges the branch in a scratch tree, runs the project's check command, handing it the tests the change can break as `clm affected` selects them (`CLM_SELECTION`, `graph-test-selection`), and `clm check` on the result unless the ledger says that code was already checked, records what it did in the ledger, and pushes the merge commit to `main` without force, retrying when `main` moved. It is the only thing in CLM that pushes `main`. What it keeps in a clone, the land tree, the shared target `.clm/land`, its lock, the check lock `.clm/check.lock`, the two locks' queues and failing checks' logs, is under the `.clm/` of the main worktree's CLM root, which `clm init` has the repository ignore, so a land leaves nothing untracked whatever the project's language (`land-places-ignored`). It prints `landed <sha> <kind>`, `<kind>` being `code`, `spec` or `spec review` as the gate's first line gives it, and exits 0; it exits 1 with the reason when it pushed nothing, and 2 on a git failure. Whether the branch is approved or held by a request for changes, and the signing of approvals and lands, are `land.approval`'s, as is `clm approve`, which `clm-land` re-exports. There is no after-merge review: a land that needs review lands only once approved (`yes-no-review`). `clm reconnect` pushes `main` and the local ledgers to an `origin` that `main` fast-forwards, as when a repository with `host = "none"` is moved onto a real remote or one landed with no remote by an older `clm` gets one. `clm land --check-only` runs the same check command, with every test (no `CLM_SELECTION`), and `clm check` on the current working tree, so a session learns the land check's verdict through a command that names no path (`land-check-command`). It lands nothing; when both pass on a clean working tree whose committed `HEAD` contains `origin/main`, it appends a `checked` record for `HEAD`'s code key, so a land whose merged tree is that tree, `main` not having moved, runs no check of its own (`worktree-check-counts`). The check command of a land, and of a `--check-only` run outside a build place, waits for any other such check of the same clone to end first (`check-runs-one-at-a-time`), so either can take as long as the checks queued before it; the waiting runs take their turns in the order they began to wait, except that a land carrying `P1` work goes first, a land whose code is already checked and runs only the tests that read what it changed goes next, and the rest go by priority (`lock-queue`, `ordered-lock-queue`); a `--check-only` run in a build place waits for none, its tests kept apart from other runs' by its slot (`tests-run-concurrently`). Both run it with `CLM_SLOT` set to the slot of the build place that names the branch checked, as `orchestrator`'s `place_slot` gives it, or 0, so the project's tests key their ports and paths apart from those of the tests other builds run at the same time (`build-slots`, `tests-run-concurrently`). While `origin/main`'s backlog holds a `kind: bug` item with `verify_failures`, a slow test the human took in as broken after the land, `clm land` refuses a code land that changes a file of one of the item's components, unless a commit of the branch resolves that item or another item holding the same component, so separate fixes for items that hold one component each land; it warns, and lands, a branch touching the components of a red `verified` result no item has taken in yet (`verify-stop`, `verify-after-land`). A host's run never holds a land by itself (`land-locally`). `clm-land` also offers the runner it runs `[land] check` through, under the clone's check lock, for `verify` to run `[verify] check` the same way (`shared-check-runner`), and the cap it applies to `.clm/land`, for `verify` to bound its own build place the same way (`shared-target-cap`). When a land runs the check command it times the command and each step the check marks, and keeps the times, with the files that selected each step, on the `landed` record it appends (`step-times`); `--check-only` times nothing. After the push of such a land it prints the check's time against `[land] check_target_seconds` and, over it, its costliest steps, the chain that selected each and how recent lands fared, so an agent can judge whether the selection reaches too far or a suite belongs after the land (`check-time-report`); it files nothing, and the report never changes the land's outcome.

## MODIFIED Requirements

### Requirement: verify-stop <!-- base: 493381b74666b14a -->
Before merging, `clm land` SHALL take the *held* components: the `components` of every `kind: bug` item on the fetched `origin/main`'s backlog whose `verify_failures` is not empty, each held by every such item that names it. A held component the branch's code changes a file under the `paths` of, as the gate judges code, is *fixed* when a commit the branch adds carries the trailer `Resolves: backlog/<slug>` for at least one item holding it, so that separate branches each fixing one of several items that hold a component each land, while the items not yet resolved go on holding it. When such a component is not fixed, `clm land` SHALL push nothing and exit 1 with rule `land/verify-stop`, naming each held component it touches and is not fixed with every item that holds it, and saying that the branch lands once a commit it adds carries the trailer `Resolves: backlog/<slug>` for one item holding each component named; a branch every held component of which it touches is fixed SHALL land as before. It SHALL also read the red `verified` results on `origin/main`'s first-parent chain after the newest passing one, the newest record for each commit (`ledger/verified-record`), and for each that no backlog item's `verify_failures` names and whose `components` the branch's code touches, print on stderr `warning: slow tests failed after <sha>, culprit <land|unknown>, on <component>...; run /clm:triage to take it in` and land as before. A spec land, and a code land touching no held component, SHALL land as before.

#### Scenario: held component refused
- GIVEN `origin/main` holding `backlog/b-broke.md` with `kind: bug`, `components: [b]` and `verify_failures` naming a commit, and a code branch changing a file under `b`'s `paths`
- WHEN `clm land` runs
- THEN it exits 1 with `land/verify-stop` naming `b` and `b-broke`, and `main` on `origin` is unchanged

#### Scenario: other components land
- GIVEN the same item, and a code branch changing only a file under `a`'s `paths`
- WHEN `clm land` runs
- THEN it lands

#### Scenario: the fix lands
- GIVEN the same item, and a branch changing `b`'s file whose commit carries `Resolves: backlog/b-broke` and deletes the item
- WHEN `clm land` runs
- THEN it lands

#### Scenario: two items hold one component
- GIVEN `origin/main` holding `backlog/b-broke.md` and `backlog/b-slow.md`, each with `kind: bug`, `components: [b]` and `verify_failures` naming a commit, and a code branch changing a file under `b`'s `paths` with no trailer
- WHEN `clm land` runs
- THEN it exits 1 with `land/verify-stop` naming `b` with both `b-broke` and `b-slow`, and saying one item holding it is enough

#### Scenario: each fix of two lands
- GIVEN the same two items, and a branch changing `b`'s file whose commit carries `Resolves: backlog/b-broke` and deletes that item only
- WHEN `clm land` runs
- THEN it lands, and a later code branch changing `b`'s file with no trailer is refused naming `b` and `b-slow`, while one whose commit carries `Resolves: backlog/b-slow` lands

#### Scenario: each held component needs its own fix
- GIVEN `origin/main` holding `backlog/b-broke.md` with `components: [b]` and `backlog/c-broke.md` with `components: [c]`, each `kind: bug` with `verify_failures`, and a branch changing a file of `b` and a file of `c` whose commit carries `Resolves: backlog/b-broke` only
- WHEN `clm land` runs
- THEN it exits 1 with `land/verify-stop` naming `c` and `c-broke`, and not `b`

#### Scenario: an untriaged red warns
- GIVEN no item with `verify_failures`, a red `verified` record for `origin/main`'s head naming component `b`, and a code branch changing `b`'s file
- WHEN `clm land` runs
- THEN it prints a warning naming the red commit and `b` and `/clm:triage`, and lands

#### Scenario: a red record alone holds nothing
- GIVEN a red `verified` record naming `b` written by a host's pipeline, and no backlog item naming it
- WHEN `clm land` runs on a branch changing `b`'s file
- THEN it lands
