---
title: Land
status: active
owners: ["@clunny"]
depends_on: [ledger, host, gate, check, check.coverage, check.affected, config, vcs, model, apply, effort, land.approval, sync.regenerate, orchestrator, rename]
paths: ["crates/clm-land/**"]
successor: ""
serves: [capability/land-safely, architecture/land-locally]
---

## Contract
Landing a branch on `main`, on the machine that runs the command (`land-locally`). `clm land [<branch>]` lands the branch's head, the local branch when it contains `origin/<branch>` or `origin` doesn't hold the branch, else `origin/<branch>` when there is no local branch (`land.approval/branch-head`): it judges the head with the gate's library before anything else, runs the gate against `origin/main`, merges the head in a scratch tree, runs the project's check command, handing it the tests the change can break as `clm affected` selects them (`CLM_SELECTION`, `graph-test-selection`), and `clm check` on the result unless the ledger says that code was already checked, records what it did in the ledger, pushes the merge commit to `main` without force, retrying when `main` moved, and deletes the landed branch from `origin`. It is the only thing in CLM that pushes `main`. What it keeps in a clone, the land tree, the shared target `.clm/land`, its lock, the check lock `.clm/check.lock`, the push turn's lock, the three locks' queues and failing checks' logs, is under the `.clm/` of the main worktree's CLM root, which `clm init` has the repository ignore, so a land leaves nothing untracked whatever the project's language (`land-places-ignored`). It prints on standard output `landed <sha> <kind>`, `<kind>` being `code`, `spec`, `spec review` or `light` as the gate's first line on the merge commit gives it, then `removed <path>` when it freed a landed build's `target/`, and exits 0; it exits 1 with the reason when it pushed nothing, and 2 when git, the ledger or the settings fail, which after a push that went through leaves `main` holding a land whose records the next `clm land` of the branch writes (`land-push-retry`, `unrecorded-land-finished`). Its reason is written on standard error after `clm land: `, and each warning or note after `clm land: warning: ` (`commands-in-crate`). A branch's name is a claim its files must agree with, never a rule by itself: a branch named `impl/<component>` lands only as that component's work, and one named `spec/<id>` only as a spec branch (`build-holds-own-files`, `spec-branch-holds-spec-paths`). A `light` branch, scenarios added to one component's requirements together with the code that proves them (`gate/light`), needs no approval and lands on checks as a code branch does. Whether the branch is approved or held by a request for changes, and the signing of approvals and lands, are `land.approval`'s, as is `clm approve`. There is no after-merge review: a land that needs review lands only once approved (`yes-no-review`). `clm reconnect` pushes `main` and the local ledgers to an `origin` that `main` fast-forwards, as when a repository with `host = "none"` is moved onto a real remote or one landed with no remote by an older `clm` gets one. `clm land --check-only` runs the same check command, with every test (no `CLM_SELECTION`), and `clm check` on the current working tree, so a session learns the land check's verdict through a command that names no path (`land-check-command`). It lands nothing; when both pass on a clean working tree whose committed `HEAD` contains `origin/main`, it appends a `checked` record for `HEAD`'s code key, so a land whose merged tree is that tree, `main` not having moved, runs no check of its own (`worktree-check-counts`). The check command of a land, and of a `--check-only` run outside a build place, waits for any other such check of the same clone to end first (`check-runs-one-at-a-time`), so either can take as long as the checks queued before it; the waiting runs take their turns in the order they began to wait, except that a land carrying `P1` work goes first, a land whose code is already checked and runs only the tests that read what it changed goes next, and the rest go by priority (`lock-queue`, `ordered-lock-queue`); a `--check-only` run in a build place waits for none, its tests kept apart from other runs' by its slot (`tests-run-concurrently`). Both run it with `CLM_SLOT` set to the slot of the build place that names the branch checked (`orchestrator/place-slots`), or 0, so the project's tests key their ports and paths apart from those of the tests other builds run at the same time (`build-slots`, `tests-run-concurrently`). While `origin/main`'s backlog holds a `kind: bug` item with `verify_failures`, a slow test the human took in as broken after the land, `clm land` refuses a land whose code changes a file of one of the item's components, unless a commit of the branch resolves that item or another item holding the same component, so separate fixes for items that hold one component each land; it warns, and lands, a branch touching the components of a red `verified` result no item has taken in yet (`verify-stop`, `verify-after-land`). A host's run never holds a land by itself (`land-locally`). `clm land` and `clm land --check-only` refuse a branch of any name, a replayed rename aside, that loses a citing test no approved list names, a list being a change's `deleted-tests.txt` on the base the land lands on (`deleted-tests-approved`, `removal-needs-a-yes`). When a land runs the check command it times the command and each step the check marks, and keeps the times, with the files that selected each step, on the `landed` record it appends (`step-times`); `--check-only` times nothing. After the push of such a land it prints the check's time against `[land] check_target_seconds` and, over it, its costliest steps, the chain that selected each and how recent lands fared, so an agent can judge whether the selection reaches too far or a suite belongs after the land (`check-time-report`); it files nothing, and the report never changes the land's outcome. Every land takes the clone's push turn around its push, printing `waiting for another land's push: <n> ahead` on stderr when it waits (`push-turn`), and a land whose check passed runs again, when `main` has moved, only what its changes and `main`'s new commits can both break (`overlap-recheck`). Lands waiting for the land lock are landed together on one check by the one that takes it; each still gets its own merge commit and `landed` record and its `clm land` prints the same `landed` line, so not every commit of `main` is a tree a check ran on, though every commit a push left `main` at is (`batched-lands`, `failed-batch`). A `clm land` of a branch `main` already holds through a merge no `landed` record names finishes that land's records and merges nothing (`unrecorded-land-finished`). Any other `clm land` of a branch whose head `main` already holds, when it starts or as it comes to hold it while the land waits or checks, merges and appends nothing: when the branch's own merge on `main` has a `landed` record, it prints that record's `landed` line and exits 0, and otherwise it exits 1 saying the branch is already on `main` and naming the commit that brought it there (`already-on-main`).

`clm-land` offers, as library calls, `Env` being the environment as a lookup and `Output { lines, warnings }` what a command prints on standard output and as warnings on standard error, both `clm-approval`'s, re-exported:
- `land(root, branch, env)`: `clm land [<branch>]` in the process that calls it, returning its `Output` or an `Error`, `Refused(why)` when it pushed nothing, which the command exits 1 on, or `Git(why)` when git, the ledger or the settings failed, which it exits 2 on; `verify` lands what its mutation pass files through it (`verify/files-unproven`). `check_only(root, branch, env)` and `reconnect(root, env)` are `clm land --check-only` and `clm reconnect` the same way.
- `run_check_as(name, command, dir, selection, env, log)`: the runner `clm land` runs `[land] check` through, with `name` the command that runs it, which its warning names, `env` a `CheckEnv { set, unset }`, returning a `CheckRun { passed, code, tail, logged, step, seconds, steps }`, `step` a `Step { name, first, last }`, or an `Error` (`shared-check-runner`); `verify` runs `[verify] check` through it as `clm verify`.
- `run_check(command, dir, selection, env, log)`: `run_check_as` with the name `clm land`, the runner's form from before it took a name, offered only until `runner-names-its-caller` removes it, while `verify` moves off it.
- `cap_directory(dir, name, cap_gb)`: the cap `clm land` applies to `.clm/land`, returning nothing or a `Capped`, `Cleared(line)` or `Warning(line)`, whose `line()` is what to print (`shared-target-cap`); `verify` bounds its own build place with it.
- `land_command(args, root, env, out, err)` and `reconnect_command(args, root, env, out, err)`, each command as a call, with `LandArgs` and `ReconnectArgs`, the arguments `crates/clm`'s parser reads (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## Design
Crate `clm-land`, called from `crates/clm`'s `land` and `reconnect` subcommands and from `verify`. Every git call goes through `clm-git`, the remote's name taken from `REMOTE` (`git-plumbing-layer`), and every delete of a branch on `origin` through `host`'s leased delete.

**`clm land [<branch>]`**, default the current branch. Each attempt, up to ten:
1. Fetch `origin` (`main`, the branch, `clm/ledger/*`) and fold the ledger. Take the head, its fingerprint and change IDs from `land.approval` (`land.approval/branch-head`). Then, before the gate's process, the lock, the merge and the checks, *judge the head*: `clm-gate`'s `evaluate_at(root, <fetched base>, <head>, false)`, in-process, gives its kind, which the claim of a `spec/<id>` name is checked against (`spec-branch-holds-spec-paths`) and the deleted-tests check reads (**Deleted tests**, below). The claim of an `impl/<component>` name is checked as it always was, with `foreign_files` for that component (`build-holds-own-files`), so a build may still edit a capability test its component serves; whether a code branch of any name is one component's work is the gate's own block (`gate/own-files`), which the gate's process reports in step 2. The gate's process in step 2 still decides the land, since a test or a project may name another gate (`CLM_GATE`), and its first line on the merge commit gives the kind the land records. Lands taken together judge each land with the same call (**Lands taken together**), so the two paths can't drift. In a signed repository `land.approval`'s key check comes next, then its hold check, then whether the fingerprint is approved under the policy at the fetched base.
2. Run the gate's process on the head against `origin/main` in the tree this attempt checks in (**Places**, below), adding `--approved` when approved; for a head **Behind in `specs/`** (below), on the merge commit instead, once step 4 has made it. What follows its block is `land`'s and `approval-policy`'s.
3. `git merge-tree --write-tree origin/main <head>`. A conflict, or `specs/` changed on both sides, is `land`'s to judge, except for a head **Behind in `specs/`** (below), whose merged `specs/` the land regenerates.
4. The merge commit, `git commit-tree <merged tree> -p origin/main -p <head>`, message `Merge <branch> into main`, with `Change: <id>` on a following line for a land the gate reads as a spec land that adds or modifies live changes (the first by ID), and, as its last line, the trailer `Gate-Rules: <n>`, `n` being `clm-gate`'s `RULES`, the version of the gate's rules step 6 judges the land by (`gate-rules-trailer`, `audit-by-the-rules-in-force`), committed as the repository's git user. A light branch's change is archived in the branch, as a spec branch's design-only change is, so neither names a live change and neither merge commit has a `Change:` line. A head that already contains `main` is still merged with a merge commit, so a claim's tip reaches `main` only through its land (`merged-claims-released`). The commit is checked out detached into the tree, so the gate and the checks run on a commit. Then **Derived specs** (below).
5. The merged tree's code key (`ledger`). When a `checked` record names it with the same command, it asks **Readers of checked code** (below): when they select no file, only `clm check --no-detectors` runs; when they select some, `[land] check` runs as below with their selection and then `clm check --no-detectors`, and no second `checked` record is appended. When none names it and the CLM root's `clm.toml` differs between the fetched base and the merge commit only in what `clm-gate`'s `settings_unchanged` leaves out (`upgraded`, `declined_migrations`, comments and formatting), it looks up the *settings-free key* instead, `ledger`'s `code_key_with(root, <merge>, <prefix>clm.toml, <base>)`, in the same place the attempt computes the code key, before the lock (**Places**), so a land it counts as checked takes no lock unless its readers select a file: when a `checked` record names that with the same command, the land goes on as one whose code key is checked, as above, and once `clm check --no-detectors` (and the readers' check, when they selected a file) passes it appends a `checked` record for the merged tree's own code key, so the `landed` record names a checked key and a retry finds it (`settings-unchanged-is-checked`); otherwise **Selected tests** (below), `[land] check` through `sh -c` with `CI` unset (so a script that installs a browser only in CI doesn't), `CLM_LAND_DIR` set to `<home>/.clm/land` (`land-dir-env`), and `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL` left as the caller has them, since where a build's output goes is the project's check script's choice (`shared-land-target`), then `clm check`, and on success a `checked` record. `<main>` is the first worktree `git worktree list --porcelain` lists and `<home>` the CLM root in it, `<main>/<prefix>` with `<prefix>` what `git rev-parse --show-prefix` prints in the CLM root, computed once; a land from a build's worktree under `.claude/worktrees/` so shares one target (`build-places`). The gate then runs again on the merge commit, since a file both sides touched can change whether it needs review, and a light branch's requirements must still be current on the tree that lands (`gate/light`); in a signed repository `land.approval` signs the `landed` record.
6. `git push origin <merge>:refs/heads/main --force-with-lease=refs/heads/main:<the base step 1 fetched>`. A refusal isn't always `main` moving (`land-push-retry`), so it fetches `origin`'s `main` again, directly rather than through step 1's offline refusal, and judges it: the merge commit, or a commit that contains it, means the push went through; neither the merge commit nor the base means `main` moved and the next attempt starts, its `checked` record standing, so a retry with the same code key runs only `clm check --no-detectors`; still the base means a refusal for another reason, such as `[remote rejected] main (failure)` while other sessions push, and the same commit is pushed again with the same lease at most twice more, after fixed waits of 1 and then 2 seconds with no override, these pushes not counting as attempts. The push keeps the remote's lines (it drops `--quiet` if that hides them). When the refetch fails the push may have gone through; what the land then says is `land-push-retry`'s.
7. Append, in one commit (`ledger/append-all`), the `effort` records `clm effort record --branch <branch>` would append for the branch's sessions on this machine, read once before the first attempt so neither the reading nor a second ledger push lengthens the time `main` has moved with no `landed` record (`effort-accounted`, `records-effort`), then `landed`: commit, branch, `land_kind` and `review` from the gate's first line on the merge commit (a `light` line recorded as `land_kind` `code` with `light` true, so that a clone on an older release reads and audits it as the code land it is), fingerprint, the step-5 code key as `code_key`, the change IDs. `main` has moved by then, and what an append reported unpushed, or failing, then leads to is `land-push-retry`'s; the next `clm land` of the branch finishes a land whose records are missing (`unrecorded-land-finished`). Then `host`'s leased delete of the landed branch on `origin`, whatever its name, when `origin` held it, leased on `origin`'s tip as step 1 fetched it (a failure warns), the local branch too when it is checked out nowhere and the head contains it (`clm wrap-up` tidies one that is), and, for a land whose kind is `code` or `light`, **Freeing a build's disk** (below).
8. Print `landed <sha> <kind>`.

**Derived specs** (`derived-specs-fingerprint`). When the head adds a change (`clm-gate`'s `added_changes` over the merge base's and the head's `ls-tree -r -z` listings, the question `gate.fingerprint/fingerprint` asks), or applies one (a change whose `change.md` parses with status `proposed` or `approved` at the base, `origin/main`'s, and in the merge commit has status `applied`, or is absent while `changes/archive/<id>/change.md` has status `done`, as `sync`'s Design defines it), the IDs of the added changes whose `change.md` in the merge commit parses with status `applied` (live) or `done` (new archived, a light branch's among them), and of the applied ones, sorted, go to `sync.regenerate`'s `regenerated` (crate `clm-regenerate`, which reaches no `clm-health`: `clm/health-isolated`) without its stale-delta merge (`sync.regenerate/regenerated-tree`), run with the tree holding the merge commit as `root` and the fetched base's commit SHA as `onto`, so the scratch `specs/`, the copied `changes/` and `backlog/` and the re-apply are `clm rebase`'s own. `clm-land` calls no apply and no model load of its own for it. The regenerated `specs/` and the merge commit's are compared byte for byte, a symlink by its target, over the union of their paths in byte order, a path on one side only being a difference. It runs before step 5, so before the `--no-detectors` run too. The fingerprint skips `specs/` on a head that adds a change, so this is what keeps an approval from covering a hand edit there. A head that applies a change and adds none keeps `specs/` in its fingerprint, which reads listings alone and can't see a status, so a sync that regenerates its `specs/` onto a `main` that changed them asks for its approval again.

**Behind in `specs/`** (`land-regenerates-behind-specs`, `regenerate-dont-merge`). A head that adds or applies a change, as **Derived specs** reads them, is *behind in `specs/`* when a file under `specs/` changed both between the merge base and the base and between the merge base and the head, and every path `git merge-tree --write-tree --name-only origin/main <head>` reports conflicted, if any, lies under `specs/`. Concurrent spec lands make this the usual case: each regenerates the same `specs/<component>/SPEC.md` and the root `specs/SPEC.md`, and a land whose check runs for minutes can find `main` moved by another's when it pushes. Such a head lands without a `clm rebase` round, its next attempt regenerating again, in two regenerates. Whether a head is behind is decided where the attempt computes the merged tree; the code key and the lock decision use the tree merge-tree wrote, since the code key leaves `specs/` out, and both regenerates run in the attempt after that decision. The merge commit's message takes the `Change: <id>` line when the head adds or applies a live change. First the land proves the head's own `specs/` derived: `regenerated` from the tree on the head as `root`, onto the merge base, for the changes the head adds whose status there is `applied` or `done` and those it applies over the merge base, compared with the head's `specs/` as **Derived specs** compares, so a hand edit under `specs/` is caught here as there (`regenerates-behind-specs`). Then it regenerates as **Derived specs** does, onto the base, from a commit of the tree merge-tree wrote (whose `specs/` `regenerated` doesn't read, so conflict markers there don't matter), and the merge commit's tree is that tree with `specs/` replaced by the regenerated one. **Derived specs**' comparison then holds by construction and isn't run. Step 2's gate runs on that merge commit, which contains the base, so `gate/up-to-date` can't fire, and step 5's gate runs on it again as on any land. The approval stands: the fingerprint skips `specs/` on a head that adds a change, and on one that only applies a change it holds the head's `specs/`, which the first regenerate has proved derived. What lands under `specs/` differs from the head's only by what the base brought, regenerated as `clm rebase` would have made it.

**Places** (`scratch-outside-git`, `land-tree`, `fixed-land-tree`). Build tools such as Cargo name a workspace's outputs by its path and judge them fresh by file times, so two lands that overlap on one build place can check each other's code, and a fresh path per land rebuilt everything (43 GB a day on one machine). The land hands its check that place as `CLM_LAND_DIR` and the project's check script points its toolchain there (the starter script does for cargo: `init/starter-check-land-dir`); the land sets no toolchain's variables itself. Before step 2 an attempt computes the merged tree and its code key (`git merge-tree --write-tree` writes only objects); when it will run `[land] check`, it takes the exclusive advisory lock on `<home>/.clm/land.lock` (`land-target-lock`, `<home>/.clm/` created when missing, released by the system when a land is killed), waiting in the order **The queue** (below) sets, with no time limit, and holds it until the push has gone through and the cap is applied, or the attempt ends; the append, the deletes and the print run outside it. **The cap** (`land-target-cap`): right after a push that went through, still under the lock, it sums the lengths of the regular files under `.clm/land` (no symlink followed, so a sparse file counts its length) and, above `[land] target_cap_gb` gigabytes, removes the directory. Incremental data and units for feature sets no land builds again are what grow there, and Cargo prunes neither, so the cap trades one cold land for a bounded directory, and each attempt takes it afresh. A merge that conflicts and a checked code key whose readers select nothing take no land lock; a caller's `CARGO_TARGET_DIR` changes nothing here, since the land neither reads nor sets it. **The check lock** (`check-lock`): whatever the target, the run of `[land] check` itself, by a land or by a `--check-only` run whose `CLM_SLOT` (`check-slot`) is 0, holds a second exclusive advisory lock, on `<home>/.clm/check.lock`, from just before the command is spawned until it exits, taken with the same helper as the land lock (`lock.rs`, generalised to take the lock file's path and the waiting line), in the order **The queue** sets, with no time limit. The lock is taken inside the helper both callers share for running `[land] check`, before the output pipe is read, and dropped when it returns, on every path. The project's tests share the host, not only the target: a test that binds a fixed local port fails in whichever of two overlapping checks starts second, for a reason that isn't its branch's. A `--check-only` run in a build place, whose slot isn't 0, takes no lock: its slot keeps its tests apart from every other run's (`tests-run-concurrently`), and builds' pre-land checks, the clone's most frequent checks, queued behind every land's while it took one (`worktree-check-counts`); runs outside a place share slot 0, which keeps nothing apart, so they still take it. The helper is told whether to take the lock; `verify`'s runner (`shared-check-runner`) always takes it. A land takes it after `land.lock`, never before, and `--check-only` takes at most it, so neither can wait on the other in a cycle; `clm affected`, `clm check` and the gate run outside it. A locked attempt checks out in the land tree `<home>/.clm/land-tree` (checks of a root in a subdirectory run in `<home>/.clm/land-tree/<prefix>`): when `git worktree list --porcelain` lists it (both canonicalised) and it exists, `git checkout --quiet --force --detach <commit>` and then `git clean --quiet -fdx`; otherwise, or when either fails, it removes the path, runs `git worktree prune` and `git worktree add --quiet --detach <path> <commit>`. The tree stays when the attempt ends; `clm land` and `clm wrap-up` never remove it, and removing it by hand starts the next land cold. Any other attempt uses a fresh `land-*` worktree under `TMPDIR` (read through the injected environment lookup) or else `std::env::temp_dir()`, removed on every exit path, never one under `.claude/worktrees/`, and outside the repository, so no tool that refuses paths under `.git/` (Vite's `server.fs.deny`) or walks up to project files trips on it; a killed land's tree stays until the system clears it.

**The queue** (`lock-queue`, `ordered-lock-queue`). The operating system wakes the waiters of an advisory lock in no stated order, so `lock.rs`'s helper puts an ordered queue in front of each lock and keeps the lock itself for exclusion. Each lock `<home>/.clm/<name>.lock` has its queue in the directory `<home>/.clm/<name>.queue/`. A run that finds the lock free and no live ticket in the queue takes the lock at once, with no ticket. Otherwise it takes a ticket: under a short exclusive lock on `<name>.queue/next.lock`, it reads the counter in `<name>.queue/next` (0 when missing), writes it back one higher, creates the ticket file `<counter, zero-padded to 20 digits>-<lane>-p<n>` (`lane` being `priority` or `ordinary`, `p<n>` the run's priority, `p1` to `p5`), takes an exclusive advisory lock on that file, and only then releases `next.lock`. The ticket's own lock is what makes it live: the system drops it when the run's process dies, however it dies, and a run that reads the queue, under `next.lock`, opens each ticket for reading and removes each whose lock it can take, so a killed land never holds a turn and no process ID is read. A ticket the reading run can't open at all is left where it is and out of that reading's order: exclusion rests on the lock, so the cost of misjudging such a ticket is an order, never two holders, where taking it as live could hold every later run's turn for ever. **Turns.** Among the live tickets the turn goes, first, to the `p1` tickets, a `priority` one before an `ordinary` one and otherwise the oldest; then to the `priority` tickets, the highest priority first and then the oldest; then to the `ordinary` tickets in the same order; except that when the queue's `streak` file (in the same directory, written under `next.lock`) says the last six runs to take the lock below `p1` held `priority` tickets while an `ordinary` ticket was waiting, and no `p1` ticket waits, the `ordinary` ticket that would come first among them takes it. A waiter re-reads the queue at most every 250 milliseconds; only while the turn is its own does it try the lock, without blocking, and once it holds the lock it updates `streak` (unchanged when its ticket is `p1`, one higher when its ticket was `priority` and an `ordinary` ticket is waiting, 0 otherwise) and writes its ticket's name to the queue's `holder` file, both under `next.lock`, keeping the ticket and its lock until it releases the lock, when it removes the ticket. A run that takes the lock with no ticket writes `holder` empty and `streak` 0. A ticket is a file in the queue named by the pattern `<20 digits>-priority-p<1-5>` or `<20 digits>-ordinary-p<1-5>`; `next`, `next.lock`, `streak` and `holder` stay from run to run. Between a holder taking the lock and writing `holder` the waiting count can be one too high for a moment, which only the waiting line shows. **Lanes.** A land takes a `priority` ticket, for `land.lock` and for `check.lock` alike, when its merged tree's code key counts as checked (with the same command, or through the settings-free key) and **Readers of checked code** selected a test, so its check runs only those readers (a readers' selection that failed, so that every test runs, is `ordinary`); every other run, a land of unchecked code, a `--check-only` run with slot 0 and `verify`'s runner (`shared-check-runner`), takes an `ordinary` one. The lane is a parameter of `lock.rs`'s helper and of the crate's internal runner (`run_in`); the library call `run_check_as` (`shared-check-runner`) always waits with an `ordinary` ticket, so `verify` passes no lane. A land knows its lane only where it takes the lock for its readers, after **Readers of checked code** has run `clm affected --readers`; its earlier `land.lock` wait, for unchecked code, is always `ordinary`. A land that waited there and, on preparing again, finds its code checked, releases that lock and its ticket and queues afresh with the lane its readers give it, so a land waits with the lane of the check it will run. **Priority.** A land's priority is the one `orchestrator/land-priority` gives its branch, head and the fetched base, computed once per attempt before its first wait and kept when the land queues afresh within that attempt; every other run, a `--check-only` run and `verify`'s runner, waits at `p5`. A priority that can't be computed, such as for specs that don't load at the base, is `p5`, since the order is never worth failing a land for. Exclusion never rests on the queue: a run that skips it, such as an older `clm`, still waits on the lock itself, and only the order is lost.

**Its own binary** (`land-binary-copy`). A check that rebuilds `clm` can replace the file `clm land` runs from. So before its first subcommand it copies the binary it is running into a fresh `land-bin-*` directory beside its temporary worktrees, keeping permissions, and runs the gate, `clm affected` and `clm check` from the copy, removed on every exit path. The file `std::env::current_exe` names can be gone by then: that function reads the link `/proc/self/exe` on Linux, and once the file the process started from has been replaced or deleted (the dev install renames a newly built `clm` over it whenever `main` moves, and `clm verify` lands its unproven items in-process at the end of a run that can last hours, `verify/files-unproven`), the link reads `<path> (deleted)`. So it takes that path with a trailing ` (deleted)` cut off, and copies the file there when one exists, which after a dev-install rebuild is the newer `main`'s `clm`, so the land is still judged by `main`'s rule; only when no file is there does it copy `/proc/self/exe` itself, the running image, which the kernel keeps readable for as long as the process runs, judging the land by the rule of the `main` the process started from rather than refusing it. The copy is named after the cut path. A system with no `/proc` copies the path as it is, and a binary deleted under it can't be copied. The lander's binary is `main`'s rule, which the merged tree lives under. `CLM_GATE`, `CLM_CHECK` and `CLM_AFFECTED` override the three commands, run as given, so tests fake them; when all three are overridden nothing runs from the binary, so it makes no copy. That the land runs from its copy and leaves none behind needs the `clm` binary's own file, so the root states it (`clm/land-as-a-process`).

**Selected tests** (`graph-test-selection`). Before `[land] check` it runs `clm affected --base <sha>`, `<sha>` the fetched base's commit (the merge commit's merge base with it, so the selection is what the branch brings), in the tree on the merge commit and under the land lock when the check holds it (the check lock covers `[land] check` alone), and writes its stdout unchanged to `selection` in a temporary directory, which `CLM_SELECTION` names. When it fails the check runs without `CLM_SELECTION`: a failed selection costs time, never a skipped test.

**Readers of checked code** (`doc-readers-marked`). `code_key` leaves the doc-only paths out, so a `checked` record says nothing of the tests that read them: a spec branch's prose can fail such a test (a retired term under `specs/`) on a tree whose code was checked. A land whose code key is checked checks the merge commit out into a temporary worktree, as an attempt with no lock does, and runs `clm affected --base <sha> --readers` there (`check.affected/affected-readers`), `<sha>` as in **Selected tests**. When it prints nothing, the attempt goes on in that worktree with no lock. The gate on the head and **Derived specs** have run by then, in that worktree. When it prints a line, the attempt removes that worktree and goes on as one whose check runs: it takes the lock, checks the merge commit out into the land tree, so the check reuses the warm build place, and runs `[land] check`, so the readers' tests alone, with `CLM_SELECTION` naming a file holding that output unchanged. When it fails, it goes on the same way with `CLM_SELECTION` unset.

**The check's output** (`land-check-log`, `check-step-markers`). One pipe (`std::io::pipe`) carries `[land] check`'s stdout and stderr in the order written, copied to stderr and to `<home>/.clm/land-logs/<branch>.log`, the branch's slashes as directories, created and truncated first; the last 40 lines (split on `\n`) are kept in memory. It reads until the pipe closes, so a check must end what it starts. **Steps.** While reading, the runner numbers the output's lines from 1 and follows the steps the check marks in GitHub Actions' form, testing each line with a trailing `\r` removed: a line starting `::group::` starts a step named by the rest of the line, trimmed, at that line, ending any step open at the line before it; a line that is `::endgroup::` once trailing whitespace is trimmed ends the open step at that line; a line of either kind can sit anywhere in the output, a marker split across reads is matched once the line is whole, and a last line with no `\n` counts. The step open when the pipe closes, if any, is the *last step*, spanning its marker line to the output's last line; when every step had ended, or none was marked, there is none. Only the name, the first line and the line count are kept, so the steps cost no memory beyond the tail. A failing check's log stays and a passing check's goes (`check-log`). When the log can't be created the line numbers count the output copied to stderr. `clm check`'s output isn't logged, and `--check-only` names no step; `verify` prints the step and the tail after its outcome line (`verify/verify-run`).

**Step times** (`step-times`, `check-time-report`, `land-times-on-the-landed-record`). The runner that follows the check's steps also reads the clock, and `CheckRun` gains the timing beside its verdict, leaving `Step` as it is: a step's time runs from the read of its `::group::` line to the read of the line that ends it, or to the pipe's close for the last step, and the command's time from its start to its exit, each in whole seconds rounded down. Within a step, a line starting `::clm-selected::`, trailing whitespace trimmed, adds the rest of the line to the step's `selected_by`, each path once in the order first seen; such a line outside any step is ignored (`check-selector-lines`). A name may repeat, each step its own entry. Any attempt whose check command ran, a full selection or only the readers of checked code, puts the timing on the `landed` record it appends; a land that runs only `clm check` puts none. `--check-only` and `verify` use the same runner and record no timing. Once the check passes, and while the merged checkout and the land lock are still held, the land computes the chains it may print: for each of the up to three steps with the most seconds (ties in the check's order), `clm affected --why <path> --base <sha>` for the step's first `selected_by` path, run as it runs `clm affected`, against the same base. The report after step 8's append reads the last 19 timed lands from step 1's fold (`Fold::timed_lands(19)`). Whether a pattern of slow lands calls for a backlog item, a selection fix or a move to `[verify]` is the reader's judgement, which the guide's section `clm guide ci-and-deployment` informs.

**Freeing a build's disk** (`land-frees-build-target`). After the push and the append of a land whose kind is `code` or `light` it deletes the `target/` at `git rev-parse --show-toplevel` when that is a linked worktree whose `HEAD` is the landed branch; a missing one is skipped. The kind is the land's, not the branch's name, so a small change built in a worktree of its own frees its build's disk as a claimed build does, and a spec land, which builds nothing, keeps whatever is there. `clm run --finish` removes the worktree itself (`orchestrator.run`).

**`--check-only`** (`land-check-command`, `worktree-check-counts`). A branch given as well is judged by `check-only`'s own rule, not by clap's `conflicts_with`. On any branch, in the CLM root (`--root`, else found from the current directory), it judges `HEAD` against the local `origin/main` as a land judges its head (`build-holds-own-files`, `deleted-tests-approved`), then runs `[land] check` as step 5 does but with `CLM_LAND_DIR` removed (`land-dir-env`), so a build's check compiles where the caller's environment says, its own `target/` by default, then `clm check` with its detectors (`CLM_CHECK` overrides it), on the working tree as it stands, output on stderr. **The record.** Before the check command starts it reads `HEAD`'s commit (`git rev-parse HEAD`) and whether the working tree is clean: `git status --porcelain` in the CLM root's worktree prints nothing, so no tracked file differs from `HEAD`, staged or not, and there is no untracked file git doesn't ignore. After `clm check` passes it reads both again, and appends, with `ledger`'s `append`, a `checked` record for `code_key(root, HEAD)` only when the tree was clean both times, `HEAD` was the same commit both times and the local `origin/main` is an ancestor of `HEAD` (`git merge-base --is-ancestor`). An append reported unpushed is enough, since a land on this clone folds the local writer's branch. Its check command runs under `<home>/.clm/check.lock` (**The check lock**, above) only when its slot is 0, `<home>` found from the main worktree as a land finds it. **Why the record holds for the land** (`worktree-check-counts`). A land looks a `checked` record up by the merged tree's code key alone, an exact match, never by who wrote it or where (`legacy-code-key`). A land of a commit that contains the `main` it fetches makes a merge whose tree is that commit's tree, so its code key is the one recorded; when `main` has moved, the merged tree's code key differs whenever the code differs, and the land runs its usual selected check, the safety net. The record is made by the same `clm` helpers, on the same clone and machine and into the same ledger as a land's, from a clean committed tree that is the commit's own, with every test where a land would run only those `clm affected` selects, so it proves more than a land's record does; the build's full run stays the safeguard `graph-test-selection` counts on for a test the selection would skip. What differs is ignored state only: a build's own `target/`, where a land checks in a tree `git clean -fdx` has emptied of ignored files beside the warm `.clm/land`. A check that passes only because of an ignored file it doesn't make itself is the project check's own defect, which the next land after `main` moves, or `clm verify`, shows. It exists for a build's pre-land check (`orchestrator.run`), which Claude Code's worktree isolation refused as a script path.

**Offline** (`no-local-land-with-remote`, `none-keeps-its-own-remote`). Every host lands through `origin`: with `host = "none"` it is the clone's own remote (`host/none-keeps-a-remote`), so there is no local land and no `(local)` output. `main` has one copy, the remote's. Lands from linked worktrees of one clone share its remote-tracking refs, and one land's ledger push or branch deletion can move `refs/remotes/origin/...` between another's fetch reading a ref and updating it, which git refuses with `cannot lock ref ... is at <x> but expected <y>`. So every fetch of `clm land` and `clm reconnect` goes through `clm-git`'s retrying fetch (`vcs/fetch-retries`), which tries again with waits of 1 and then 2 seconds and then asks `origin` with `git ls-remote` whether it can be reached at all: what an unreachable `origin` and a fetch that keeps failing then print is `offline-land`'s. A land's fetch prunes: every landed branch is deleted from `origin`, and a tracking ref left behind would make `land.approval/branch-head` read a branch `origin` no longer holds as one the clone must push back. A truly offline land, or reconnect, spends the 3 seconds of waits before it says so. `clm approve`'s fetch goes through the same call (`land.approval/approve-retries-fetch`), so the three commands retry one way. **`clm reconnect`**, for a repository whose `origin` is newly set, fetches `origin` with `--quiet --prune`, so a remote-tracking ref the old remote left doesn't stand for the new one's branch, then pushes `main` with a lease when `origin/main` is missing or an ancestor, then every local `clm/ledger/*` branch; when `origin/main` has moved it pushes only the ledgers and names each local land's branch, read as the merge commit's second parent.

**The push turn** (`push-turn`, `a-passed-check-is-kept`). A third advisory lock, `<home>/.clm/push.lock`, `<home>/.clm/` created when missing since a project with no `[land] check` has none, taken with `lock.rs`'s helper and a queue of its own beside it, every ticket `ordinary` at `P5` so turns go in arrival order: `P1` goes first on the land lock, and is left out here because a turn lasts seconds. An attempt that runs no `[land] check` takes it before step 1's fetch and holds it through the push. An attempt that ran one takes it once the check has passed, still holding the land lock, and fetches `main` again inside it: unchanged, it pushes; moved to a commit that contains the head, it lands nothing, as **A head `main` already holds** (below) says; moved otherwise, it applies **A kept check**. The locks are always taken in the order land lock, push turn, check lock, so none waits on another in a cycle, and an attempt that holds the land lock never asks for it again. A land that waited for the land lock prepares again once it holds it on both of its paths, the early one for unchecked code and the late one for readers of checked code, so neither checks a merge made before the wait. A `clm` older than this rule takes no push turn, so its lands can still move `main` under a turn; the attempt then goes on as for a push from another clone.

**A kept check** (`overlap-recheck`, `a-passed-check-is-kept`). The attempt keeps the merge commit `P` its checks passed on. Inside the push turn, with `main` now at `M`, it runs `clm affected --overlap <P> <M>` in a temporary worktree (`P` holds the earlier `main`, so their merge base is that `main`), rebuilds its merge commit on `M`, runs the gate and `clm check` on it, runs `[land] check` with the overlap's selection in the land tree when the overlap printed a line, and pushes. A conflict while rebuilding, an `M` whose code key no `checked` record names, or an `--overlap` that fails ends the attempt as a moved `main` does, and the next attempt starts from step 1. Such rounds count toward the ten attempts. Lands taken together keep their check the same way (**A batch's kept check**, below).

**Lands taken together** (`batched-lands`). A ticket for the land lock holds its land's branch name as the file's content, written before the ticket is locked and before the waiting line prints, its name unchanged, so a `clm` without this rule still reads the queue's order and its own empty tickets are left to wait. The attempt that takes the lock reads the waiting tickets in `turn_order`, resolves each branch in the clone's own refs, and builds the chain: for each, what step 1 does before the lock (the judging of its head included, through the one call both paths share), the approval and gate checks of steps 1 and 2, `git merge-tree` onto the chain's tip, a merge commit whose first parent is the commit before it, and the gate on that commit. A land whose `specs/` changed on both sides is left to land alone, since only a land alone regenerates them (**Behind in `specs/`**). The tip's code key is computed from the trees before any commit is made, and every commit of a chain of more than one holds the line `Checked: <that key>` in its message, after `Change: <id>` when it has one. Each member's part of the selection is what step 5's paths give it alone, with `clm affected --overlap` for it and each other member; the check runs their union. One `checked` record for the tip's key is appended before the push, as now; after it, one `append_all` holds every member's `effort` and `landed` records, written and signed by the clone's one writer as the holder's own are, the holder reading each member's effort as `clm effort record --branch` does and appending before step 8 drops the lock; a holder that can't sign lands no batch. A waiting land whose head the commit it would merge onto already contains is left out of the chain, to wait for its own turn, as **A head `main` already holds** (below) says. A waiting land wakes when the holder's ticket is gone, prepares again, and finds its `landed` record in the fold.

**A failed batch** (`failed-batch`). The suspects are found from the failed run's output and the members' parts of the selection, with no further check. A member's part is the `file` lines of its selection, compared with the step's paths as paths; an `all` line or a selection that failed is every test. The failing run's `CheckRun` is returned to the caller, which judges it (`failed-batch`). A suspect left to land alone gets a file `<ticket>.alone` beside its ticket, which the next holder reads: it neither takes that ticket nor, when the ticket's own land takes the lock, takes others with it. The file goes with the ticket, so the mark lasts until that `clm land` ends. Halving keeps the first `ceil(n/2)` suspects in queue order.

**A batch's kept check** (`batch-keeps-its-check`, `a-passed-check-is-kept`). The holder acts on it when it finds `main` at an `M` other than its chain's base inside the push turn. `P` is the last of the commits it would have pushed, which holds the earlier `main`, so the merge base of `P` and `M` is that `main`, and the `checked` record for `P`'s code key, appended before the turn, stands as the `head` of the composed record's basis. It asks the overlap of `P` and `M`, then builds the chain again on `M` with the code that built it first, for the same lands in the same order, comparing each land's check variables with those the check ran under, and gives way when the new chain holds fewer lands, since the check ran on a tree holding all of them. When `main` moved again while the chain was built, the round asks about the commit it moved to, with the same `P`. Each new merge commit names the new last commit's code key in its `Checked:` line, computed from the trees before any commit is made, as for any chain. The overlap's run and `clm check` run in the land tree on the new last commit; the run takes the check lock in the holder's rank and keeps no log, as the chain's first run does. The composed `checked` record is appended before the push, so the key the `Checked:` lines name is checked by the time `main` holds them (`unrecorded-land-finished`). A push refused because `main` moved again starts another round with the commit last checked as `P`, as a land alone goes round again, and the first land's record sums the seconds of every run and lists their steps in order. To give way, the holder's attempt starts afresh under the lock it holds, taking no land with it, and the other lands' tickets wait unmarked.

**Finishing an unrecorded land** (`unrecorded-land-finished`). Step 1, before the gate, the lock and the merge, looks for a commit on `origin/main`'s first-parent chain whose second parent is the head. A head on that chain itself, or one whose merge a `landed` record names, is not this rule's but **A head `main` already holds**'s (below). With no `landed` record for the commit, it is covered by its own code key's `checked` record, or by the `checked` record for the key its `Checked:` line names when that key is the code key of the last commit of the run of first-parent commits holding the same line; the other commits of that run are the push's other lands, their branches read from their `Merge <branch> into main` subjects. A forged `Checked:` line proves nothing, since the run's last commit must carry its own checked key. The fingerprint is recomputed from the commit's parents as `ledger`'s verify recomputes it, the gate runs in a temporary worktree on the commit with its first parent as base, and the records are signed by the clone's writer and the `effort` read as for any land.

**A head `main` already holds** (`already-on-main`). A merge of a head that `origin/main` already contains is empty, and a second `landed` record for it counts the same work twice (it also reads as a land of the kind the empty diff gives, `spec` for a code branch). So every reading of `origin/main` that a merge is built or pushed on asks it, `git merge-base --is-ancestor <head> <main>`, whatever the land waited for before it: step 1's, on the first attempt and on every preparing again (after the land lock's wait as after a moved `main`), and the push turn's fetch of `main`, which follows the check lock's wait and the check. The answer goes three ways. A commit of the first-parent chain that merges the head as its second parent, named by a `landed` record of the branch (`batch.rs`' `landed_record`, until now asked only after a wait for the land lock): the land does what a waiting land taken together does once the holder landed it (`finish_found`), deleting the branch as a land does and printing that record's `landed <commit> <kind>`, and exits 0. Such a commit that no `landed` record names: **Finishing an unrecorded land**. Any other head `main` holds, one on the first-parent chain itself or one another branch's land brought there: it exits 1 saying the branch is already on `main` and naming the oldest commit of the first-parent chain that contains the head. A land that finds the head on `main` in its push turn first releases the push turn and the land lock it holds, since finishing an unrecorded land takes the land lock; the `checked` record its check appended stands. In a batch the holder asks it of each waiting land against the commit that land would merge onto, and leaves out one whose head that commit contains.

**Approval at a land** (`approval-policy`, `approval-from-base`, `changes-requested-holds`, `signed-land`). `clm-approval` answers whether the fingerprint's approvals count and why each doesn't, under which policy, whether a request for changes holds the branch, and whether the repository signs (`land.approval`); what `clm land` then does is stated here and proven by landing a branch on a fixture repository, since `clm-approval` can land nothing.

**Deleted tests** (`deleted-tests-approved`, `cuts-are-what-a-program-lists`). The check runs before the lock and before the gate's process, and judges every branch, whatever its name: a small change can delete a test as a build can, and a spec branch can delete a test that sits under a spec path. The one branch it leaves alone is a replayed rename, whose rewritten markers `rename/replay` vouches for: `clm-land` asks `clm-rename`'s `replays` itself, of a head that adds exactly one change setting `renames`, as the gate asks it, so a replay whose markers all sit under spec paths, such as in `docs/`, is recognised as well as one that rewrites code files. A branch that reads `spec` because its only other change is a `clm.toml` the gate counts as a spec change is judged, since its approval edit vouches for no test. `--check-only` makes the same judgement of `HEAD` and reads no branch name. It reads, at the merge base and at the head, only the files under the CLM root that `check.coverage` says can hold a test (`check.coverage/test-holding-paths`), the rest holding none on either side, and neither tree when the diff touches no such file, since every test is then where it was; this gives the verdict `clm coverage --deleted-tests` gives reading every file of the two trees. A file the diff doesn't touch holds the same tests on both sides and so makes no test gone, but its tests' names decide whether a name in a touched file is new, which `deleted_tests` asks before it counts a gone test as renamed, so it is read whenever any such file is touched: reading only the touched files let a test renamed to a name an untouched file's test carries pass the land while `clm coverage` listed it. The lists are read from the fetched base with `git ls-tree` and `git show`, live changes and archived ones, never from the branch.

## Requirements

### Requirement: land
`clm land [<branch>]` SHALL fetch `origin`, judge the branch's head against `origin/main` as `build-holds-own-files`, `spec-branch-holds-spec-paths` and `deleted-tests-approved` say, run the gate on the head against `origin/main` (on the merge commit instead for a head `regenerates-behind-specs` covers) with `--approved` when the ledger holds an `approved` record for the head's fingerprint that the `approval` policy accepts, merge the head into `origin/main` in a scratch tree, run `[land] check` and then `clm check` on the merged tree unless a `checked` record names its code key with the same command (then `clm check --no-detectors`, preceded by `[land] check` only when the readers `selected-tests` asks for select a file), run the gate on the merged tree, make a merge commit of the head into `origin/main`, push it to `main` without force and with a lease on the fetched `origin/main`, append `landed` naming the merged tree's code key, the land's kind and whether the land needed review, and print `landed <sha> <kind>`, `<kind>` being the gate's first line on the merged tree, `code`, `spec`, `spec review` or `light`, recorded as `land_kind` `code`, `spec` with `review` false, `spec` with `review` true, or `code` with `light` true, `review` false (`ledger/landed-record-fields`). Its fetch of `origin` at the start of each attempt, and of each batch of lands taken together, SHALL prune the remote-tracking refs of branches `origin` no longer holds, so a branch another land deleted from `origin` reads as one `origin` doesn't hold. The head SHALL be the one `land.approval/branch-head` gives for a land, whatever the branch's name: the local branch when `origin`'s tip of it is the local branch or an ancestor of it, or when `origin` doesn't hold the branch; `origin`'s tip when there is no local branch; it SHALL exit 1, pushing and appending nothing, when `origin` holds the branch and its tip is neither the local branch nor an ancestor of it, printing the advice `land.approval/branch-head` gives, for a local branch that is an ancestor of `origin`'s tip and for one diverged from it; when neither the clone nor `origin` holds the branch, naming it, and when no branch is named and `HEAD` is detached, saying to name the branch. When the push is refused because `main` moved it SHALL warn `main moved during attempt <n>; trying again`, `<n>` the attempt's number, and start again from the fetch, up to ten attempts; a move it finds by fetching `main` in its push turn, before it pushes (`push-turn`, `overlap-recheck`), starts the next attempt with no such warning. It SHALL push nothing and exit 1, naming why, when the gate blocks; when the merge conflicts, or the head and `origin/main` have both changed a file under `specs/` since their merge base, outside what `regenerates-behind-specs` regenerates, naming `clm rebase`; when a check fails; and when every attempt is refused, saying `main` keeps moving; and it SHALL append `checked` whenever the check command and `clm check` pass on a tree, even when the push is later refused.


#### Scenario: code branch lands
- GIVEN `origin/main`, a branch `impl/a` with a passing code change, no `checked` record, and fake gate and check commands that record the tree they ran on
- WHEN `clm land impl/a` runs
- THEN `main` on `origin` is a merge commit whose parents are the old `main` and the head, the check command ran on that commit's tree, the ledger has `checked` for its code key and `landed` for the commit with `land_kind` `code` and that code key, and it printed `landed <sha> code`

#### Scenario: already checked code
- GIVEN a branch that changes only `specs/x/SPEC.md`, which no `clm:reads` marker names, and a `checked` record for `origin/main`'s code key with the configured command
- WHEN `clm land` runs
- THEN the check command never ran, `clm check --no-detectors` ran on the merged tree, and the branch landed

#### Scenario: unapproved branch that needs review
- GIVEN a `spec/x` branch whose gate prints `spec review` and no `approved` record for its fingerprint
- WHEN `clm land spec/x` runs
- THEN it exits 1 printing `gate/approval` and naming `clm approve spec/x`, and `main` on `origin` is unchanged

#### Scenario: approved branch that needs review
- GIVEN the same branch and an `approved` record for its fingerprint
- WHEN `clm land spec/x` runs
- THEN it prints `landed <sha> spec review`, and the `landed` record has `land_kind` `spec`, `review` true and `changes` `[x]`

#### Scenario: stale approval
- GIVEN an `approved` record for fingerprint `f`, and a branch whose fingerprint is now `g` after a content change
- WHEN `clm land` runs
- THEN it exits 1 printing `gate/approval`

#### Scenario: main moves
- GIVEN a check command that, the first time it runs, pushes another commit to `origin/main`
- WHEN `clm land` runs
- THEN it starts again, `main` on `origin` ends as a merge commit whose parents are that commit and the head, and the check command's last run was on that merge's tree

#### Scenario: conflict
- GIVEN a branch that conflicts with `origin/main` in a file outside `specs/`
- WHEN `clm land` runs
- THEN it exits 1 naming `clm rebase`, the check command never ran, and `main` on `origin` is unchanged

#### Scenario: check fails
- GIVEN a check command that fails
- WHEN `clm land` runs
- THEN it exits 1, `main` on `origin` is unchanged, and the ledger has no `checked` or `landed` record for the attempt

#### Scenario: from main
- GIVEN the checkout on `main`
- WHEN `clm land` runs with no argument
- THEN it exits 1 saying `main` can't be landed

#### Scenario: a light land
- GIVEN a branch `impl/a` with a passing change and no `checked` record, no `approved` record, and a fake gate that prints `light` on the head and on the merge commit
- WHEN `clm land impl/a` runs
- THEN the check command ran on the merge commit's tree, it printed `landed <sha> light`, the `landed` record has `land_kind` `code`, `light` true, `review` false and no `changes`, and the merge commit's message has no `Change:` line

#### Scenario: the head it lands
- GIVEN in turn: `origin/impl/a` one commit ahead of the local `impl/a`, and in turn no local `impl/a`; `fix-x` pushed to `origin` with one more local commit; `fix-z` pushed and then diverged locally by an amended commit; `fix-y`, a branch with a passing change that `origin` doesn't hold; a local branch `gone` with a passing change, pushed to `origin` and since deleted there by another clone; no branch `nope` anywhere; and a checkout whose `HEAD` is detached
- WHEN `clm land impl/a` in each, `clm land fix-x`, `clm land fix-z`, `clm land fix-y`, `clm land gone`, `clm land nope` and, in the detached checkout, `clm land` run
- THEN the first exits 1 naming `impl/a` and saying it is behind `origin/impl/a`, and with no local `impl/a` lands `origin`'s tip; the second lands with the local tip as `main`'s second parent; the third exits 1 naming `fix-z` and saying it has diverged, with `main` on `origin` unchanged; the fourth and the fifth land the local tip; the sixth exits 1 naming `nope`; and the last exits 1 saying to name the branch; and no refusal appends a record

#### Scenario: specs changed on both sides
- GIVEN a branch `impl/a` that adds no change and changes a line of `specs/x/SPEC.md`, and an `origin/main` that has since changed another line of that file, so git merges the two without a conflict
- WHEN `clm land impl/a` runs
- THEN it exits 1 naming `clm rebase`, the check command never ran, and `main` on `origin` is unchanged

#### Scenario: main moved, and said
- GIVEN a passing code branch `impl/a`, and a script named `git` first on the `PATH` the land sees that, for the first `push` naming `refs/heads/main`, first pushes another commit onto `origin`'s `main` and then runs the real push, and runs the real `git` for every other call
- WHEN `clm land impl/a` runs
- THEN it lands on that commit, and its standard error holds `clm land: warning: main moved during attempt 1; trying again` once

#### Needs
- clm/renamed-commands
- land.approval/branch-head
- ledger/landed-record-fields
- gate/light

### Requirement: landed-branch-cleanup
After a successful push, `clm land` SHALL delete the landed branch on `origin`, whatever its name, when `origin` held the branch at its last fetch, through `host`'s leased delete (`host/branch-delete-call`), leased on `origin`'s tip of the branch as that fetch read it, which is the head it landed or an ancestor of it (`land.approval/branch-head`), and SHALL warn without failing, ``kept `<branch>` on origin: <why>``, `<why>` the reason that call refuses with, when it refuses: for a branch that moved on `origin` meanwhile, one `origin` rejects, an `origin` that can't be reached, and `main` or a branch under `clm/`, which it never deletes. For a branch `origin` doesn't hold it SHALL delete nothing on `origin`. It SHALL also delete the local branch when no worktree has it checked out and the head it landed contains the local branch's tip, warning without failing when that delete fails, and SHALL keep the local branch otherwise.

#### Scenario: impl branch removed
- GIVEN a landed `impl/a` whose tip on `origin` is the head it landed
- WHEN `clm land impl/a` finishes
- THEN `origin` has no `impl/a` and it exits 0

#### Scenario: moved head kept
- GIVEN a check command that pushes a new commit to `origin/impl/a` while the land runs
- WHEN `clm land impl/a` finishes
- THEN `origin/impl/a` is still that commit, it printed a warning, and it exits 0

#### Scenario: any branch removed
- GIVEN `fix-x`, a branch with a passing change pushed to `origin`; `fix-w`, pushed to `origin` with one more local commit; and `fix-y`, one `origin` doesn't hold
- WHEN `clm land fix-x`, `clm land fix-w` and `clm land fix-y` run
- THEN `origin` has no `fix-x` once the first exits 0, the second lands its local tip and `origin` then has no `fix-w`, and the third lands with `origin` holding no `fix-y` and no warning about it

#### Scenario: a delete origin refuses
- GIVEN a passing branch `fix-x` pushed to `origin`, and a `pre-receive` hook on `origin` that rejects every delete of a branch printing `remote says no` and accepts every other push
- WHEN `clm land fix-x` runs
- THEN it lands and exits 0, `origin` still has `fix-x`, and its standard error holds ``clm land: warning: kept `fix-x` on origin: `` with `remote says no`

#### Scenario: the local branch
- GIVEN a pushed `impl/a` checked out in no worktree, and a pushed `impl/b` checked out in a linked worktree
- WHEN `clm land impl/a` and then `clm land impl/b` run from the main worktree
- THEN both land, the clone has no local `impl/a`, and it still has `impl/b`

#### Needs
- host/branch-delete-call
- land.approval/branch-head

### Requirement: offline-land
`clm land` SHALL land through `origin` on every host, `none` included, whose `origin` is the clone's own remote (`host/none-keeps-a-remote`), and SHALL never land on the local `main` alone. When `origin` can't be reached, `clm land` SHALL exit 1 saying it is offline and to land when `origin` is reachable, land nothing locally and append nothing. When `origin` answers but holds no `main`, `clm land` SHALL exit 1 naming `origin/main` and `clm reconnect`, landing and appending nothing. A fetch of `origin` by `clm land` or `clm reconnect` that fails SHALL be tried again, at most twice more, waiting 1 second before the first of these and 2 seconds before the second, so that a land whose fetch lost a race with another land of the same clone updating a remote-tracking ref (git's `cannot lock ref`) goes on as if the first fetch had succeeded; when the last try fails it SHALL say it is offline only when `origin` can't be reached either, and otherwise exit 1 with git's message from the last fetch and no `offline`, landing nothing and appending nothing. `clm reconnect` SHALL fetch `origin`, pruning the remote-tracking refs of branches `origin` no longer has, then push `main` when `origin/main` doesn't exist or is an ancestor of it, then every local ledger branch, and print `reconnected <n> lands`; when `origin/main` has moved it SHALL push only the ledgers, print `land again <branch>` for each local land's branch, and exit 1. `clm reconnect` SHALL exit 1, pushing nothing, when the clone has no `origin` remote, printing ``no `origin`: add the remote, then reconnect``, and when the clone has no local `main`, naming it.

#### Scenario: no host lands through the clone's own remote
- GIVEN a repository with `host = "none"` whose `origin` is a bare repository `keep_remote` made, holding `main`, and a passing code branch `impl/a` pushed there
- WHEN `clm land impl/a` runs
- THEN `main` on `origin` is a merge commit of the branch, the ledger branch on `origin` has `landed`, and it printed `landed <sha> code` with no `(local)`

#### Scenario: offline with a remote refuses
- GIVEN a clone with `host = "github"` whose `origin` URL points nowhere, and a passing code branch
- WHEN `clm land` runs
- THEN it exits 1 saying it is offline, the local `main` is unchanged, and the ledger has no new record

#### Scenario: first push after adding a remote
- GIVEN the repository of *no host lands through the clone's own remote* after its land, its `origin` then set with `git remote set-url` to an empty bare repository, the remote-tracking refs of the old one still in the clone
- WHEN `clm reconnect` runs
- THEN `origin/main` is the local `main`, the ledger branch is on `origin`, and it printed `reconnected 1 lands`

#### Scenario: origin moved
- GIVEN the same, but `origin` already has a `main` that isn't an ancestor of the local one
- WHEN `clm reconnect` runs
- THEN `origin/main` is unchanged, the ledger branch is on `origin`, it printed `land again` with the branch, and it exited 1

#### Scenario: a fetch that loses a ref race is tried again
- GIVEN a passing code branch `impl/a`, and a script named `git` first on the `PATH` the land and the programs it starts see that, for the first `fetch` naming `origin`, prints `error: cannot lock ref 'refs/remotes/origin/main'` on stderr and exits 1 without running the real `git`, and runs the real `git` for every other call
- WHEN `clm land impl/a` runs
- THEN it exits 0 printing `landed <sha> code`, `main` on `origin` is that merge commit, and its stderr doesn't contain `offline`

#### Scenario: a fetch that keeps failing with origin reachable
- GIVEN a passing code branch `impl/a`, and a script named `git` first on `PATH`, as in *a fetch that loses a ref race is tried again*, that fails every `fetch` naming `origin` that way
- WHEN `clm land impl/a` runs
- THEN it exits 1, its message contains `cannot lock ref` and not `offline`, `main` on `origin` is what it was before the land, and the ledger has no new record

#### Scenario: origin without main
- GIVEN a clone whose `origin` is an empty bare repository, and a branch `impl/a`
- WHEN `clm land impl/a` runs
- THEN it exits 1 naming `origin/main` and `clm reconnect`, `origin` still holds no `main`, and the ledger has no new record

#### Scenario: nothing to reconnect
- GIVEN in turn a clone with no `origin` remote, and a clone whose `origin` is reachable and that has no local `main`
- WHEN `clm reconnect` runs in each
- THEN the first exits 1 printing ``no `origin`: add the remote, then reconnect``, the second exits 1 naming `main`, and neither pushed anything

#### Needs
- host/none-keeps-a-remote
- vcs/fetch-retries

#### Serves
- capability/git-host
- capability/work-concurrently
- architecture/land-locally

### Requirement: scratch-outside-git
`clm land` SHALL merge and check in a worktree no component of whose path is `.git`: the clone's land tree, `.clm/land-tree` in the CLM root of the main worktree, when the attempt holds the lock on `.clm/land.lock` (`land-tree`), else a temporary worktree under the system's temporary directory, which it SHALL remove on every exit path.

#### Scenario: the check runs outside .git
- GIVEN a repository whose `[land] check` prints its working directory
- WHEN `clm land` runs on a branch
- THEN the directory it printed is `.clm/land-tree` in the main worktree (both compared canonicalised) and has no `.git` component

#### Scenario: removed after a failing check
- GIVEN a repository whose `[land] check` exits 1, and an empty `TMPDIR` of the test's own
- WHEN `clm land` runs on a branch
- THEN it exits 1, `TMPDIR` is empty again, and `git worktree list` names no worktree but the main one and the land tree

#### Scenario: a land that compiles nothing uses a temporary worktree
- GIVEN a branch that changes only `specs/x/SPEC.md`, a `checked` record for `origin/main`'s code key with the configured command, and a fake `CLM_CHECK` that records its working directory
- WHEN `clm land` runs on it
- THEN the directory recorded is under `TMPDIR`, not `.clm/land-tree`, and `TMPDIR` is empty again afterwards

### Requirement: shared-land-target
`clm land` SHALL leave `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL` in `[land] check`'s environment as the caller's environment has them, setting, changing and removing neither, so that where a check's build output goes is the project's check script's choice. The place `clm land` keeps for that output is `.clm/land` in the CLM root of the repository's main worktree (the first worktree `git worktree list --porcelain` lists), whichever worktree of the repository it runs in, which it hands the check as `CLM_LAND_DIR` (`land-dir-env`). Whether a land holds the clone's lock on `.clm/land.lock` and checks in the land tree SHALL follow only whether it runs `[land] check` (`land-target-lock`, `land-tree`), never the caller's `CARGO_TARGET_DIR`.

#### Scenario: land from a linked worktree
- GIVEN a repository whose `[land] check` records `CLM_LAND_DIR`, `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL`, and a linked worktree of it with a branch checked out
- WHEN `clm land` runs in the linked worktree with `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL` unset
- THEN the check saw `CLM_LAND_DIR` set to the main worktree's `.clm/land`, not the linked worktree's, and neither cargo variable set

#### Scenario: CLM root in a subdirectory
- GIVEN a git repository whose CLM root is its subdirectory `tool/`, and a `[land] check` that records `CLM_LAND_DIR`
- WHEN `clm land` runs on a branch
- THEN the check saw `CLM_LAND_DIR` set to the main worktree's `tool/.clm/land`

#### Scenario: the caller's cargo variables kept
- GIVEN the first scenario's check, which also records its working directory
- WHEN `clm land` runs with `CARGO_TARGET_DIR` set to a directory of the caller's and `CARGO_INCREMENTAL=1`
- THEN the check saw both as the caller set them, `CLM_LAND_DIR` set to the main worktree's `.clm/land`, and its working directory was the main worktree's `.clm/land-tree`

#### Serves
- capability/work-concurrently
- capability/land-safely

#### Needs
- init/starter-check-land-dir
- upgrade/migration-starter-land-dir

### Requirement: land-target-lock
A `clm land` that will run `[land] check` SHALL hold an exclusive advisory lock on `.clm/land.lock` beside the main worktree's `.clm/land` from before it checks the head out into the land tree until the push of the merge commit has gone through and the target's cap has been applied (`land-target-cap`), or the attempt ends sooner, so that the gate on the merge commit, `clm check`, the push and the cap run under it, releasing it on every exit path, whatever `CARGO_TARGET_DIR` the caller set. A land that waits for the lock SHALL say so on stderr as `lock-queue` says, naming `.clm/land`. A land that waited for the lock SHALL merge its branch with `main` as `origin` holds it once the lock is its own, never with a `main` it fetched before it waited, whichever check it then runs. A land that runs no `[land] check`, because its code key was already checked or no check is set, SHALL NOT take this lock.

#### Scenario: checked code takes no lock
- GIVEN a branch that changes only `specs/x/SPEC.md`, a `checked` record for `origin/main`'s code key, and another process holding the lock
- WHEN `clm land` runs
- THEN it lands without waiting and prints no `waiting` line

#### Scenario: a land that waited checks the new main
- GIVEN a branch `impl/a` with no `checked` record, a branch `spec/x` whose merged tree's code key has a `checked` record and whose readers select a test, and a `[land] check` that appends to a log the files of the tree it runs in and sleeps 2 seconds
- WHEN `clm land impl/a` starts, and `clm land spec/x` starts once `impl/a` holds the lock
- THEN both land, `spec/x`'s check ran in a tree holding `impl/a`'s file, and `spec/x`'s stderr holds no `main moved` line

#### Scenario: lock released after a failing check
- GIVEN a `[land] check` that exits 1
- WHEN `clm land` runs and then a second `clm land` runs
- THEN the first exits 1 and the second doesn't print `waiting`

#### Serves
- capability/work-concurrently
- capability/land-safely

#### Needs
- init/starter-check-land-dir
- upgrade/migration-starter-land-dir

### Requirement: land-binary-copy
Before running its first subcommand, `clm land` SHALL copy its own binary into a fresh directory under the system's temporary directory and run the gate, `clm affected` and `clm check` from that copy, each unless `CLM_GATE`, `CLM_AFFECTED` or `CLM_CHECK` names its command, making no copy when all three do; it SHALL remove the copy on every exit path. When the path the system gives for the running executable ends in ` (deleted)`, as on Linux once the file the process started from has been replaced or deleted (the dev install replaces `clm` whenever `main` moves, while `clm verify` runs for hours before it lands what it files, `verify/files-unproven`), it SHALL copy the file at that path without the suffix when one exists, and otherwise the running image at `/proc/self/exe`, so the land still copies a binary and lands. When the copy can't be made it SHALL exit 1 naming the path, pushing and recording nothing.

#### Scenario: the copy can't be made
- GIVEN a `TMPDIR` that is a file, so no directory can be made in it
- WHEN a land of a branch runs
- THEN it is refused naming that path, the gate never ran, `main` is unchanged and no record is written

#### Scenario: the running binary's file deleted
- GIVEN a process started from a scratch copy of a binary that calls the land, which deletes that scratch file before the land starts, on Linux, with `CLM_GATE` and `CLM_CHECK` naming passing fakes and `CLM_AFFECTED` unset, so the land still makes its copy
- WHEN the land of a branch runs in that process
- THEN it lands, `origin/main` holds the branch's commit, and the scratch file is still gone

#### Scenario: every command named
- GIVEN `CLM_GATE`, `CLM_AFFECTED` and `CLM_CHECK` each naming a passing fake that records the entries of `TMPDIR`
- WHEN a land of a branch runs
- THEN it lands, and no fake saw a `land-bin-*` directory in `TMPDIR`

### Requirement: land-frees-build-target
After a successful push of a land the gate reads as `code` or `light`, whatever the branch's name, `clm land` SHALL delete the `target/` directory at the top of the worktree it runs in when that worktree is a linked one whose `HEAD` is the landed branch, and print `removed <path>` on standard output after the `landed` line, warning without failing when the delete fails. It SHALL NOT delete the main worktree's `target/`, or delete anything after a land the gate reads as `spec` or `spec review` or a land that pushed nothing.

#### Scenario: build's target freed
- GIVEN a linked worktree with `impl/a` checked out and a `target/` directory holding a file, and a passing check
- WHEN `clm land impl/a` runs in that worktree
- THEN it exits 0, the worktree's `target/` is gone, the worktree itself is still there, and it printed `removed` with the `target/` path

#### Scenario: main worktree kept
- GIVEN the main worktree with `impl/a` checked out and a `target/` directory holding a file
- WHEN `clm land impl/a` runs there
- THEN it exits 0 and `target/` and its file are still there

#### Scenario: spec land kept
- GIVEN a linked worktree with `spec/y` checked out, a branch whose gate prints `spec`, and a `target/` directory
- WHEN `clm land spec/y` lands
- THEN `target/` is still there

#### Scenario: failed land kept
- GIVEN a linked worktree with `impl/a` checked out, a `target/` directory, and a `[land] check` that exits 1
- WHEN `clm land impl/a` runs
- THEN it exits 1 and `target/` is still there

#### Scenario: other branch checked out
- GIVEN a linked worktree with `spec/z` checked out and a `target/` directory, and a pushed branch `impl/a` with a passing check
- WHEN `clm land impl/a` runs in that worktree
- THEN it lands and `target/` is still there

#### Scenario: a code land of any name
- GIVEN a linked worktree with `fix-x` checked out, a branch whose gate prints `code`, and a `target/` directory holding a file
- WHEN `clm land fix-x` runs in that worktree
- THEN it lands, the worktree's `target/` is gone, and its standard output is `landed <sha> code` and then `removed` with the `target/` path

#### Serves
- capability/wrap-up-session

#### Needs
- gate/light

### Requirement: land-push-retry
When the push of the merge commit to `main` is refused, `clm land` SHALL fetch `origin`'s `main` again and judge it: when it is the merge commit or a commit that contains it, it SHALL treat the push as done and go on to append `landed`; when it is neither such a commit nor the fetched base, it SHALL start the next attempt (`land`); when it is still the base, it SHALL push the same merge commit again with the same lease, at most twice more in that attempt, waiting 1 second before the first of these and 2 seconds before the second, judging each refusal the same way, without counting these pushes as attempts. When `main` is still at the base after the last push it SHALL exit 2 with git's message for the refused push, including what the remote printed, and a line saying the failure may be transient and that running `clm land <branch>` again is safe, having appended no `landed` record. When the fetch after a refusal fails it SHALL exit 2 with the refused push's git message and no such line. When `append` reports the `landed` record unpushed, on any host, it SHALL warn naming the writer's ledger branch `clm/ledger/<person>/<clone>` and `git push origin` with that branch, and exit 0. When the append after a push that went through fails, it SHALL exit 2 with the ledger's error and a line saying that `main` holds the land and that running `clm land <branch>` again records it (`unrecorded-land-finished`).

#### Serves
- capability/work-concurrently
- capability/land-safely

#### Needs
- ledger/append-refused-push

#### Scenario: one transient refusal
- GIVEN a passing code branch `impl/a`, and a `pre-receive` hook on `origin` that logs each push naming `refs/heads/main`, declines the first with `declined by test` and accepts the next
- WHEN `clm land impl/a` runs
- THEN the hook logged two pushes to `main`, the check command ran once, `main` on `origin` is the merge commit of the head, the ledger has one `landed` record for it, and it exits 0 printing `landed <sha> code`

#### Scenario: refused every time
- GIVEN a passing code branch `impl/a`, and a `pre-receive` hook on `origin` that logs the time of each push naming `refs/heads/main` and declines it with `declined by test`, accepting every other push
- WHEN `clm land impl/a` runs
- THEN the hook logged three pushes to `main`, the second at least 1 second after the first and the third at least 2 seconds after the second, it exits 2, its message contains `declined by test`, `may be transient` and `clm land impl/a`, and not `keeps moving`, `main` on `origin` is what it was before the land, and the ledger has one `checked` record from this land and no `landed` record for it

#### Scenario: rerun after giving up
- GIVEN the repository of *refused every time*, with the hook then removed
- WHEN `clm land impl/a` runs again
- THEN the check command didn't run again, and it lands, printing `landed <sha> code`

#### Scenario: pushed though git failed
- GIVEN a passing code branch `impl/a`, and a script named `git` first on the `PATH` the land and the programs it starts see, that runs the real `git` and, for a `push` naming `refs/heads/main`, then exits 1 whatever the real one did
- WHEN `clm land impl/a` runs
- THEN it exits 0 printing `landed <sha> code`, `main` on `origin` is that merge commit, whose second parent is the head, and the ledger has one `landed` record for it

#### Scenario: ledger push refused
- GIVEN a passing code branch `impl/a`, and a `pre-receive` hook on `origin` that declines every push naming a `refs/heads/clm/ledger/` ref and accepts every other push
- WHEN `clm land impl/a` runs
- THEN it exits 0 printing `landed <sha> code`, `main` on `origin` is the merge commit, the local ledger branch has the `landed` record, and it warned naming that ledger branch and `git push origin`

#### Scenario: refetch fails
- GIVEN a passing code branch `impl/a`, and a script named `git` first on the `PATH` the land sees that runs the real `git` but refuses every `push` naming `refs/heads/main` and fails every `fetch` naming `main` after such a refusal
- WHEN `clm land impl/a` runs
- THEN it exits 2 with the push's message and no `may be transient`, `main` on `origin` is unchanged, and the ledger has no `landed` record for it

#### Scenario: pushed and built on
- GIVEN a passing code branch `impl/a`, and a script named `git` first on the `PATH` the land sees that runs the real `git` and, for a `push` naming `refs/heads/main`, then pushes one more commit on top of it to `origin`'s `main` and exits 1
- WHEN `clm land impl/a` runs
- THEN it exits 0 printing `landed` with its merge commit, which `main` on `origin` contains, and the ledger has one `landed` record for it

#### Scenario: append fails after the push
- GIVEN a passing code branch `impl/a`, and a script named `git` first on the `PATH` the land sees that runs the real `git`, and fails every write to a `clm/ledger/` ref once a push naming `refs/heads/main` has gone through
- WHEN `clm land impl/a` runs, and then, with the script removed, again
- THEN the first exits 2 naming the ledger's error, saying `main` holds the land and naming `clm land impl/a`, with `main` on `origin` the merge commit and no `landed` record; and the second prints `landed` with that merge commit and merges nothing

### Requirement: selected-tests
When `clm land` runs `[land] check`, it SHALL first run `clm affected --base <sha>`, `<sha>` being the commit of the `origin/main` it fetched, in the worktree it checks, write its stdout unchanged to a file in its temporary directory, and run `[land] check` with `CLM_SELECTION` set to that file's path. When `clm affected` fails, it SHALL print `clm affected failed: <reason>; the check runs every test` on one line of stderr, `<reason>` being the first non-empty line of its stderr, trimmed, or the spawn error, and run `[land] check` with `CLM_SELECTION` unset. When the merged tree's code key has a `checked` record with the same command, it SHALL instead run `clm affected --base <sha> --readers`, and SHALL run `[land] check` only when that prints a line, with `CLM_SELECTION` naming a file holding its output unchanged, or when it fails, with the same message and `CLM_SELECTION` unset; such a check SHALL run in the land tree under the lock as any other check that compiles into `.clm/land` does (`land-target-lock`), and on success SHALL append no second `checked` record. A land that runs no check SHALL run no selection other than that `--readers` one.

#### Scenario: selection handed over
- GIVEN a branch that changes one component's code, a `[land] check` that copies the file `CLM_SELECTION` names, and `CLM_AFFECTED` set to a command that prints the component line of `a` and `file crates/a/src/lib.rs`
- WHEN `clm land` runs
- THEN the copied file holds those two lines, and the branch lands

#### Scenario: base is the fetched main
- GIVEN the same branch and a `CLM_AFFECTED` command that records its arguments
- WHEN `clm land` runs
- THEN the command was given `--base` and the commit `origin/main` named when the land fetched

#### Scenario: selection fails
- GIVEN the same branch and `CLM_AFFECTED` set to a command that exits 1
- WHEN `clm land` runs
- THEN it prints `clm affected failed:` on stderr, runs `[land] check` with no `CLM_SELECTION`, and the branch lands

#### Scenario: code already checked
- GIVEN a branch whose merged tree's code key has a `checked` record with the same command, and a `CLM_AFFECTED` command that records its arguments and prints nothing
- WHEN `clm land` runs
- THEN it ran that command once, given `--base`, the fetched commit and `--readers`, never ran `[land] check`, and the branch landed

#### Scenario: readers of checked code
- GIVEN a branch that changes only `ARCHITECTURE.md`, a `checked` record for the merged tree's code key with the same command, a `[land] check` that copies the file `CLM_SELECTION` names, `CARGO_TARGET_DIR` unset, and a `CLM_AFFECTED` command that prints `file crates/a/tests/r.rs` when given `--readers`
- WHEN `clm land` runs
- THEN the copied file holds that line, the check ran in the land tree, the branch landed, and the ledger holds one `checked` record for that code key

#### Scenario: a reader fails on checked code
- GIVEN the same branch and `CLM_AFFECTED`, and a `[land] check` that exits 1
- WHEN `clm land` runs
- THEN it exits 1, and `main` on `origin` is unchanged

#### Scenario: readers' selection fails on checked code
- GIVEN the same branch and `checked` record, and `CLM_AFFECTED` set to a command that exits 1
- WHEN `clm land` runs
- THEN it prints `clm affected failed:` on stderr, runs `[land] check` with no `CLM_SELECTION`, and the branch lands

#### Needs
- check.affected/affected-readers

### Requirement: check-only
`clm land --check-only` SHALL run, in the working tree of the CLM root `--root` names, or else the one found from the current directory, `[land] check` when `clm.toml` sets it, the way a land runs it (through `sh -c`, with `CI` unset, and `CARGO_TARGET_DIR` left as the caller's environment has it, so a build's check compiles into its worktree's own `target/` (`build-places`)), and then `clm check`. It SHALL run the check command with `CLM_SELECTION` unset, so every test runs. It SHALL print `checks pass` as its last line and exit 0 when both pass; otherwise it SHALL exit 1 naming the command that failed, append nothing, and SHALL NOT run `clm check` after a failing check command. When both pass, the working tree had no change to a tracked file and no untracked file git doesn't ignore both before the check command started and after `clm check` ended, `HEAD` was the same commit both times, and the local `origin/main` is an ancestor of it, it SHALL append a `checked` record for `HEAD`'s code key with the command a land's `checked` record names (`[land] check`, or `clm check` when none is set) and print `recorded <HEAD> as checked` on stdout; otherwise it SHALL append nothing and print `not recorded: <reason>` on stdout, `<reason>` the first that holds of `the working tree has uncommitted changes`, `HEAD moved during the check` and `HEAD doesn't contain origin/main`, either line coming before `checks pass`. An append that fails SHALL only warn, `checked not recorded: <error>`. It SHALL fetch nothing, make no commit and push nothing but that record. Given a branch as well, it SHALL exit 1 without running either command. When `clm.toml` can't be loaded, it SHALL exit 1 with the reason, running neither command.

#### Scenario: checks pass on an uncommitted edit
- GIVEN a repository whose `clm.toml` sets `[land] check` to a fake that records its working directory and whether `CI` is set, a fake `CLM_CHECK` that passes, and an uncommitted edit in the working tree
- WHEN `clm land --check-only` runs there with `CI=true`
- THEN the fake ran in that working tree with `CI` unset, `clm check` ran after it, it printed `not recorded`, then `checks pass`, and exited 0, and `HEAD`, `origin/main` and the ledger are unchanged

#### Scenario: a clean committed head is recorded
- GIVEN a branch `impl/a` checked out with a clean working tree, one commit changing `crates/a/src/lib.rs` on top of `origin/main`, a `[land] check` that appends a line to a log outside the checkout, and a fake `CLM_CHECK` that passes
- WHEN `clm land --check-only` runs, and then `clm land impl/a` with `origin/main` unmoved
- THEN the first printed `recorded` with `HEAD`'s commit and `checks pass`, the ledger has a `checked` record for `HEAD`'s code key with the configured command, the land landed printing `landed <sha> code`, and the log has one line

#### Scenario: main moved after the record
- GIVEN the same branch after its recorded check, and `origin/main` then moved by a commit changing `crates/b/src/lib.rs`
- WHEN `clm land impl/a` runs
- THEN the check command ran again, on the merge's tree, and the branch landed

#### Scenario: head behind main
- GIVEN a clean branch whose `HEAD` doesn't contain `origin/main`
- WHEN `clm land --check-only` runs with passing checks
- THEN it prints `not recorded` naming `origin/main`, then `checks pass`, and the ledger is unchanged

#### Scenario: a commit during the check
- GIVEN a clean branch containing `origin/main`, and a `[land] check` that commits a new file and passes
- WHEN `clm land --check-only` runs
- THEN it prints `not recorded` naming `HEAD`, then `checks pass`, and the ledger is unchanged

#### Scenario: check command fails
- GIVEN the same repository with a fake check command that fails
- WHEN `clm land --check-only` runs
- THEN it exits 1 naming the check command, `clm check` never ran, and the ledger is unchanged

#### Scenario: no check command
- GIVEN a repository whose `clm.toml` has no `[land]` table and a fake `CLM_CHECK` that passes
- WHEN `clm land --check-only` runs
- THEN only `clm check` ran, and it printed `checks pass` and exited 0

#### Scenario: with a branch
- GIVEN the same repository
- WHEN `clm land --check-only impl/a` runs
- THEN it exits 1 and neither the check command nor `clm check` ran

#### Scenario: unreadable settings
- GIVEN a repository whose `clm.toml` isn't valid TOML
- WHEN `clm land --check-only` runs
- THEN it exits 1 naming `clm.toml`, and neither the check command nor `clm check` ran

#### Scenario: another root
- GIVEN a repository whose `[land] check` records its working directory, and a current directory outside it
- WHEN `clm land --check-only --root <repository>` runs
- THEN the check ran in that repository's working tree and it printed `checks pass`

#### Serves
- capability/work-concurrently
- capability/land-safely

#### Needs
- gate/files-outside-spec-paths
- rename/replay

### Requirement: derived-specs
When the head adds a change, as `gate.fingerprint/fingerprint` defines adding one, or applies one, a change whose `changes/<id>/change.md` has status `proposed` or `approved` at the base and, in the merge commit, has status `applied`, or is gone while `changes/archive/<id>/change.md` has status `done`, `clm land` SHALL, after the merge commit and before any check, regenerate `specs/` with `sync.regenerate`'s `regenerated`, without its stale-delta merge, onto the base, `origin/main`'s on every host (`land/offline-land`), from the CLM root in the tree holding the merge commit, for the added changes whose status is `applied`, or `done` for a new archived one, and the applied ones, in the order `regenerated` chooses (`sync.regenerate/regenerated-tree`), a `change.md` that doesn't parse regenerating nothing, and compare the result with the merge commit's `specs/` byte for byte over the union of both sides' paths in byte order, a path on one side only being a difference. When they differ, or `regenerated` stops, it SHALL push nothing and exit 1 with rule `land/derived-specs`, naming the first differing path or the lines `regenerated` returned, and `clm rebase`. A head that adds and applies no change SHALL land without the comparison, since its `specs/` edits are its own.

#### Scenario: regenerated branch lands
- GIVEN `spec/x` adding applied change `x`, whose delta adds requirement `a/r`, with `specs/` as `clm rebase` regenerates it, and an `origin/main` that has since added requirement `b/s`
- WHEN `clm land spec/x` runs
- THEN it lands, and `main`'s `specs/a/SPEC.md` holds `a/r` and `specs/b/SPEC.md` holds `b/s`

#### Scenario: change built on another lands whatever its ID
- GIVEN `spec/x` adding applied changes `x` and `w`, `w` sorting first, where `x`'s delta rewrites the first line of `a`'s Design with `origin/main`'s hash as its base and `w`'s rewrites its second line with the Design's hash after `x` applies as its base, with `specs/` as `clm rebase` regenerates it
- WHEN `clm land spec/x` runs
- THEN it prints no `land/derived-specs` line and lands, and `main`'s `specs/a/SPEC.md` holds both edits

#### Scenario: applied change's deltas lost
- GIVEN an `origin/main` holding change `y` with `status: proposed` and a delta adding requirement `a/x`, and `spec/z` that adds change `z`, which has no delta, and sets `y` to `applied`, with `specs/` as `origin/main`'s, so `a/x` is missing
- WHEN `clm land spec/z` runs
- THEN it exits 1 printing `land/derived-specs`, naming `specs/a/SPEC.md` and `clm rebase`, and `main` on `origin` is unchanged

#### Scenario: hand edit under specs refused
- GIVEN the same branch with one more commit changing a line of `specs/a/SPEC.md` that no delta of `x` holds, and an `approved` record for its fingerprint
- WHEN `clm land spec/x` runs
- THEN it exits 1 printing `land/derived-specs`, naming `specs/a/SPEC.md` and `clm rebase`, the check command never ran, and `main` on `origin` is unchanged

#### Scenario: branch that adds no change
- GIVEN `impl/a`, which adds no change and sets `status: active` in `specs/a/SPEC.md`
- WHEN `clm land impl/a` runs
- THEN it lands with that edit

#### Serves
- capability/catch-drift
- architecture/hash-based-staleness
- architecture/humans-review-intent

#### Needs
- clm/renamed-commands
- gate.fingerprint/fingerprint
- sync.regenerate/regenerated-tree
- land.approval/approve

### Requirement: land-tree
An attempt of `clm land` that holds the lock on `.clm/land.lock` SHALL check the head and the merge commit out in `.clm/land-tree` in the CLM root of the main worktree, a detached worktree of the repository that it SHALL keep when the attempt ends. Before each checkout it SHALL make that tree exist and clean: when git lists the path as a worktree and its directory exists, `git checkout --force --detach` there and then `git clean -fdx`; otherwise, or when either fails, it SHALL remove whatever is at the path, prune git's worktree list and add the worktree afresh. When it can't remove what is at the path, or can't add the worktree afresh, it SHALL exit 2 naming the path, having pushed and appended nothing. Everything a land keeps in the main worktree, the land tree, `.clm/land`, `.clm/land.lock`, `.clm/check.lock` and `.clm/land-logs/`, SHALL be under that CLM root's `.clm/`, so that a repository whose `.gitignore` lists only the entries `clm init` adds (`init/init-ignores-local-settings`) shows none of it in `git status`.

#### Scenario: one tree for every land
- GIVEN two branches `impl/a` and `impl/b`, no `checked` records, and a `[land] check` that records its working directory
- WHEN `clm land impl/a` runs and then `clm land impl/b`
- THEN both checks ran in the main worktree's `.clm/land-tree`, and `git worktree list` names it afterwards, detached

#### Scenario: leftovers cleaned
- GIVEN the same branches and a `[land] check` that exits 1 when a file `left` exists in its working directory and otherwise writes it
- WHEN `clm land impl/a` runs and then `clm land impl/b`
- THEN both land

#### Scenario: tree repaired after .clm is removed
- GIVEN a land of `impl/a` that made the land tree, and the main worktree's `.clm/` then removed
- WHEN `clm land impl/b` runs
- THEN it lands and its check ran in `.clm/land-tree`

#### Scenario: stale directory replaced
- GIVEN a directory at the main worktree's `.clm/land-tree`, holding a file, that git lists no worktree for
- WHEN `clm land impl/a` runs
- THEN it lands, its check ran in `.clm/land-tree`, and the file is gone

#### Scenario: nothing untracked in a project that doesn't ignore target
- GIVEN a repository whose committed `.gitignore` lines are `clm.local.toml`, `.clm/` and `.claude/worktrees/` and nothing else, a branch `impl/a` with a passing `[land] check` that writes a file into `CLM_LAND_DIR`, and a branch `impl/b` whose `[land] check` exits 1
- WHEN `clm land impl/a` runs and then `clm land impl/b`
- THEN the first lands, the second exits 1, and `git status --porcelain` in the main worktree prints nothing, with no `target/` directory there

#### Scenario: the land tree can't be made
- GIVEN a branch `impl/a` with no `checked` record, and the main worktree's `.clm/land-tree` a path the land can neither remove nor add a worktree at
- WHEN `clm land impl/a` runs
- THEN it exits 2 naming `.clm/land-tree`, the check command never ran, and `main` on `origin` and the ledger are unchanged

#### Needs
- init/starter-check-land-dir
- upgrade/migration-starter-land-dir

### Requirement: check-log
`clm land` SHALL write `[land] check`'s stdout and stderr, in the order produced, to `.clm/land-logs/<branch>.log` in the CLM root of the main worktree while also printing them to stderr. When the check fails its refusal SHALL hold, under a line naming the check, the last 40 lines of that output, kept in memory, and end with a line naming the log's path, and it SHALL keep the log; when the check passes it SHALL remove the log. When the log can't be created it SHALL warn naming the path and run the check all the same, and a failing check's refusal then names no path. The check MAY mark its steps in its output with GitHub Actions' group lines (`check-step-markers`): a line starting `::group::` starts a step named by the rest of the line, trimmed, ending any step open, and a line `::endgroup::` ends the open step, each tested with trailing whitespace removed. When a step is open as a failing check's output ends, the refusal's last line SHALL also name that step and the first and last line numbers, counted from 1 in the whole output, of the lines it spans, from its `::group::` line to the output's last line. When no step is open then, because the check marked none or ended every step it marked, the refusal SHALL be as it is for a check that marks none.

#### Scenario: failing check's tail and log
- GIVEN a `[land] check` that prints the lines `1` to `100`, the odd ones to stdout and the even ones to stderr, and exits 1
- WHEN `clm land impl/a` runs
- THEN it exits 1, its stderr holds a line naming the check followed by the lines `61` to `100`, the refusal names the main worktree's `.clm/land-logs/impl/a.log` and no step, and that file holds the lines `1` to `100` in order

#### Scenario: short output repeated whole
- GIVEN a `[land] check` that prints three lines and exits 1
- WHEN `clm land impl/a` runs
- THEN its stderr holds the three lines after the line naming the check

#### Scenario: passing check leaves no log
- GIVEN a passing `[land] check`
- WHEN `clm land impl/a` runs
- THEN it lands and the main worktree has no `.clm/land-logs/impl/a.log`

#### Scenario: log can't be created
- GIVEN a file, not a directory, at the main worktree's `.clm/land-logs`
- WHEN `clm land impl/a` runs with a passing check
- THEN it warns naming the log's path, the check ran, and the branch landed

#### Scenario: the failing step named
- GIVEN a `[land] check` that prints `::group::unit`, 5 lines and `::endgroup::`, then `::group::e2e`, a line `boom` and 60 more lines, and exits 1
- WHEN `clm land impl/a` runs
- THEN it exits 1, its stderr holds the last 40 lines after the line naming the check, and the refusal's last line names the step `e2e`, lines 8 to 69, and the log's path

#### Scenario: a step that ended isn't named
- GIVEN a `[land] check` that prints `::group::unit`, a line and `::endgroup::`, then a line `boom`, and exits 1
- WHEN `clm land impl/a` runs
- THEN it exits 1 and the refusal names the log's path and no step

#### Scenario: a group marker split across writes
- GIVEN a `[land] check` that writes `::gro`, sleeps 0.2 seconds, writes `up::e2e` and a newline, prints a line `boom`, and exits 1
- WHEN `clm land impl/a` runs
- THEN the refusal names the step `e2e`, lines 1 to 2

### Requirement: records-effort
After the push to `main`, `clm land` SHALL append, in the commit that appends `landed` and before it, the `effort` records `clm effort record --branch <branch>` would append for the landed branch's sessions on this machine, read once before its first attempt from the transcripts directory its environment gives (`effort/record-command`). When they can't be read it SHALL warn, naming why, and land as `land` states.

#### Scenario: sessions recorded
- GIVEN a transcript of a session on `impl/a` run in the repository
- WHEN `clm land impl/a` lands it
- THEN the fold holds an `effort` record for `impl/a`, written in the commit that writes the `landed` record and before its line, and the last line printed is `landed <sha> code`

#### Scenario: no transcripts
- GIVEN no transcripts directory
- WHEN `clm land impl/a` runs
- THEN it lands, prints `landed <sha> code`, appends no `effort` record and warns `effort: no transcripts at <dir>`

#### Scenario: push retried
- GIVEN a transcript on `impl/a`, and `origin` declining the first push to `main`
- WHEN `clm land impl/a` lands it on a later push
- THEN the ledger holds exactly one `effort` record, in the commit that writes the `landed` record

#### Scenario: push never taken
- GIVEN a transcript on `impl/a`, and `origin` declining every push to `main`
- WHEN `clm land impl/a` gives up
- THEN the ledger holds no `effort` record

#### Scenario: effort unreadable
- GIVEN a checkout `clm effort record` refuses
- WHEN `clm land impl/a` runs
- THEN it lands, appends `landed`, and warns `effort not recorded: <why>`

#### Serves
- capability/review-capability-cost
- architecture/effort-accounted

### Requirement: land-target-cap
After the push of a land that holds the lock on `.clm/land.lock`, and before it releases that lock, `clm land` SHALL sum the lengths of the regular files under the main worktree's `.clm/land`, following no symbolic link, and when the sum is greater than `[land] target_cap_gb` gigabytes (10^9 bytes each) SHALL remove `.clm/land` and print on stderr `cleared .clm/land (<n> GB, cap <cap> GB)`, `<n>` the sum in whole gigabytes rounded down. When any entry can't be read the measure fails and nothing is removed; a failure to measure or remove SHALL print a warning naming the path and leave the land's outcome unchanged. A land that took no lock SHALL neither measure nor remove it, and no land SHALL measure or remove a directory the caller's `CARGO_TARGET_DIR` names.

#### Scenario: over the cap
- GIVEN a `clm.toml` with `[land]`, a `check` that creates a sparse file of 2,000,000,000 bytes in `CLM_LAND_DIR`, and `target_cap_gb = 1`, and a branch `impl/a`
- WHEN `clm land impl/a` runs
- THEN it prints `landed`, stderr holds `cleared .clm/land (2 GB, cap 1 GB)`, and the main worktree's `.clm/land` doesn't exist

#### Scenario: under the cap
- GIVEN the same, with a check that creates a file of 10 bytes and the cap left at its default
- WHEN `clm land impl/a` runs
- THEN it lands, prints no `cleared` line, and the file is still under `.clm/land`

#### Scenario: failed check keeps the target
- GIVEN the over-the-cap check, made to exit 1 after creating its file
- WHEN `clm land impl/a` runs
- THEN it exits 1 and `.clm/land` still holds the file

#### Scenario: caller's target untouched
- GIVEN the over-the-cap setup, its check made to create the file in `CARGO_TARGET_DIR` instead
- WHEN `clm land impl/a` runs with `CARGO_TARGET_DIR` set to a directory of the caller's
- THEN it lands, prints no `cleared` line, and the caller's directory still holds the file

#### Serves
- capability/work-concurrently
- capability/land-safely

#### Needs
- init/starter-check-land-dir
- upgrade/migration-starter-land-dir
- config/land-check-setting

### Requirement: reconnect-names-rebase
When `origin/main` has moved, the message `clm reconnect` prints SHALL tell the person to run `clm rebase` on each branch landed locally and then land it again, starting ``origin's `main` has moved``.

#### Scenario: moved while offline
- GIVEN a repository that landed `impl/a` on its local `main` while offline, and an `origin/main` that has moved since
- WHEN `clm reconnect` runs
- THEN it prints ``origin's `main` has moved`` with `clm rebase`, prints `land again impl/a`, and exits 1

#### Serves
- architecture/plain-terms

### Requirement: spec-branch-holds-spec-paths
`clm land <branch>`, for a branch named `spec/<id>`, SHALL exit 1 when the head's kind against the fetched base, as `gate/evaluate-at` gives it in-process, is other than `spec`, so `code`, `light` and `mixed` alike, naming each file the head changes against its merge base that isn't a spec path (`gate/classify`) and saying a spec branch holds only spec paths: the name claims a spec branch, which a maintainer lands from an outside contribution (`capability/contribute-from-outside`), so code on it never lands on checks. It SHALL do so before it takes the land's lock, merges or runs any check, and it SHALL push and append nothing. When that in-process call fails, as when git fails, it SHALL exit 2 with its error, before the lock. For a branch of any other name this rule SHALL refuse nothing.

#### Scenario: code on a spec branch
- GIVEN a branch `spec/x` that changes only `crates/x/src/lib.rs`, whose gate prints `code`, and a check command that records that it ran
- WHEN `clm land spec/x` runs
- THEN it exits 1 naming `crates/x/src/lib.rs`, the check command never ran, and `main` on `origin` and the ledger are unchanged

#### Scenario: a suggestion on a spec branch
- GIVEN a branch `spec/x` that only adds `backlog/x.md`, whose gate prints `spec`
- WHEN `clm land spec/x` runs
- THEN it lands

#### Scenario: code on an impl branch
- GIVEN a branch `impl/x` that changes only `crates/x/src/lib.rs`
- WHEN `clm land impl/x` runs
- THEN this rule refuses nothing and it lands

#### Scenario: a light branch named spec
- GIVEN a branch `spec/c` that the gate reads as `light`, adding a scenario to `a/x` with `crates/a/src/lib.rs` and `crates/a/tests/t.rs`, and a check command that records that it ran
- WHEN `clm land spec/c` runs
- THEN it exits 1 naming `crates/a/src/lib.rs` and `crates/a/tests/t.rs`, the check command never ran, and `main` on `origin` is unchanged

#### Needs
- gate/evaluate-at
- gate/files-outside-spec-paths
- gate/classify
- gate/light

#### Serves
- capability/contribute-from-outside
- architecture/ai-writes-code

### Requirement: check-lock
Every run of `[land] check` by `clm land`, whatever `CARGO_TARGET_DIR` the caller set, and every run by `clm land --check-only` whose `CLM_SLOT` is 0, the slot `check-slot` sets for the run (from the checkout's branch, never the caller's environment), SHALL hold an exclusive advisory lock on `.clm/check.lock` in the CLM root of the repository's main worktree (the first worktree `git worktree list --porcelain` lists), whichever worktree of the repository it runs in, from before the command starts until it exits, releasing it on every exit path, so that no two such checks of one clone run at the same time. A run that waits for the lock SHALL say so on stderr as `lock-queue` says, naming `.clm/check.lock`, and wait with no time limit, taking its turn in the order `lock-queue` sets. A `clm land --check-only` run whose `CLM_SLOT` isn't 0, a build's check in its own place, SHALL neither take the lock nor wait for it. A land that also holds the lock on `.clm/land.lock` (`land-target-lock`) SHALL take it after that lock. `clm check`, `clm affected` and the gate SHALL NOT run under it, and a land that runs no `[land] check` SHALL NOT take it.

#### Scenario: overlapping check-only runs
- GIVEN two linked worktrees of one clone on branches no place names, a `[land] check` that appends `start <worktree>` to a log outside both, sleeps 2 seconds and appends `end <worktree>`, and a fake `CLM_CHECK` that passes
- WHEN `clm land --check-only` starts in both at once
- THEN both print `checks pass`, in the log every `start` is followed by the `end` of the same worktree before any other `start`, and the run that waited printed `waiting` naming `.clm/check.lock` on stderr

#### Scenario: a check-only run beside a land
- GIVEN a branch `impl/a` with no `checked` record, a linked worktree on a branch no place names, and the same logging `[land] check`
- WHEN `clm land impl/a` and `clm land --check-only` in the linked worktree start at once
- THEN the land lands, the check-only run prints `checks pass`, and the two checks never interleave in the log

#### Scenario: a check-only run in a build place
- GIVEN a place `a` naming `impl/a` with slot 1, a linked worktree on `impl/a`, a fake `CLM_CHECK` that passes, and another process holding the lock on `.clm/check.lock`
- WHEN `clm land --check-only` runs in that worktree
- THEN it prints `checks pass` without waiting and prints no `waiting` line

#### Scenario: a land with the caller's target
- GIVEN two branches `impl/a` and `impl/b`, no `checked` records, the same logging `[land] check`, and `CARGO_TARGET_DIR` set to a different directory for each land
- WHEN `clm land impl/a` and `clm land impl/b` start at once
- THEN both land and the two checks never interleave in the log

#### Scenario: released after a failing check
- GIVEN a `[land] check` that exits 1
- WHEN `clm land --check-only` runs and then a second `clm land --check-only` runs
- THEN both exit 1 naming the check command, and the second doesn't print `waiting`

#### Scenario: checked code takes no check lock
- GIVEN a branch that changes only `specs/x/SPEC.md`, a `checked` record for `origin/main`'s code key, and another process holding the lock on `.clm/check.lock`
- WHEN `clm land` runs
- THEN it lands without waiting and prints no `waiting` line

#### Needs
- orchestrator/place-slots

#### Serves
- capability/work-concurrently
- capability/land-safely

### Requirement: verify-stop
Before merging, `clm land` SHALL take the *held* components: the `components` of every `kind: bug` item on the fetched `origin/main`'s backlog whose `verify_failures` is not empty, each held by every such item that names it. A held component the branch's code changes a file of, a code file being one the gate doesn't count as a spec path and the file's component being its owner as `check.coverage/owner` gives it over the specs at the fetched base, is *fixed* when a commit the branch adds carries the trailer `Resolves: backlog/<slug>` for at least one item holding it, so that separate branches each fixing one of several items that hold a component each land, while the items not yet resolved go on holding it. When such a component is not fixed, `clm land` SHALL push nothing and exit 1 with rule `land/verify-stop`, naming each held component it touches and is not fixed with every item that holds it, and saying that the branch lands once a commit it adds carries the trailer `Resolves: backlog/<slug>` for one item holding each component named; a branch every held component of which it touches is fixed SHALL land as `land` states. It SHALL also read the red `verified` results on `origin/main`'s first-parent chain after the newest passing one, the newest record for each commit (`ledger/verified-record`), and for each that no backlog item's `verify_failures` names and one of whose `components` owns a code file the branch changes, warn `slow tests failed after <sha>, culprit <land|unknown>, on <component>...; run /clm:triage to take it in` and land as `land` states. A land whose code changes no file a held component owns, a spec land among them, SHALL land as `land` states.

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

#### Scenario: a test another component's tests glob owns
- GIVEN component `b` owning `crates/b/**` and component `x` whose `tests` glob owns `crates/b/tests/x.rs`, and a branch changing only `crates/b/tests/x.rs`; in turn with `b` held by a `kind: bug` item with `verify_failures`, and with `x` held instead
- WHEN `clm land` runs in each
- THEN the first lands, and the second exits 1 with `land/verify-stop` naming `x`

#### Needs
- check.coverage/owner

#### Serves
- capability/slow-tests-after-land

### Requirement: shared-check-runner
`clm-land` SHALL offer as a library call the runner that `clm land` and `clm land --check-only` run `[land] check` through: it takes the name of the command that runs it, the command, the directory to run it in, an optional selection file for `CLM_SELECTION`, the environment to set and unset, and the log file's path, runs the command through `sh -c` under the clone's check lock (`check-lock`), waiting for it with an ordinary ticket and printing its waiting line as `lock-queue` says when it waits, copies its output to stderr and the log as `check-log` says, writing, when the log can't be created, the warning `<name>: warning: can't write the check's log at <path>: <error>; its output goes to stderr alone` live on stderr, `<name>` being the name it was given, and returns whether it passed and the command's exit code (none when a signal ended it), with the last 40 lines, whether the log holds the whole output, and the step open when its output ended, if any, with that step's name and first and last line numbers as `check-log` counts them, leaving the log for the caller to keep or remove. `clm land` and `clm land --check-only` SHALL run `[land] check` through it, each giving the name `clm land` and the environment `land-dir-env`, `check-slot` and `check-env` give. A write to stderr that fails (its waiting line, its copy of the command's output, its warning about the log) SHALL neither end the run nor change what it returns: the runner goes on reading the command's output until the pipe closes and returns its verdict as for any other run.

#### Scenario: a caller waits on the check lock
- GIVEN a command that writes `ok` and exits 0, and `.clm/check.lock` held by another process that releases it after 1 second
- WHEN the runner runs the command
- THEN it printed `waiting for another check` ending `: 1 ahead`, ran the command after the lock was released, wrote `ok` to the log, and returned a pass

#### Scenario: the log's warning
- GIVEN a command that writes `ok` and exits 0, a log path whose directory can't be created, and the name `clm x`
- WHEN the runner runs the command under that name
- THEN it writes on stderr a line starting `clm x: warning: can't write the check's log at ` with the log's path, runs the command and returns a pass

#### Scenario: a land names itself
- GIVEN a branch to land, a `[land] check` that exits 0, and `.clm/land-logs` a file, so the check's log can't be created
- WHEN `clm land` runs
- THEN stderr holds a line starting `clm land: warning: can't write the check's log at `, and the branch landed

#### Scenario: the open step returned
- GIVEN a command that prints `::group::a`, a line, `::group::b` and two lines, and exits 1
- WHEN the runner runs the command
- THEN it returns a failure with the step `b`, lines 3 to 5

#### Scenario: the exit code returned
- GIVEN a command that exits 127
- WHEN the runner runs the command
- THEN it returns a failure with exit code 127

#### Scenario: a failing stderr ends nothing
- GIVEN Linux, a command that prints 1000 numbered lines and exits 1, a log path whose directory can't be created, `.clm/check.lock` held by another process that releases it after 1 second, and the runner's stderr opened on `/dev/full`
- WHEN the runner runs the command
- THEN it returns a failure with exit code 1 and the last 40 of those lines, having run the command to its end

#### Serves
- capability/slow-tests-after-land

### Requirement: settings-unchanged-is-checked
When no `checked` record names the merged tree's code key with the same command, `clm land` SHALL count the tree as checked when the bytes of the CLM root's `clm.toml` differ between the fetched base and the merge commit (a missing file read as `None`), `gate/settings-unchanged` holds of its bytes at the two, and a `checked` record names, with the same command, the key `ledger/code-key-with` gives for the merge commit with that `clm.toml` taken from the fetched base. It SHALL then run what a land whose code key is checked runs (`clm check --no-detectors`, preceded by `[land] check` only when the readers `clm affected --base <sha> --readers` prints select a file, a test marked `clm:reads clm.toml` among them) and, once that passes, SHALL append a `checked` record for the merged tree's own code key with that command before the push. The lookup SHALL be made where the attempt computes the merged tree's code key, before it decides on the land lock, so such a land takes the lock only when its readers select a file. Any other difference in `clm.toml`, or no `checked` record for that key, SHALL leave the land to run its check as `land` states.

#### Scenario: the upgrade stamp only
- GIVEN a branch that changes only `upgraded` in `clm.toml` and edits `docs/x.md`, no `clm:reads` marker naming either, and a `checked` record for `origin/main`'s code key with the configured command
- WHEN `clm land` runs
- THEN the check command never ran, `clm check --no-detectors` ran on the merged tree, the ledger has a `checked` record for the merged tree's code key, and the branch landed with that code key

#### Scenario: a comment only
- GIVEN a branch that only appends a commented-out table to `clm.toml`, and a `checked` record for `origin/main`'s code key with the configured command
- WHEN `clm land` runs
- THEN the check command never ran and the branch landed

#### Scenario: a reader of clm.toml
- GIVEN the same stamp-only branch and `checked` record, and `crates/e/tests/r.rs` holding `clm:reads clm.toml`
- WHEN `clm land` runs
- THEN the check command ran once with `CLM_SELECTION` naming a file that holds only `file crates/e/tests/r.rs`, and the branch landed

#### Scenario: a setting changed
- GIVEN a branch that changes `upgraded` and `[land] check` in `clm.toml`, and a `checked` record for `origin/main`'s code key with the old command
- WHEN `clm land` runs
- THEN the new check command ran and `clm check` ran with its detectors

#### Scenario: code changed too
- GIVEN a branch that changes only `upgraded` in `clm.toml` and `crates/a/src/lib.rs`, and a `checked` record for `origin/main`'s code key
- WHEN `clm land` runs
- THEN the check command ran

#### Needs
- ledger/code-key-with
- gate/settings-unchanged
- clm/doc-readers-marked

#### Serves
- capability/upgrade-clm
- capability/land-safely

### Requirement: check-slot
Every run of `[land] check`, by `clm land <branch>` or by `clm land --check-only`, SHALL set `CLM_SLOT` in the command's environment to the slot `orchestrator/place-slots` gives a branch: for a land, the branch it lands; for `--check-only`, the branch the checkout has checked out, or 0 without asking when the checkout's `HEAD` is detached. It SHALL replace any `CLM_SLOT` the caller's environment holds, setting `CLM_SLOT=0` too. When the slot can't be read, it SHALL exit 2 with the reason and run no check. The check lock (`check-lock`) and `CLM_SELECTION` SHALL be unchanged by it.

#### Scenario: a check-only run in a build's place
- GIVEN a place `a` naming `impl/a` with slot 2, a checkout on `impl/a`, and a `[land] check` that writes `$CLM_SLOT` to a file outside the checkout
- WHEN `clm land --check-only` runs
- THEN it prints `checks pass` and the file holds `2`

#### Scenario: a land
- GIVEN the same place and `[land] check`, and no `checked` record for the merged tree's code key
- WHEN `clm land impl/a` runs
- THEN it lands and the file holds `2`

#### Scenario: outside a place
- GIVEN a checkout on a branch `x` that no place names, the same `[land] check`, and `CLM_SLOT=5` in the caller's environment
- WHEN `clm land --check-only` runs
- THEN the file holds `0`

#### Needs
- orchestrator/place-slots

#### Serves
- architecture/tests-run-concurrently
- capability/work-concurrently

### Requirement: regenerates-behind-specs
When the head adds or applies a change, as `derived-specs` defines both, and a file under `specs/` changed both between the merge base and the base and between the merge base and the head, and every path `git merge-tree --write-tree --name-only` reports conflicted, if any, lies under `specs/`, `clm land` SHALL land the head without sending its author to `clm rebase`. It SHALL first regenerate `specs/` with `sync.regenerate`'s `regenerated`, without its stale-delta merge, from the head's tree onto the merge base, for the changes the head adds whose status at the head is `applied` or `done` and those it applies over the merge base, and compare the result with the head's `specs/` as `derived-specs` compares; it SHALL then regenerate `specs/` from the merged tree onto the base for the changes `derived-specs` names, and make the merge commit's tree the merged tree with `specs/` replaced by that result. When either regenerate stops, or the first comparison finds a difference, it SHALL push nothing and exit 1 with rule `land/derived-specs`, naming the differing path or the lines `regenerated` returned, and `clm rebase`. It SHALL run the gate it runs on the head on that merge commit instead, before any check. A head that adds and applies no change, or whose merge conflicts in a path outside `specs/`, SHALL be refused as `land` states.

#### Scenario: behind in a derived file lands
- GIVEN `spec/x` adding applied change `x`, whose delta adds requirement `a/r`, with `specs/` as `clm rebase` regenerated it, and an `origin/main` that has since landed a change adding requirement `a/s` to the same `specs/a/SPEC.md`, so the file changed on both sides
- WHEN `clm land spec/x` runs
- THEN it prints no `gate/up-to-date` or `land/derived-specs` line and lands, `main`'s `specs/a/SPEC.md` holds `a/r` and `a/s`, and the gate ran on the merge commit

#### Scenario: main moves during the check
- GIVEN that `spec/x` on an `origin/main` that hasn't moved, and a check command that, the first time it runs, lands on `origin/main` a change adding requirement `a/s` to `specs/a/SPEC.md`
- WHEN `clm land spec/x` runs
- THEN its second attempt lands with no `clm rebase` in between, and `main`'s `specs/a/SPEC.md` holds `a/r` and `a/s`

#### Scenario: hand edit on a behind branch refused
- GIVEN the behind `spec/x` with one more commit changing a line of `specs/a/SPEC.md` that no delta of `x` holds
- WHEN `clm land spec/x` runs
- THEN it exits 1 printing `land/derived-specs`, naming `specs/a/SPEC.md` and `clm rebase`, the check command never ran, and `main` on `origin` is unchanged

#### Scenario: delta stale against main refused
- GIVEN `spec/x` whose delta modifies requirement `a/q`, and an `origin/main` that has since landed a change modifying `a/q`
- WHEN `clm land spec/x` runs
- THEN it exits 1 printing `land/derived-specs` and `clm rebase`, the check command never ran, and `main` on `origin` is unchanged

#### Scenario: branch that adds no change still refused
- GIVEN `impl/a`, which adds no change and sets `status: active` in `specs/a/SPEC.md`, and an `origin/main` that has since changed `specs/a/SPEC.md`
- WHEN `clm land impl/a` runs
- THEN it exits 1 naming `clm rebase`, and `main` on `origin` is unchanged

#### Serves
- capability/work-concurrently
- architecture/hash-based-staleness

### Requirement: lock-queue
The clone's land lock, `.clm/land.lock` (`land-target-lock`), and its check lock, `.clm/check.lock` (`check-lock`), SHALL each go to the runs waiting for it in an order CLM sets, not in the operating system's: a run that must wait takes a ticket in a queue beside the lock in the main worktree's `.clm/`, and tries the lock only when its turn comes. A ticket SHALL carry a *lane*, priority or ordinary, and a priority from `P1` to `P5`. A `clm land` takes a priority-lane ticket when its merged tree's code key counts as checked and its readers select a test (`selected-tests`), so that its `[land] check` runs only those readers, and an ordinary one otherwise, a land whose readers' selection failed, so that every test runs, included; its priority is the one `orchestrator/land-priority` gives the branch, its head and the fetched base, and `P5` when that can't be worked out, when it SHALL warn `land priority not worked out, queueing at P5: <why>` and go on. Every other run takes an ordinary ticket at `P5`. The turn SHALL go first to the `P1` tickets, a priority-lane one before an ordinary one and otherwise the one taken first; then to the other priority-lane tickets, the highest priority first and then the one taken first; then to the other ordinary tickets in the same order. When six runs in a row below `P1` have taken the lock with priority-lane tickets while an ordinary ticket was waiting, and no `P1` ticket is waiting, the ordinary ticket whose turn would come first among the ordinary ones SHALL take the next turn. A ticket SHALL hold no turn once its run has ended, however it ended, and no ticket of an ended run SHALL stay in the queue past the next run that reads it. A ticket file a reading run can't open SHALL hold no turn in that reading and be left where it is, since the lock and not the queue keeps two runs apart. A waiting run SHALL print its waiting line on stderr when it begins to wait and again each time the count changes: `waiting for another land's check on <path>: <n> ahead` for the land lock, `<path>` the main worktree's `.clm/land`, and `waiting for another check on <path>: <n> ahead` for the check lock, `<path>` the main worktree's `.clm/check.lock`, `<n>` counting the run that holds the lock and the waiting runs whose turn comes before its own. A run that finds the lock free and no ticket waiting SHALL take the lock at once, printing nothing. A queue's `next`, `streak` or `holder` file that is missing SHALL read as its default, 0 or empty; a lock file, a queue directory or a queue file that can't be created, read, written or locked otherwise SHALL end the run with exit 2 naming its path, pushing and appending nothing. The order SHALL decide only which run takes the lock next: a run that takes the lock without the queue, such as one by an older `clm`, still never holds it beside another. The push turn has an order of its own (`push-turn`).

#### Scenario: turns in arrival order
- GIVEN another process holding `.clm/check.lock`, three linked worktrees `a`, `b` and `c` on branches no place names, a `[land] check` that appends `start <worktree>` to a log outside them, and a fake `CLM_CHECK` that passes
- WHEN `clm land --check-only` starts in `a`, then in `b` once `a` has printed its waiting line, then in `c` once `b` has, and then the holder releases the lock
- THEN the log reads `start a`, `start b`, `start c`, and `c` printed a waiting line ending `: 3 ahead`, then `: 2 ahead`, then `: 1 ahead`

#### Scenario: a checked land goes first
- GIVEN another process holding `.clm/check.lock`, a `clm land --check-only` run outside any build place waiting for it, and then a `clm land` of a branch that changes only a file a test reads, whose merged tree's code key has a `checked` record and whose readers select that test
- WHEN the holder releases the lock
- THEN the land's check starts before the `--check-only` run's, and the land printed a waiting line ending `: 1 ahead`

#### Scenario: priority runs can't starve the rest
- GIVEN another process holding the lock, one run waiting with an ordinary `P5` ticket, and then eight runs waiting with priority-lane `P5` tickets
- WHEN the holder releases the lock and each run releases it as soon as it holds it
- THEN the lock goes to six priority-lane runs, then the ordinary run, then the other two priority-lane runs

#### Scenario: a killed waiter holds no turn
- GIVEN another process holding `.clm/check.lock`, a `clm land --check-only` run outside any build place waiting for it, and a second such run waiting after it
- WHEN the first waiting run is killed with `SIGKILL`, and then the holder releases the lock
- THEN the second run's waiting line goes from `: 2 ahead` to `: 1 ahead`, it takes the lock and passes, and its queue holds no ticket once it has ended

#### Scenario: nothing waiting
- GIVEN no process holding `.clm/check.lock` and an empty queue
- WHEN `clm land --check-only` runs outside any build place
- THEN it prints no waiting line, and the queue holds no ticket once it has ended

#### Scenario: a run outside the queue
- GIVEN a process holding `.clm/check.lock` with no ticket, as an older `clm` would, and a `clm land --check-only` run waiting for it
- WHEN the waiting run's turn comes while the process still holds the lock
- THEN the run waits until the process releases it, its waiting line counting the process as `1 ahead`, and the two never hold the lock at once

#### Scenario: P1 goes first
- GIVEN another process holding the lock, a run waiting with a priority-lane `P5` ticket, then a run with an ordinary `P1` ticket, then a run with a priority-lane `P1` ticket
- WHEN the holder releases the lock and each run releases it as soon as it holds it
- THEN the lock goes to the priority-lane `P1` run, then the ordinary `P1` run, then the priority-lane `P5` run

#### Scenario: below P1, cheap before urgent
- GIVEN another process holding the lock, a run waiting with an ordinary `P2` ticket, then a run with a priority-lane `P4` ticket, then a run with an ordinary `P3` ticket, then a run with a priority-lane `P3` ticket
- WHEN the holder releases the lock and each run releases it as soon as it holds it
- THEN the lock goes to the priority-lane `P3` run, the priority-lane `P4` run, the ordinary `P2` run, then the ordinary `P3` run

#### Scenario: P1 doesn't wait for the cap
- GIVEN another process holding the lock, an ordinary `P5` ticket waiting, six runs below `P1` having just taken the lock in a row with priority-lane tickets while it waited, and a run waiting with an ordinary `P1` ticket
- WHEN the holder releases the lock
- THEN the `P1` run takes it before the ordinary `P5` run

#### Scenario: a P1 land
- GIVEN another process holding `.clm/check.lock`, a `clm land --check-only` run outside any build place waiting for it, and then a `clm land` of `impl/a` whose code key has no `checked` record, where `a`'s effective priority on `origin/main` is `P1`
- WHEN the holder releases the lock
- THEN the land's check starts before the `--check-only` run's, and the land printed a waiting line ending `: 1 ahead`

#### Scenario: the waiting lines
- GIVEN another process holding `.clm/land.lock`, and then `.clm/check.lock`
- WHEN `clm land impl/a` of a branch with no `checked` record runs while the first is held, and `clm land --check-only` outside any build place while the second is
- THEN the land printed `waiting for another land's check on <main worktree>/.clm/land: 1 ahead` and the check-only run `waiting for another check on <main worktree>/.clm/check.lock: 1 ahead`

#### Scenario: a ticket it can't open
- GIVEN another process holding `.clm/check.lock`, and in its queue a ticket file named as a waiting ticket whose permissions let no one open it, whose run has ended
- WHEN `clm land --check-only` runs outside any build place, and the holder releases the lock
- THEN its waiting line ends `: 1 ahead`, it takes the lock and passes, and the file is still there

#### Scenario: an unwritable queue
- GIVEN in turn a file, not a directory, at the main worktree's `.clm/check.queue` with another process holding `.clm/check.lock`, and a file at `.clm/land.queue` with another process holding `.clm/land.lock`
- WHEN `clm land --check-only` runs outside any build place in the first, and `clm land impl/a` of a branch with no `checked` record in the second
- THEN each exits 2 naming the queue's path, the check command never ran, and `main` on `origin` and the ledger are unchanged

#### Scenario: a priority that can't be worked out
- GIVEN a branch `impl/a` with no `checked` record, another process holding `.clm/land.lock` that releases it after 1 second, and specs on `origin/main` that don't load
- WHEN `clm land impl/a` runs
- THEN it lands, and its standard error holds `clm land: warning: land priority not worked out, queueing at P5: ` and the reason

#### Needs
- orchestrator/land-priority

#### Serves
- capability/work-concurrently

### Requirement: step-times
When `clm land` runs `[land] check`, the `landed` record it appends SHALL carry the command's wall-clock `check_seconds` and, for each step the check marked (`check-step-markers`), in order, a `steps` entry with the step's name, its seconds from its `::group::` line to its end, and as `selected_by` the paths of the `::clm-selected::<path>` lines within it, each once in the order first seen, all in whole seconds rounded down. A check that marks no steps SHALL give `check_seconds` and no steps, and a land that runs only `clm check` SHALL give neither. `clm land --check-only` SHALL record no timing.

#### Scenario: two steps
- GIVEN a `[land] check` that prints `::group::a`, `::clm-selected::x/y.rs`, `::clm-selected::x/y.rs`, sleeps 1 second, prints `::endgroup::` and `::group::b`, and exits 0
- WHEN `clm land impl/a` lands
- THEN its `landed` record has `check_seconds` of at least 1 and two steps, `a` with at least 1 second and `selected_by` `x/y.rs` once, then `b` with no `selected_by`

#### Scenario: no steps marked
- GIVEN a `[land] check` that prints `ok` and exits 0
- WHEN `clm land impl/a` lands
- THEN its `landed` record has `check_seconds` and no steps

#### Scenario: code already checked
- GIVEN a branch whose merged tree's code key has a `checked` record and whose readers select no file
- WHEN `clm land` lands it
- THEN its `landed` record has neither `check_seconds` nor `steps`

#### Needs
- ledger/landed-times

### Requirement: check-time-report
After the push of a land whose check command ran and passed, `clm land` SHALL print on stderr `check <n>s, target <t>s`, `<n>` the command's whole seconds rounded down and `<t>` being `[land] check_target_seconds`. When `<n>` is greater than `<t>` it SHALL then print, in this order: a line for each of the up to three steps with the most seconds, most first and ties in the check's order, naming the step and its seconds, each followed by the chain `clm affected --why` prints for the step's first `selected_by` path against the selection's base, or saying it was selected by no file; a line with how many of this land and the last 19 timed lands before it (`ledger/landed-times`) exceeded the target, naming the up to three steps with the most total seconds across them; and a line naming `clm guide ci-and-deployment`. It SHALL file no backlog item, and the land's outcome and exit status SHALL NOT depend on the report.

#### Scenario: under target
- GIVEN `check_target_seconds = 60` and a `[land] check` that marks one step and passes at once
- WHEN `clm land impl/a` lands
- THEN stderr holds a line matching `check <n>s, target 60s`, and no line names `clm guide ci-and-deployment`

#### Scenario: over target
- GIVEN `check_target_seconds = 1`, a branch `impl/a` that changes `b/src/lib.rs`, owned by `b`, a component `a` that depends on `b` and owns `a/src/lib.rs`, and a `[land] check` that prints `::group::fmt`, `::group::slow`, `::clm-selected::a/src/lib.rs`, sleeps 2 seconds and passes
- WHEN `clm land impl/a` lands
- THEN it prints `landed`, and stderr holds, after `check <n>s, target 1s`, a line naming `slow` with its seconds, below it the chain naming `a/src/lib.rs`, `a`, `b` and `b/src/lib.rs`, a line naming `fmt` and that it was selected by no file, a line saying 1 of the last 1 timed lands exceeded the target with most time in `slow`, and a line naming `clm guide ci-and-deployment`; the backlog is unchanged

#### Needs
- check.affected/why-chain
- config/land-check-setting
- ledger/landed-times
- clm/affected-why-command

#### Serves
- capability/slow-tests-after-land

### Requirement: overlap-recheck
A `clm land` attempt whose merged tree's code key no `checked` record names with the land's command, directly or through the settings-free key (`settings-unchanged-is-checked`), SHALL check only the overlap when all of these hold: `[land] check` is set; a `checked` record with the same command names the code key of the branch's head `H`; one names the code key of the fetched base `B`; and `H` doesn't contain `B`. It SHALL run `clm affected --overlap <H> <B>` (`check.affected/overlap-selection`) in the tree on the merge commit, and then:
- when it prints nothing, run no `[land] check`, take neither the lock on `.clm/land.lock` nor the one on `.clm/check.lock`, run `clm check` with its detectors on the merge commit in a temporary worktree as an attempt with no lock does, and print `re-check: none, the branch and main since <short merge base> affect no test in common` on stderr;
- when it prints a line, go on as an attempt whose code key isn't checked, with `CLM_SELECTION` naming a file holding that output unchanged in place of `selected-tests`' selection, and print `re-check: overlap, <n> files` on stderr, `<n>` the number of its `file` lines, or `re-check: overlap, every test` when it prints an `all` line;
- when it fails, print `clm affected failed: <the first non-empty line of its stderr, or the spawn error>; the land checks its own selection` and go on as `selected-tests` says.

A check that passed SHALL NOT be run again because `main` moved. An attempt that follows one of the same `clm land` whose `[land] check` and `clm check` passed on its merge commit `P` with an earlier `main`, whichever path that attempt took, the readers of checked code included, SHALL take `P` for `H` when a `checked` record with the land's command names the code key of the fetched base `B`: it SHALL run `clm affected --overlap <P> <B>` and go on as the three cases above say, inside its push turn (`push-turn`), so that it runs again neither its own selection nor its readers, only what its changes and `main`'s new commits can both break. Its composed `checked` record's `basis` SHALL name `P`'s code key and `B`'s; when the new merged tree's code key already counts as checked, it SHALL append none.

When `clm check` and any `[land] check` it ran pass, the land SHALL append a `checked` record for the merged tree's code key with the land's command and a `basis` naming the two code keys (`ledger/composed-check`), and its `landed` record SHALL say how it re-checked (`ledger/landed-recheck`): `recheck: none`, or `overlap` with `recheck_files` the number of `file` lines, null for an `all` line. Every other land SHALL say it too: `checked` when its code key counted as checked, and `selected` when it ran `selected-tests`' selection, with `recheck_files` counted the same way, null when every test ran. The gate on the merge commit and the push SHALL run as for any land. An attempt for which any condition fails SHALL check as `selected-tests` says.

#### Scenario: main moved on an unrelated component
- GIVEN components `a` and `b` with no `depends_on` between them, a `[land] check` that appends to a log outside the repository, a branch `impl/a` whose head changes only `crates/a/src/lib.rs` and has a `checked` record, and `main` moved since by a land changing only `crates/b/src/lib.rs`, whose code key has a `checked` record
- WHEN `clm land impl/a` runs while another process holds the lock on `.clm/check.lock`
- THEN it prints `landed`, without waiting, the log is empty, stderr holds `re-check: none`, and the ledger has a `checked` record for the merged tree's code key whose `basis` names both keys, and the `landed` record holds `recheck: none`

#### Scenario: a dependant of both
- GIVEN the same, and a component `c` with `depends_on: [a, b]` and a test `crates/c/tests/t.rs`, and a `[land] check` that copies the file `CLM_SELECTION` names
- WHEN `clm land impl/a` runs
- THEN the copy holds `file crates/c/tests/t.rs` and no file of `a` or `b`, stderr holds `re-check: overlap, 1 files`, and the `landed` record holds `recheck: overlap` and `recheck_files: 1`

#### Scenario: the head was never checked whole
- GIVEN the first scenario's repository with no `checked` record for the head
- WHEN `clm land impl/a` runs
- THEN the check runs with `clm affected --base`'s selection, as `selected-tests` says, and stderr holds no `re-check:` line

#### Scenario: main's head not checked
- GIVEN the first scenario's repository, with `main`'s head a commit pushed outside `clm land`, whose code key no `checked` record names
- WHEN `clm land impl/a` runs
- THEN the check runs with `clm affected --base`'s selection and stderr holds no `re-check:` line

#### Scenario: main moved during the check, on an unrelated component
- GIVEN components `a` and `b` with no `depends_on` between them, a branch `impl/a` with no `checked` record, a `[land] check` that appends `start` to a log outside the repository and waits for a file to appear, and a branch whose head changes only `crates/b/src/lib.rs` and whose land runs no check
- WHEN `clm land impl/a` is in its check, the other branch lands, and the file appears
- THEN `impl/a` lands, the log holds one `start`, its stderr holds `re-check: none`, and the `checked` record for the tree it landed has a `basis` naming the code key of the merge commit it checked and that of the `main` it landed on

#### Scenario: main moved during a readers' check
- GIVEN a branch `spec/x` whose merged tree's code key has a `checked` record and whose readers select test `t`, the same waiting `[land] check`, and a branch whose land runs no check and changes no file `t` reads
- WHEN `clm land spec/x` is in its check, the other branch lands, and the file appears
- THEN `spec/x` lands, and the log holds one `start`

#### Scenario: main moved during the check, with a dependant in common
- GIVEN the first of these scenarios, a component `c` with `depends_on: [a, b]` and a test `crates/c/tests/t.rs`, and a `[land] check` that also copies the file `CLM_SELECTION` names on each run
- WHEN `clm land impl/a` is in its check, the other branch lands, and the file appears
- THEN `impl/a` lands, its check ran twice, and the second copy holds `file crates/c/tests/t.rs` and no file of `a` or `b`

#### Needs
- check.affected/overlap-selection
- ledger/composed-check
- ledger/landed-recheck

### Requirement: land-dir-env
Every run of `[land] check` by `clm land` SHALL set `CLM_LAND_DIR` in the command's environment to the `.clm/land` directory of the main worktree's CLM root, the place `shared-land-target` names, replacing any value the caller's environment holds, so a project's check can keep its build output there whatever its toolchain. Every run of `[land] check` by `clm land --check-only` SHALL run with `CLM_LAND_DIR` removed from its environment.

#### Scenario: a land in the land tree
- GIVEN a `[land] check` that writes `$CLM_LAND_DIR` to a file outside the repository, a branch `impl/a` with no `checked` record, and `CLM_LAND_DIR=/elsewhere` in the caller's environment
- WHEN `clm land impl/a` runs from a linked worktree
- THEN the file holds the path of the main worktree's `.clm/land`

#### Scenario: a land with the caller's target
- GIVEN the same, and `CARGO_TARGET_DIR` set to a directory of the caller's
- WHEN `clm land impl/a` runs
- THEN the file holds the path of the main worktree's `.clm/land`

#### Scenario: a check-only run
- GIVEN the same `[land] check` and `CLM_LAND_DIR=/elsewhere` in the caller's environment
- WHEN `clm land --check-only` runs
- THEN the file holds an empty line

#### Needs
- init/starter-check-land-dir
- upgrade/migration-starter-land-dir

### Requirement: shared-target-cap
`clm-land` SHALL offer as a library call the cap that `land-target-cap` applies to `.clm/land`: it takes a directory, the name to print for it and a cap in gigabytes (10^9 bytes each), sums the lengths of the regular files under the directory, following no symbolic link, and when the sum is greater than the cap removes the directory and returns the line `cleared <name> (<n> GB, cap <cap> GB)`, `<n>` the sum in whole gigabytes rounded down. A directory that doesn't exist SHALL return nothing and remove nothing; when any entry can't be read the measure fails, nothing is removed, and it SHALL return a warning naming the path, as it SHALL when the removal fails. `clm land` SHALL apply `land-target-cap` through this call, naming the directory `.clm/land`.

#### Scenario: over the cap
- GIVEN a directory holding a sparse file of 2,000,000,000 bytes and a symbolic link to a directory outside it holding another such file
- WHEN the call runs on it with the name `.clm/x` and a cap of 1
- THEN it returns `cleared .clm/x (2 GB, cap 1 GB)`, the directory doesn't exist, and the file the link pointed to still does

#### Scenario: under the cap
- GIVEN a directory holding a file of 10 bytes
- WHEN the call runs on it with a cap of 1
- THEN it returns nothing and the file is still there

#### Scenario: no directory
- GIVEN a path where nothing exists
- WHEN the call runs on it
- THEN it returns nothing

### Requirement: check-env
Every run of `[land] check`, by `clm land <branch>` or by `clm land --check-only`, SHALL set in the command's environment each variable of the `[env]` table (`config/env-setting`) that the CLM root's `clm.toml` in the tree it checks sets (the merge commit's for a land, the working tree's for `--check-only`), with the clone's `clm.local.toml` over it, replacing any value the caller's environment holds for that name. The command a `checked` record names, and that every lookup of a `checked` record with the same command compares, SHALL be `[land] check` preceded by `<NAME>=<value> ` for each of those variables in byte order of the names, or `[land] check` alone when there is none, so a record counts only for a check that ran with the same variables. When a land's variables differ from those the fetched base's `clm.toml` gives with the same `clm.local.toml`, it SHALL run `[land] check` with `CLM_SELECTION` unset, since a variable can change any test's result. `CLM_SLOT` (`check-slot`), `CLM_LAND_DIR` (`land-dir-env`), the removal of `CI` and the check lock (`check-lock`) SHALL be as they are without it.

#### Scenario: the clone's value wins
- GIVEN a `[land] check` that writes `$RELAY_SERVER_IMAGE` to a file outside the repository, a `clm.toml` with `[env]` setting `RELAY_SERVER_IMAGE = "relay:amd64"`, a `clm.local.toml` with `[env]` setting `RELAY_SERVER_IMAGE = "relay:arm64"`, and `RELAY_SERVER_IMAGE=relay:shell` in the caller's environment
- WHEN `clm land --check-only` runs
- THEN the file holds `relay:arm64`, and the `checked` record it appends names the command `RELAY_SERVER_IMAGE=relay:arm64 ` followed by the check

#### Scenario: no table
- GIVEN the same check, no `[env]` in either file, and `RELAY_SERVER_IMAGE=relay:shell` in the caller's environment
- WHEN `clm land --check-only` runs
- THEN the file holds `relay:shell`, and the `checked` record names the check alone

#### Scenario: a record under other variables
- GIVEN a branch whose merged tree's code key has a `checked` record naming `[land] check` alone, and a `clm.local.toml` with `[env]` setting `A = "1"`
- WHEN `clm land` runs
- THEN it runs `[land] check` as for unchecked code, and appends a `checked` record naming `A=1 ` followed by the check

#### Scenario: a changed variable runs every test
- GIVEN a branch that changes only `clm.toml`'s `[env]`, and a `[land] check` that writes whether `CLM_SELECTION` is set to a file outside the repository
- WHEN `clm land` runs
- THEN the file says it is unset

#### Needs
- config/env-setting

### Requirement: push-turn
Every `clm land` attempt SHALL hold the clone's *push turn*, which one attempt holds at a time, from before the fetch of `main` its push relies on until that push has gone through or the attempt ends, on every exit path, so that between that fetch and that push no other `clm land` of the clone moves `main`. An attempt that runs `[land] check` on its merge SHALL take the turn once that check has passed, still holding the land lock (`land-target-lock`), and SHALL run inside it whatever `overlap-recheck` then asks. An attempt that runs no `[land] check` SHALL take the push turn and no other lock, so it never waits for a check that isn't an overlap re-check. The turn SHALL go to the attempts waiting for it in the order they asked, and an attempt that waits SHALL print `waiting for another land's push: <n> ahead` on stderr when it begins to wait and each time `<n>` changes.

#### Scenario: a land with nothing to check doesn't wait for a check
- GIVEN a branch `impl/a` with no `checked` record, a `[land] check` that waits for a file to appear, and a branch whose head changes only `specs/x/SPEC.md`, whose merged tree's code key has a `checked` record and whose readers select nothing
- WHEN `clm land impl/a` is in its check and `clm land` of the other branch runs
- THEN the other branch lands before the file appears, and its stderr holds no `waiting` line

#### Scenario: two lands with nothing to check
- GIVEN two branches whose lands run no `[land] check`, and a fake `CLM_CHECK` that sleeps 2 seconds and passes
- WHEN `clm land` starts for both at once
- THEN both land, the one that waited printed `waiting for another land's push: 1 ahead`, and neither's stderr holds a `main moved` line

#### Scenario: turns in the order asked
- GIVEN three branches whose lands run no `[land] check`, and a fake `CLM_CHECK` that sleeps 2 seconds and passes
- WHEN `clm land` starts for the first, then the second once the first holds the turn, then the third once the second has printed its waiting line
- THEN `main`'s first-parent history holds their merge commits in that order

#### Scenario: released when the attempt fails
- GIVEN a branch whose land runs no `[land] check` and a fake `CLM_CHECK` that exits 1
- WHEN `clm land` runs for it and then for a second such branch with a `CLM_CHECK` that passes
- THEN the first exits 1, and the second lands and prints no `waiting` line

#### Serves
- capability/work-concurrently
- capability/land-safely

### Requirement: batched-lands
A `clm land` that takes the land lock SHALL land with its own branch, on one run of `[land] check`, the other lands of the clone that are waiting for that lock and whose tickets name their branch, in the order their turns would have come (`lock-queue`); a waiting land whose ticket names none, as an older `clm`'s, SHALL be left to wait. For each land it takes it SHALL do what that land's own attempt does before its check, judging its head as a land alone judges it (`build-holds-own-files`, `spec-branch-holds-spec-paths`, `deleted-tests-approved`, `verify-stop`), and SHALL run the gate on that land's own merge commit against that commit's first parent, reading the approval policy there, as a land does against the `main` it merges into. It SHALL leave out, to wait for the lock and land alone, a land either of those would refuse; one that conflicts with `main` and the lands taken before it; one whose head and the commit it would merge onto have both changed a file under `specs/` since their merge base, which only a land alone regenerates (`regenerates-behind-specs`); and one whose check environment differs from its own (`check-env`). The lands taken SHALL reach `main` in one push, each as its own merge commit `Merge <branch> into main` in that order, with `[land] check` and `clm check` run once, on the last of those commits, the check's selection holding every test one of them would have run landing alone and every test two of them can both break. Each land taken SHALL get the `effort` and `landed` records its own land would have appended, except that its `landed` record names, with its own merge commit, the code key the check ran on, where a land alone names its merged tree's (`land`); says `recheck: selected` with the shared selection's `recheck_files`; and carries the check's `check_seconds` and `steps` only on the record of the first of the lands that run landed (`step-times`). It SHALL append all of those records before it releases the land lock. So `clm ledger verify` finds each of them recorded and checked, and no waiting land finds its head on `main` unrecorded while the land that pushed it still runs. Each waiting `clm land` SHALL then print `landed <its merge commit> <kind>`, `<kind>` the word its land alone would print, `light` included, its `landed` record holding `light` true as a light land's alone does, and exit 0 and remove what a land removes, having run no `[land] check` itself. A land with no other waiting SHALL land alone, as `land` states.

#### Scenario: three waiting lands, one check
- GIVEN branches `impl/a`, `impl/b` and `impl/c` on unrelated components with no `checked` records, a `[land] check` that appends `start` to a log outside the repository and sleeps 2 seconds, and another process holding the land lock
- WHEN `clm land` starts for `impl/a`, then `impl/b`, then `impl/c`, each once the one before has printed its waiting line, and the holder releases the lock
- THEN the log holds one `start`, `main`'s first-parent history ends with `Merge impl/a into main`, `Merge impl/b into main` and `Merge impl/c into main`, each `clm land` printed `landed` with its own commit, the ledger holds a `landed` record for each with the same `code_key` and `recheck: selected`, only `impl/a`'s holds `check_seconds` and `steps`, and `clm ledger verify` prints nothing

#### Scenario: a land that conflicts with one before it
- GIVEN the same, with `impl/b` and `impl/c` both changing the same line of a file `main` holds
- WHEN the three lands run as above
- THEN `impl/a` and `impl/b` land on one check, and `impl/c`'s `clm land` exits 1 telling the caller to run `clm rebase`, as it does landing alone

#### Scenario: a land the gate refuses once another is merged before it
- GIVEN `impl/a` and `spec/b` waiting in that order, and a fake `CLM_GATE` that passes except on a merge commit of `spec/b` whose first parent holds `impl/a`, where it reports `gate/approval`
- WHEN the holder releases the lock
- THEN `impl/a` lands alone on its check, and `spec/b`'s `clm land`, on its own turn, exits 1 with the gate's `gate/approval`

#### Scenario: the selection covers what two of them share
- GIVEN components `a` and `b`, a component `c` with `depends_on: [a, b]` and a test `crates/c/tests/t.rs`, branches `impl/a` and `impl/b` each with a `checked` record for its head, and a `[land] check` that copies the file `CLM_SELECTION` names
- WHEN the two lands wait and are landed together
- THEN the copy holds `file crates/c/tests/t.rs`

#### Scenario: a land with another check environment
- GIVEN `impl/a` and `impl/b` waiting in that order, `impl/b`'s `clm land` run with a `[land.env]` variable set to another value than `impl/a`'s (`check-env`), and a `[land] check` that appends `start` to a log
- WHEN the holder releases the lock
- THEN both land, and the log holds two `start` lines

#### Scenario: a ticket that names no branch
- GIVEN `impl/a` waiting, and a second waiting ticket that names no branch
- WHEN the holder releases the lock
- THEN `impl/a` lands alone, and the second ticket is still waiting

#### Scenario: a land alone
- GIVEN a branch with no `checked` record and nothing else waiting
- WHEN `clm land` runs
- THEN it lands as before, with one merge commit and one `landed` record holding `check_seconds`

#### Scenario: a light land taken with another
- GIVEN `impl/b` and then `impl/a` waiting for the land lock with no `checked` records, and a fake gate that prints `light` for `impl/a`'s merge commits and `code` for `impl/b`'s
- WHEN the holder releases the lock and the two are landed together
- THEN `impl/a`'s `clm land` prints `landed <its merge commit> light`, `impl/b`'s prints `landed <its merge commit> code`, and `impl/a`'s `landed` record has `land_kind` `code` and `light` true

#### Scenario: a land behind in specs waits
- GIVEN `impl/a` and `spec/b` waiting in that order, where `spec/b` adds an applied change whose `specs/b/SPEC.md` `origin/main` has changed since `spec/b` was made, and a `[land] check` that appends `start` to a log
- WHEN the holder releases the lock
- THEN `impl/a` lands alone on its check, and `spec/b` then lands on its own turn with its `specs/` regenerated, the log holding two `start` lines

#### Serves
- capability/work-concurrently
- capability/land-safely

#### Needs
- ledger/landed-record-fields
- gate/light
- gate/evaluate-at
- land.approval/branch-head

### Requirement: failed-batch
When `[land] check` or `clm check` fails on lands taken together (`batched-lands`), the land that holds the lock SHALL push nothing, SHALL print on its stderr, in place of a refusal, one line saying the check of the lands taken together failed and what it does next, keeping no log of that run, and SHALL say nothing of the failure to the others. The *suspects* are the lands taken whose own part of the selection, what each would have run landing alone and what it and another of them can both break, holds a path the step that was open as the check's output ended named with a `::clm-selected::` line (`check-log`, `step-times`); a land whose part is every test is always one; and when no step was open, the step named no path, no land would be a suspect by this, or it was `clm check` that failed, every land taken is a suspect. The holder SHALL then, before it releases the lock:
- take together again the lands that aren't suspects, when there are any, and treat a failure of theirs as this rule says, a single land so taken that fails being a suspect;
- when there are four suspects or fewer, leave each to land alone, as a land alone lands or fails, with that land's own output, log and exit code: its own at once, and each other on its own turn, when it SHALL neither take other lands with it nor be taken with another's;
- when there are more than four, take together the first half of them, then the second, each half as this rule says.

So a `clm land` is told its check failed only by its own run, on a tree holding `main` and its branch alone. A check of lands taken together that a signal ends, with no exit code, is no failed batch: the holder SHALL exit 1 naming the check command, pushing nothing and marking none of the others, which wait for the lock as lands never taken together.

#### Scenario: the failed step points at one land
- GIVEN three waiting lands as in `batched-lands`' first scenario, where `impl/c` makes `[land] check` exit 1 inside a step that printed `::clm-selected::crates/c/tests/t.rs`, a path only `impl/c`'s selection holds
- WHEN the holder releases the lock
- THEN `impl/a` and `impl/b` land on the second run of the check, and `impl/c`'s `clm land` then runs the check alone, exits 1 naming the check command, and `main` doesn't hold `impl/c`

#### Scenario: no step says who
- GIVEN the same three lands, a `[land] check` that marks no steps, and `impl/c` making it exit 1
- WHEN the holder releases the lock
- THEN the check runs four times in all, once on the three together and once on each alone, `main` is unchanged by the first run, `impl/a`'s stderr holds the line saying the check of the lands taken together failed, `impl/a` and `impl/b` land, and `impl/c`'s `clm land` exits 1 naming the check command

#### Scenario: the holder's own branch breaks the check
- GIVEN the three lands and the `[land] check` that marks no steps of the scenario before, where `impl/a` makes the check exit 1
- WHEN the holder releases the lock
- THEN the check runs four times in all, `impl/a`'s `clm land` exits 1 naming the check command after running the check alone, and `impl/b` and `impl/c` then land, each alone

#### Scenario: the failed step names a path of no land
- GIVEN the three lands of the first scenario, where `impl/c` makes `[land] check` exit 1 inside a step that printed `::clm-selected::docs/x.md`, a path no land's selection holds
- WHEN the holder releases the lock
- THEN each of the three runs the check alone, as when no step says who

#### Scenario: more than four suspects are halved
- GIVEN six waiting lands `impl/a` to `impl/f` on unrelated components, a `[land] check` that marks no steps and appends `start` to a log, and `impl/f` making it exit 1
- WHEN the holder releases the lock
- THEN `impl/a`, `impl/b` and `impl/c` land together on the second run, `impl/d` and `impl/e` each land alone, `impl/f`'s `clm land` exits 1, and the log holds six `start` lines

#### Scenario: a check a signal ends
- GIVEN the three lands of the first scenario, and a `[land] check` that kills itself with `SIGKILL` the first time it runs and passes after that
- WHEN the holder releases the lock
- THEN `impl/a`'s `clm land` exits 1 naming the check command with `main` unchanged by it, and `impl/b` and `impl/c` then land

#### Serves
- capability/work-concurrently
- capability/land-safely

### Requirement: unrecorded-land-finished
A `clm land` of a branch whose head `main` already holds, merged by a commit on `main`'s first-parent chain that no `landed` record names, as a land killed between its push and its records leaves it, SHALL finish that land and merge nothing, holding the land lock while it does, so it never finishes a land whose own `clm land` is still appending its records. When a `checked` record covers the commit, naming its own code key or, for a commit pushed with lands taken together (`batched-lands`), the code key their check ran on, and the land was one `clm land` would have made, judged on that commit as a land judges its merge (the branch's fingerprint taken from the commit's two parents and approved under the policy at its first parent, and the gate run on the commit), it SHALL append the `landed` record naming that commit and that key, with no `recheck`, `check_seconds` or `steps`, and the branch's `effort` records, print `landed <commit> <kind>` and exit 0. It SHALL do the same for every other commit of that push whose `landed` record is missing, leaving unrecorded, and naming on stderr, one that fails that judgement, so that after one of the lands taken together has run, `clm ledger verify` reports none of those a land would have made. When no `checked` record covers the commit, or the land was not one `clm land` would have made, it SHALL append nothing and exit 1 naming the commit and `ledger/unrecorded-land`.

#### Scenario: lands taken together, pushed and not recorded
- GIVEN `main` holding the merge commits of three lands taken together as `batched-lands` pushes them, a `checked` record for the code key their check ran on, and no `landed` record for any of them
- WHEN `clm land impl/b` runs
- THEN it prints `landed` with `impl/b`'s merge commit and exits 0, `main` is unchanged, the ledger holds a `landed` record for each of the three commits, each with no `check_seconds`, and `clm ledger verify` prints nothing

#### Scenario: a land alone, pushed and not recorded
- GIVEN `main` holding the merge commit of `impl/a` as a land alone pushes it, a `checked` record for that commit's code key, and no `landed` record for it
- WHEN `clm land impl/a` runs
- THEN it prints `landed` with that commit, `main` is unchanged, and `clm ledger verify` prints nothing

#### Scenario: the land that pushed is still running
- GIVEN the first scenario's repository, and another process holding the land lock
- WHEN `clm land impl/b` runs
- THEN it prints `waiting` naming `.clm/land` and appends nothing until the lock is released

#### Scenario: a merge no land made
- GIVEN a commit pushed to `main` by hand that merges `impl/a`, whose code key no `checked` record names
- WHEN `clm land impl/a` runs
- THEN it exits 1 naming the commit and `ledger/unrecorded-land`, and the ledger is unchanged

#### Scenario: an unapproved branch pushed by hand
- GIVEN a branch that needs review and has no approval, and a commit pushed to `main` by hand that merges it, whose code key a `checked` record names
- WHEN `clm land` runs for the branch
- THEN it exits 1 naming the commit and `ledger/unrecorded-land`, and the ledger is unchanged

#### Serves
- capability/land-safely
- architecture/git-is-truth

#### Needs
- ledger/landed-record-fields
- gate/light

### Requirement: build-holds-own-files
`clm land <branch>`, for a branch named `impl/<component>` other than `impl/archive/<id>`, the claim of a build of that component, SHALL exit 1 when `gate/own-files`' `foreign_files` lists any file for its head against the fetched base and that component, printing for each `<file>: land/build-holds-own-files: owned by `<owner>`; impl/<component> changes its own files, the wiring and a capability test it serves. Build this in `<owner>`, or leave the requirement unbuilt and add a question item that blocks it`. For a file `foreign_files` lists with an empty owner, a file outside the CLM root (`gate/paths-under-the-root`), it SHALL print `<file>: land/build-holds-own-files: outside the CLM root, which no build of the root's components may change. Leave the requirement unbuilt and add a question item that blocks it` instead, naming no owner. It SHALL do so before it takes the land's lock, merges or runs any check, and it SHALL push and append nothing. `clm land --check-only`, in a checkout whose branch is `impl/<component>`, SHALL print the same lines for `HEAD` against the local `origin/main` and exit 1 before it runs the check command, so a build learns of it before its land; with no local `origin/main` it SHALL judge nothing. For a branch of any other name, and for a detached `HEAD`, this rule SHALL refuse nothing: whether a code branch's files are one component's is the gate's to judge for every branch (`gate/own-files`), and its block is the land's refusal (`land`).

#### Scenario: another component's source
- GIVEN a root with `wiring: []`, components `a` and `b`, and a branch `impl/a` that changes `crates/a/src/lib.rs` and `crates/b/src/lib.rs`, and a check command that records that it ran
- WHEN `clm land impl/a` runs
- THEN it exits 1 naming `crates/b/src/lib.rs` and `b` under `land/build-holds-own-files`, the check command never ran, and `main` on `origin` and the ledger are unchanged

#### Scenario: the build's pre-check
- GIVEN the same branch checked out
- WHEN `clm land --check-only` runs
- THEN it prints the same line, exits 1, and the check command never ran

#### Scenario: its own files
- GIVEN the same root and a branch `impl/a` that changes only `crates/a/src/lib.rs`
- WHEN `clm land impl/a` runs
- THEN it lands

#### Scenario: a branch of another name
- GIVEN the same root and a branch `tidy-b` that changes `crates/a/src/lib.rs` and `crates/b/src/lib.rs`, and a gate that blocks it with a `gate/own-files` line for each of the two files, as `gate/own-files` blocks such a branch
- WHEN `clm land tidy-b` runs
- THEN this rule refuses nothing, and the land exits 1 printing the gate's two lines, the check command never having run

#### Scenario: not switched on
- GIVEN a root with no `wiring`, and the first scenario's branch
- WHEN `clm land impl/a` runs
- THEN it lands

#### Scenario: a file outside the root
- GIVEN a git repository whose CLM root is its directory `tool/`, with `wiring: []` and component `a` owning `crates/a/**`, a branch `impl/a` that changes `tool/crates/a/src/lib.rs` and `b` at the repository's top, and a check command that records that it ran
- WHEN `clm land impl/a` runs in `tool/`
- THEN it exits 1 printing `../b: land/build-holds-own-files: outside the CLM root, which no build of the root's components may change. Leave the requirement unbuilt and add a question item that blocks it`, with no `owned by` line, and the check command never ran

#### Scenario: no origin main to judge against
- GIVEN the first scenario's branch checked out, and the clone's `refs/remotes/origin/main` deleted
- WHEN `clm land --check-only` runs
- THEN it prints no `land/build-holds-own-files` line and the check command ran

#### Needs
- gate/own-files

#### Serves
- architecture/path-ownership

### Requirement: landed-change-releases-claims
After a successful push of a branch that adds a change, whatever its name, `clm land` SHALL delete on `origin`, through `host`'s leased delete (`host/branch-delete-call`) leased on the tip its last fetch read, `propose/<slug>`, once per slug, for each item that a change the head adds names in `from` (as `backlog/<slug>`) or `resolves`, a change the head adds being one `gate.fingerprint/fingerprint` counts as added, a live `changes/<id>/change.md` or a new archived `changes/archive/<id>/change.md` the head has and the merge base has under neither path, its `change.md` read from the merged tree, through the frontmatter `model/parse-changes` reads, and SHALL write `released propose/<slug>` to stderr for each claim it deleted. It SHALL skip a slug with no `origin/propose/<slug>` on the fetched refs, a `from` that doesn't start `backlog/`, and a `change.md` whose frontmatter doesn't parse; a delete that fails SHALL warn as `landed-branch-cleanup`'s does, and SHALL NOT fail the land. It SHALL release no other claim, and none after a land that adds no change.

#### Scenario: claims on the change's items released
- GIVEN `origin/propose/q`, `origin/propose/r` and `origin/propose/s`, and a branch `spec/c` adding `changes/c/change.md` with `from: backlog/q` and `resolves: [r]`, where `backlog/q.md`, `backlog/r.md` and `backlog/s.md` are on `origin/main` and the branch deletes none of them
- WHEN `clm land spec/c` runs
- THEN it prints `landed <sha> spec`, `origin` has neither `propose/q` nor `propose/r` and still has `propose/s`, and stderr holds `released propose/q` and `released propose/r`

#### Scenario: moved claim kept
- GIVEN the same branch and a check command that pushes a new commit to `origin/propose/q` while the land runs
- WHEN `clm land spec/c` runs
- THEN `origin/propose/q` is still that commit, it printed a warning naming `propose/q`, and it exits 0

#### Scenario: an edit of a landed change releases nothing
- GIVEN `changes/c/change.md` with `from: backlog/q` on `origin/main`, `origin/propose/q`, and a branch `spec/d` that only modifies `changes/c/change.md`
- WHEN `clm land spec/d` runs
- THEN it lands, and `origin/propose/q` is unchanged

#### Scenario: a code land releases nothing
- GIVEN `origin/propose/q`, `changes/c/change.md` naming `backlog/q` in `from` already on `origin/main`, and a branch `impl/a` that changes only `crates/a/src/lib.rs`
- WHEN `clm land impl/a` lands it
- THEN `origin/propose/q` is unchanged

#### Scenario: a change archived in its branch
- GIVEN `origin/propose/r`, and a branch `d` adding `changes/archive/d/change.md` with `status: done` and `resolves: [r]`, where `backlog/r.md` is on `origin/main`
- WHEN `clm land d` runs
- THEN it lands, `origin` has no `propose/r`, and stderr holds `released propose/r`

#### Needs
- gate.fingerprint/fingerprint
- host/branch-delete-call

#### Serves
- capability/work-concurrently

### Requirement: batch-keeps-its-check
When `main` has moved by the time lands taken together (`batched-lands`) hold the push turn, their `[land] check` and `clm check` having passed on the last of their merge commits `P`, the land that holds the lock SHALL NOT leave them to run that check again. With `main` now at `M`, it SHALL run `clm affected --overlap <P> <M>` (`check.affected/overlap-selection`), merge the same lands in the same order onto `M`, each merged and judged as `batched-lands` says a land taken is, and run `clm check` with its detectors on the last of the new merge commits; when the overlap printed a line, it SHALL first run `[land] check` once on that commit, with `CLM_SELECTION` naming a file holding that output unchanged. It SHALL print on its stderr the `re-check:` line `overlap-recheck` gives for what the overlap printed. When these pass, it SHALL push the new commits in one push and append the records `batched-lands` asks for, except that:
- the `checked` record for the new last commit's code key has a `basis` naming `P`'s code key and `M`'s (`ledger/composed-check`), and none is appended when that key already counts as checked;
- each `landed` record says `recheck: overlap` when the overlap's `[land] check` ran in any round, with `recheck_files` counted as `overlap-recheck` counts them for the last round in which it ran, and `recheck: none` otherwise;
- the record of the first land carries the `check_seconds` and `steps` of the run that passed on `P`, with the seconds of each overlap's run added and its steps after them, so the time of every run of the check is on one record (`step-times`), and the time the holder reports against the target is that sum (`check-time-report`).

Each waiting `clm land` SHALL then end as `batched-lands` says. The lands taken SHALL instead push nothing and go on as lands that were never taken together, the holder landing alone on a new attempt under the lock it holds and each other waiting for the lock as a land never taken does, when any of these holds: no `checked` record with the check's command names `M`'s code key; `clm affected --overlap` fails; merged onto `M`, one of the lands is one `batched-lands` would leave out, a land whose check environment differs from the one the check ran under included. When the push of the new commits is refused because `main` moved again, it SHALL do the same again, `P` then being the commit it last checked, and after ten such rounds the lands SHALL go on as lands never taken together. When the overlap's `[land] check` or the `clm check` on the new last commit fails, the lands are a failed batch (`failed-batch`) in which every land taken is a suspect.

#### Scenario: main moved during the check, on an unrelated component
- GIVEN components `a`, `b` and `d` with no `depends_on` between them, branches `impl/a` and `impl/b` with no `checked` records waiting for the land lock in that order, a `[land] check` that appends `start` to a log outside the repository and waits for a file to appear, and a branch whose head changes only `crates/d/src/lib.rs` and whose land runs no check
- WHEN the holder releases the lock, `impl/a`'s `clm land` is in the check, the other branch lands, and the file appears
- THEN the log holds one `start`, `main`'s first-parent history ends with the other branch's merge commit, `Merge impl/a into main` and `Merge impl/b into main`, each `clm land` printed `landed` with its own commit, `impl/a`'s stderr holds `re-check: none`, both `landed` records hold `recheck: none` and the same `code_key`, only `impl/a`'s holds `check_seconds`, the `checked` record for that key has a `basis` naming the code key of the commit the check ran on and that of the `main` they landed on, and `clm ledger verify` prints nothing

#### Scenario: main moved during the check, with a dependant in common
- GIVEN the same, a component `c` with `depends_on: [a, d]` and a test `crates/c/tests/t.rs`, and a `[land] check` that also copies the file `CLM_SELECTION` names on each run
- WHEN the lands run as above
- THEN both land in one push, the check ran twice, the second copy holds `file crates/c/tests/t.rs` and no file of `a`, `b` or `d`, `impl/a`'s stderr holds `re-check: overlap, 1 files`, both `landed` records hold `recheck: overlap` and `recheck_files: 1`, and only `impl/a`'s holds `check_seconds` and `steps`

#### Scenario: a land that conflicts with the new main
- GIVEN the first scenario's repository, where the other branch also changes the same line of a file `main` holds as `impl/b` does
- WHEN the lands run as above
- THEN the push of the two together never happens, `impl/a` lands alone after a second run of the check, and `impl/b`'s `clm land` exits 1 telling the caller to run `clm rebase`

#### Scenario: main's new commit was never checked
- GIVEN the first scenario's repository, with `main` moved during the check by a commit pushed outside `clm land`, whose code key no `checked` record names
- WHEN the file appears
- THEN `impl/a` and `impl/b` land, each alone, the log holds three `start` lines, and no stderr holds a `re-check:` line

#### Scenario: the overlap's check fails
- GIVEN the second scenario's repository, with a `[land] check` that exits 1 on its second run and passes on every other
- WHEN the lands run as above
- THEN the second run pushes nothing, `impl/a`'s stderr holds the line saying the check of the lands taken together failed, and `impl/a` and `impl/b` then land, each alone, the log holding four `start` lines

#### Scenario: an overlap that ran stands
- GIVEN the second scenario's repository, where `main` moves once more, by a land changing only `crates/d/src/other.rs` whose code key has a `checked` record and whose overlap with the chain selects nothing, while the overlap's `[land] check` runs
- WHEN the lands run as above
- THEN both land in one push, and both `landed` records hold `recheck: overlap` and `recheck_files: 1`

#### Serves
- capability/work-concurrently
- capability/land-safely

### Requirement: commands-in-crate
`clm-land` SHALL offer each of `clm land` and `clm reconnect` as a call of its own that takes the command's arguments as the command line gives them, the repository root its caller found, which the command's `--root` overrides, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. Given `--help` it SHALL write the command's help to the output stream, nothing to the error stream, and return 0: its first line the description, `` Land a branch on `main`: gate it, merge it, check the merge, push it and record the land in the ledger; print `landed <sha> <kind>` `` for `clm land` and `` Push a `main` landed with no remote, and the local ledgers, to a newly added `origin` `` for `clm reconnect`, with no full stop after it, then an empty line, then `Usage: clm land [OPTIONS] [BRANCH]` or `Usage: clm reconnect [OPTIONS]`. Every warning or note a land, a `--check-only` run or a reconnect gives, all of those the other requirements of `land` state among them, SHALL be written on the error stream once, with the one prefix `clm land: warning: ` (`clm reconnect: warning: ` for a reconnect): as it happens, during the run, for the two that concern a command it is running, a check's log that can't be created (`check-log`) and a `clm affected` that fails (`selected-tests`, `overlap-recheck`); and after the run, whatever its outcome and before a refusal's own line `clm land: <reason>`, for every other. The waiting lines (`lock-queue`, `push-turn`) and the output of the commands it runs, the gate, `clm affected`, `[land] check` and `clm check`, SHALL be copied to the error stream as they come, with no prefix.

#### Scenario: a command called in a test
- GIVEN a fixture repository
- WHEN it calls `clm-land`'s call with the arguments of `clm land` and two buffers
- THEN the buffers hold what `clm land` prints to standard output and to standard error there, and the call returns the status `clm land` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm land` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on

#### Scenario: its help
- GIVEN the calls of `clm land` and `clm reconnect`
- WHEN each is given `--help`
- THEN each returns 0 with nothing in the error buffer, and the output buffer's first line is its description with no full stop, its second empty and its third `Usage: clm land [OPTIONS] [BRANCH]`, or `Usage: clm reconnect [OPTIONS]`

#### Scenario: warnings
- GIVEN in turn: a passing land whose effort can't be read; a passing land whose `CLM_AFFECTED` names a command that exits 1; three lands taken together whose check fails on the holder's own branch, as in `failed-batch`'s *the holder's own branch breaks the check*
- WHEN each land runs, the holder's `clm land` in the third
- THEN the first's error stream holds `clm land: warning: effort not recorded: ` once; the second's holds `clm land: warning: clm affected failed: ` before its check's output; and the holder's error stream holds the line saying the check of the lands taken together failed, starting `clm land: warning: ` once and with no second `clm land: ` in it, before its last line, the refusal starting `clm land: `

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

### Requirement: approval-policy
Under `approval = "other"`, `clm land` SHALL land a branch that needs review only when `land.approval/approvals-counted` counts an approval of the head's fingerprint, and SHALL exit 1 with `gate/approval` naming the policy when the only approvals are the branch's authors' own. Under `approval = "any"` it SHALL land on any `approved` record for the fingerprint. When it refuses a land for `gate/approval` it SHALL print, after the gate's lines: under `approval = "other"`, `approval = "other" from <source>: only an approval by someone who isn't an author of the branch counts` on a line of its own, `<source>` being where `land.approval/policy-at-base` says the policy came from, whether approvals were refused or there were none; then, when approvals of the fingerprint exist and none counts, `no approval of this diff counts:` and one line per approval with its writer, its time and why it doesn't count, as `land.approval/approvals-counted` gives them; and last ``ask another person to run `clm approve <branch>` `` when an approval was refused under `approval = "other"`, and otherwise ``ask the human to run `clm approve <branch>` ``.

#### Scenario: self-approval refused
- GIVEN `approval = "other"`, a branch that needs review whose commits are authored by `Christopher Lunny`, and an `approved` record by writer `christopher-lunny/1a2b3c4d`
- WHEN `clm land` runs
- THEN it exits 1 naming `approval = "other"` and `main` is unchanged

#### Scenario: another person approves
- GIVEN the same, with an `approved` record by writer `dana/9f9f9f9f`
- WHEN `clm land` runs
- THEN it lands

#### Scenario: any
- GIVEN `approval = "any"` and the self-approval of the first scenario
- WHEN `clm land` runs
- THEN it lands

#### Scenario: why no approval counts
- GIVEN `approval = "other"` in `origin/main:clm.toml`, a `spec/x` that needs review whose commits are authored by `Christopher Lunny`, and an `approved` record for its fingerprint by `christopher-lunny/1a2b3c4d`; and then the same branch with `approval = "any"` and no `approved` record
- WHEN `clm land spec/x` runs in each
- THEN the first exits 1 printing `approval = "other" from origin/main:clm.toml` on a line of its own, `no approval of this diff counts:`, a line starting `christopher-lunny/1a2b3c4d at ` that says its writer is one of the branch's authors, and ``ask another person to run `clm approve spec/x` ``; the second exits 1 printing no `approval = "other"` line and ``ask the human to run `clm approve spec/x` ``; and `main` is unchanged in both

#### Serves
- capability/approve-intent
- architecture/approval-policy

#### Needs
- ledger/person-slug
- land.approval/approvals-counted

### Requirement: approval-from-base
`clm land` SHALL count approvals under the policy `land.approval/policy-at-base` gives for the base it lands on, the `origin/main` it fetched, on every host, and SHALL never take it from the `clm.toml` of the root it runs in, of the head or of the merged tree. When that call returns its error, it SHALL exit 2 with it, naming `<base>:clm.toml` and `approval`, before running any check or pushing or appending anything.

#### Scenario: a branch that weakens the policy is judged by the base
- GIVEN `origin/main` whose `clm.toml` sets `approval = "other"`, a `spec/x` that needs review authored by `Christopher Lunny` whose head sets `approval = "any"`, an `approved` record by writer `christopher-lunny/1a2b3c4d`, and no `clm.local.toml`
- WHEN `clm land spec/x` runs from a checkout of `spec/x`
- THEN it exits 1 naming `approval = "other"` from `origin/main:clm.toml`, and `main` is unchanged

#### Scenario: a branch that tightens the policy is judged by the base
- GIVEN `origin/main` whose `clm.toml` sets no `approval`, the same `spec/x` whose head sets `approval = "other"`, and the same `approved` record
- WHEN `clm land spec/x` runs from a checkout of `spec/x`
- THEN it lands

#### Scenario: the clone's local file wins
- GIVEN `origin/main` whose `clm.toml` sets `approval = "other"`, the clone's `clm.local.toml` setting `approval = "any"`, and the self-approval of the first scenario
- WHEN `clm land spec/x` runs
- THEN it lands

#### Scenario: unreadable policy at the base
- GIVEN `origin/main` whose `clm.toml` sets `approval = "everyone"`, and a `spec/x` whose `clm.toml` sets `approval = "any"`, with an `approved` record
- WHEN `clm land spec/x` runs from a checkout of `spec/x`
- THEN it exits 2 naming `origin/main:clm.toml` and `approval`, runs no check, and `main` and the ledger are unchanged

#### Needs
- land.approval/policy-at-base

#### Serves
- capability/approve-intent
- architecture/approval-policy

### Requirement: signed-land
When `origin/main`'s tree holds `docs/allowed_signers`, `clm land` SHALL count an approval only as `land.approval/approvals-counted` counts it in a signed repository; it SHALL exit 1, before running any check or pushing, naming `user.signingkey`, when `land.approval/key-and-signing`'s key check refuses; and it SHALL append a `landed` record signed under `clm-land`, signed before the push, pushing nothing and exiting 1 when signing fails. Without that file it SHALL neither check signatures nor sign.

#### Scenario: unsigned approval refused
- GIVEN a signed repository, a `spec/x` that needs review, a lander who can sign, and only an `approved` record for its fingerprint with no `sig`
- WHEN `clm land spec/x` runs
- THEN it exits 1 with `gate/approval` saying the approval isn't signed, and `main` is unchanged

#### Scenario: forged name refused
- GIVEN a signed repository whose `docs/allowed_signers` lists keys for `christopher-lunny` and `dana`, `approval = "other"`, a `spec/x` that needs review authored by `Christopher Lunny`, a lander `christopher-lunny` who can sign, and an `approved` record by writer `dana/1a2b3c4d` signed with `christopher-lunny`'s key
- WHEN `clm land spec/x` runs
- THEN it exits 1 and `main` is unchanged

#### Scenario: lander approved it themselves
- GIVEN the same file and policy, a `spec/x` that needs review whose commits are authored as `Zed`, a lander whose writer person is `christopher-lunny`, and an `approved` record signed by `christopher-lunny`
- WHEN `clm land spec/x` runs
- THEN it exits 1 naming `approval = "other"` and `main` is unchanged

#### Scenario: signed land
- GIVEN the `spec/x` that needs review of *forged name refused*, with an `approved` record by `dana` signed with `dana`'s key, landed by `christopher-lunny` who can sign
- WHEN `clm land spec/x` runs
- THEN it lands, and its `landed` record's `sig` is accepted by `signed_by` under `clm-land`

#### Scenario: lander can't sign
- GIVEN a signed repository, a code branch `impl/a`, and a lander with no `user.signingkey`
- WHEN `clm land impl/a` runs
- THEN it exits 1 naming `user.signingkey` before running the check command, and `main` and the ledger are unchanged

#### Scenario: signing fails
- GIVEN a signed repository, a code branch `impl/a` with a passing change, and a lander whose `gpg.format` is `ssh` and whose `user.signingkey` names a readable file that `ssh-keygen` can't sign with
- WHEN `clm land impl/a` runs
- THEN it exits 1 naming `user.signingkey`, `main` on `origin` is unchanged, and the ledger has no `landed` record for it

#### Serves
- capability/approve-intent
- architecture/signed-ledger-records

#### Needs
- ledger/record-signing
- ledger/signature-check
- land.approval/key-and-signing

### Requirement: changes-requested-holds
`clm land` SHALL exit 1, pushing nothing, appending nothing and running no gate process and no check, when `land.approval/request-refusal` gives a refusal for the branch at the head's fingerprint, a request holding it unless `ledger/request-cleared` counts it cleared, whether or not the approval that would clear it is one the policy counts, printing that refusal on stderr. It SHALL count such a request whether or not the request's record is signed, and in a repository signed at the fetched base SHALL count as clearing it only an approval whose signature checks there, asking `land.approval/request-refusal` with that base. It SHALL go on as `land` states when every request for the branch names another fingerprint, or its writer's person approved the fingerprint with the request named.

#### Scenario: held
- GIVEN a branch `spec/x` whose fingerprint is `f`, an `approved` record for `f` by `bob/web`, a `changes_requested` record for `spec/x` at `f` by `alice/web` with the text "Name the rule", and a `commented` record on `spec/x` at `f` with target `n/r`
- WHEN `clm land spec/x` runs
- THEN it exits 1 printing `land/changes-requested` naming `alice/web` and `Name the rule`, then `land/comment` naming `n/r`, and `clm approve spec/x`; no check ran, and `main` on `origin` is unchanged

#### Scenario: cleared by a push
- GIVEN the same, after the author pushed a commit that changes a spec file, so the fingerprint is `g`, and an `approved` record for `g`
- WHEN `clm land spec/x` runs
- THEN it lands

#### Scenario: cleared by the requester
- GIVEN the held branch, and an `approved` record for `f` by `alice/1a2b3c4d` that names the request as one it clears
- WHEN `clm land spec/x` runs
- THEN it lands

#### Scenario: another person's approval doesn't clear it
- GIVEN the held branch with `approval = "any"`, and a later `approved` record for `f` by `dana/9f9f9f9f`
- WHEN `clm land spec/x` runs
- THEN it exits 1 printing `land/changes-requested`

#### Scenario: unsigned request in a signed repository
- GIVEN a `main` whose tip holds `docs/allowed_signers`, a branch `spec/x` at fingerprint `f` with a signed `approved` record by `dana/9f9f9f9f` that counts, and an unsigned `changes_requested` record for `spec/x` at `f` by `alice/web`
- WHEN `clm land spec/x` runs
- THEN it exits 1 printing `land/changes-requested` naming `alice/web`

#### Scenario: an unsigned clearing in a signed repository
- GIVEN the repository of *unsigned request in a signed repository*, and an unsigned `approved` record for `f` by `alice/1a2b3c4d` that names the request as one it clears
- WHEN `clm land spec/x` runs
- THEN it exits 1 printing `land/changes-requested` naming `alice/web`

#### Serves
- capability/follow-from-browser
- capability/approve-intent
- architecture/humans-review-intent

#### Needs
- ledger/feedback-records
- ledger/request-cleared
- land.approval/request-refusal

### Requirement: deleted-tests-approved
`clm land <branch>`, whatever the branch's name, SHALL exit 1 when `check.coverage`'s `deleted_tests`, over the texts at the merge base and at the head of the files of the two trees under the CLM root that can hold a test (`check.coverage/test-holding-paths`), named from it, returns a test no approved list names, printing for each `<file>: land/deleted-tests-approved: test <name>, which cited <id>[, <id>...], is gone and no approved list names it. Keep the test under its name, or under a name no citing test carried, in its file, citing every requirement it cited, or leave the requirement unbuilt and add a question item`; the files that can hold no test hold none on either side, so this is the verdict `clm coverage --deleted-tests` gives for the branch. A branch whose diff `rename/replay` accepts from the merge base, asked as the gate asks it of a head that adds exactly one change whose `change.md` sets `renames`, SHALL NOT be judged, whichever paths its rewritten markers sit under, since those markers are what the replay vouches for; every other branch SHALL be, one the gate reads as `spec` for another reason, such as a `clm.toml` change `gate/approval-setting` counts as a spec change, included. An *approved list* is a `changes/<id>/deleted-tests.txt` or `changes/archive/<id>/deleted-tests.txt` in the fetched base's tree, and it names a test by a list line (`gate/cuts`, read with its `list_lines`) whose file is the gone test's and whose name agrees with it as `bare_name` gives them. It SHALL do so before it takes the land's lock, merges or runs any check, and it SHALL push and append nothing. When asking `rename/replay` fails, as when git fails, it SHALL exit 2 with its error, before the lock. A head with no merge base with the base, whose land its merge refuses anyway, and a head whose diff touches no file under the CLM root that can hold a test, SHALL NOT be refused by this rule; a file whose text isn't UTF-8 SHALL be read as holding no test, as `clm coverage --deleted-tests` reads it. `clm land --check-only` SHALL judge `HEAD` against the local `origin/main` as a land judges its head, reading no branch name, print the same lines and exit 1 before it runs the check command; with no local `origin/main` it SHALL judge nothing.

#### Scenario: a deleted test
- GIVEN `main` holding `tests/t.rs` with tests `one` and `two`, each citing `a/x`, a branch `impl/a` whose `tests/t.rs` holds only `one`, and a check command that records that it ran
- WHEN `clm land impl/a` runs
- THEN it exits 1 naming `tests/t.rs`, `two` and `a/x` under `land/deleted-tests-approved`, the check command never ran, and `main` on `origin` and the ledger are unchanged

#### Scenario: a test an approved list names
- GIVEN the same, `main` also holding `changes/c/deleted-tests.txt` with the line `DELETE tests/t.rs::two`
- WHEN `clm land impl/a` runs
- THEN it lands

#### Scenario: a list in the archive
- GIVEN the same, the list at `changes/archive/c/deleted-tests.txt` instead
- WHEN `clm land impl/a` runs
- THEN it lands

#### Scenario: a moved test
- GIVEN the first scenario's `main`, and a branch `impl/a` that moves `two`, under its name and with its marker, into a new file `tests/u.rs`
- WHEN `clm land impl/a` runs
- THEN it lands

#### Scenario: a renamed test
- GIVEN the first scenario's `main`, and a branch `tidy` that renames `two` to `second`, keeping its marker
- WHEN `clm land tidy` runs
- THEN it lands

#### Scenario: a renamed test that drops its citation
- GIVEN the first scenario's `main`, and a branch `tidy` that renames `two` to `second` and takes its marker off
- WHEN `clm land tidy` runs
- THEN it exits 1 naming `two` under `land/deleted-tests-approved`

#### Scenario: a new name another file carries
- GIVEN the first scenario's `main`, also holding `tests/u.rs` with a test `kept` citing `a/x`, and a branch `tidy` that renames `two` to `kept` in `tests/t.rs`, keeping its marker, and leaves `tests/u.rs` as it is
- WHEN `clm land tidy` runs
- THEN it exits 1 naming `two` under `land/deleted-tests-approved`, as `clm coverage --deleted-tests` lists it

#### Scenario: a spec branch
- GIVEN a branch `spec/r` holding only the committed result of `clm rename r`, which rewrote the marker in the title of a test in `a.test.ts`
- WHEN `clm land spec/r` runs
- THEN this rule refuses nothing

#### Scenario: the pre-check
- GIVEN the first scenario's branch checked out
- WHEN `clm land --check-only` runs
- THEN it prints the same line, exits 1, and the check command never ran

#### Scenario: a spec branch that deletes a test
- GIVEN `main` holding `docs/t.test.ts` with tests `one` and `two`, each citing `a/x` in its title and `clm.toml` setting `approval = "other"`; a branch `spec/s` whose `docs/t.test.ts` holds only `one`; and a branch `t` that does the same and also sets `approval = "any"`
- WHEN `clm land spec/s` runs, and `clm land t`, and `clm land --check-only` in a checkout of `spec/s`
- THEN each exits 1 naming `docs/t.test.ts` and `two` under `land/deleted-tests-approved`, the last before its check command runs

#### Scenario: what it doesn't read
- GIVEN in turn: a branch changing only `README.md` and a binary `assets/logo.png`; and `main` holding a test file that isn't UTF-8, holding a test citing `a/x`, which a branch `tidy` deletes
- WHEN `clm land` runs for each
- THEN this rule refuses neither

#### Serves
- architecture/removal-needs-a-yes
- capability/land-safely

#### Needs
- check.coverage/test-lists
- check.coverage/test-holding-paths
- gate/cuts
- gate/files-outside-spec-paths
- rename/replay

### Requirement: gate-rules-trailer
Every merge commit `clm land` pushes to `main`, for a land alone or for lands taken together (`batched-lands`), and not the provisional commit a batch judges its members on, SHALL end its message with the line `Gate-Rules: <n>`, `n` being `clm-gate`'s `RULES`, in the trailer block that holds any `Change:` or `Checked:` line it writes, after them, so that `git log`'s `%(trailers:key=Gate-Rules,valueonly)` reads `n` from it.

#### Serves
- architecture/signed-ledger-records
- architecture/humans-review-intent

#### Needs
- gate/rules-version

#### Scenario: a code land
- GIVEN a branch `impl/a` that only changes `src/x.rs`
- WHEN `clm land impl/a` lands it
- THEN the merge commit's message is `Merge impl/a into main`, a blank line and `Gate-Rules: <RULES>`, and `git log -1 --format='%(trailers:key=Gate-Rules,valueonly)'` on it prints `RULES`

#### Scenario: a spec land that adds a change
- GIVEN a branch `spec/x` that adds the change `x`
- WHEN `clm land spec/x` lands it
- THEN the merge commit's message ends with the lines `Change: x` and `Gate-Rules: <RULES>`, in that order

#### Scenario: lands taken together
- GIVEN two lands of `impl/a` and `impl/b` taken together on one check
- WHEN they land
- THEN each merge commit's message ends with its `Checked:` line and then `Gate-Rules: <RULES>`

### Requirement: runner-names-its-caller
`clm-land` SHALL offer the shared check runner (`shared-check-runner`) only in the form that takes the name of the command that runs it, `run_check_as`, so no caller's warning names another command: no call that runs it under a name it doesn't give SHALL be public.

#### Scenario: the unnamed form is gone
- GIVEN a crate that calls `clm_land::run_check` with a command, a directory, no selection, an empty `CheckEnv` and no log
- WHEN it is compiled
- THEN it fails to compile, while the same call through `clm_land::run_check_as` with a name compiles

#### Needs
- verify/verify-run

#### Serves
- capability/slow-tests-after-land

### Requirement: already-on-main
A `clm land` of a branch whose head `origin/main` already contains SHALL merge nothing, push nothing and append no `landed` record, whether `main` held the head when the land began or came to hold it while the land waited for the land lock, the check lock or its push turn, or ran its check: it SHALL ask whether `origin/main` contains the head each time it reads `origin/main` to build or push a merge on. When a commit on `main`'s first-parent chain merges the head as its second parent and a `landed` record of the branch names that commit, it SHALL print that record's `landed <commit> <kind>` line, `<kind>` the word the record gives, `light` included, remove what a land removes (`landed-branch-cleanup`, `land-frees-build-target`) and exit 0, as a waiting land taken with others does (`batched-lands`); when no `landed` record names that commit, `unrecorded-land-finished` SHALL apply. For any other head `main` contains, one on its first-parent chain or one another branch's land brought there, it SHALL exit 1 saying the branch is already on `main` and naming the oldest commit of `main`'s first-parent chain that contains the head. Lands taken together SHALL leave out a waiting land whose head the commit it would merge onto contains, which then does on its own turn what this rule says.

#### Scenario: a branch landed before
- GIVEN `impl/a` landed by `clm land impl/a`, which printed `landed <m> code`, and the local branch `impl/a` made again at the head it landed
- WHEN `clm land impl/a` runs again
- THEN it prints `landed <m> code` and exits 0, `main` on `origin` is still `<m>`, the check command ran once in all, the ledger holds one `landed` record for `impl/a`, and the clone has no local `impl/a`

#### Scenario: landed while it checked
- GIVEN a passing branch `impl/a` pushed to `origin`, and a `[land] check` that, the first time it runs, lands `impl/a` from a second clone of `origin` with that clone's own `clm land` and then passes
- WHEN `clm land impl/a` runs in the first clone
- THEN it prints `landed` with the merge commit the second clone made and exits 0, `main` on `origin` is that commit, and the ledger holds one `landed` record for `impl/a`

#### Scenario: a head main holds another way
- GIVEN `impl/b`, merged into `impl/c` before `impl/c` landed as `<m>`; and `fix-x`, a branch at `origin/main`'s tip `<t>`
- WHEN `clm land impl/b` and `clm land fix-x` run
- THEN the first exits 1 saying `impl/b` is already on `main` and naming `<m>`, the second exits 1 saying `fix-x` is already on `main` and naming `<t>`, `main` on `origin` is unchanged, and neither appends a record

#### Scenario: a waiting land main already holds
- GIVEN `impl/a` and then `impl/b` waiting for the land lock while another process holds it, and `impl/b` landed meanwhile from a second clone of `origin` as `<m>`
- WHEN the holder releases the lock
- THEN `impl/a` lands with a merge commit whose first parent is `<m>`, `impl/b`'s `clm land` prints `landed <m> code` and exits 0, and `main`'s first-parent history holds one merge of `impl/b`
