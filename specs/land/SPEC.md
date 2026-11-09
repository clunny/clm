---
title: Land
status: active
owners: ["@clunny"]
depends_on: [ledger, host, gate, check, check.affected, config, vcs, model, apply, effort, land.approval, sync.regenerate, orchestrator]
paths: ["crates/clm-land/**"]
successor: ""
serves: [capability/land-safely, architecture/land-locally]
---

## Contract
Landing a branch on `main`, on the machine that runs the command (`land-locally`). `clm land [<branch>]` runs the gate against `origin/main`, merges the branch in a scratch tree, runs the project's check command, handing it the tests the change can break as `clm affected` selects them (`CLM_SELECTION`, `graph-test-selection`), and `clm check` on the result unless the ledger says that code was already checked, records what it did in the ledger, and pushes the merge commit to `main` without force, retrying when `main` moved. It is the only thing in CLM that pushes `main`. What it keeps in a clone, the land tree, the shared target `.clm/land`, its lock, the check lock `.clm/check.lock`, the two locks' queues and failing checks' logs, is under the `.clm/` of the main worktree's CLM root, which `clm init` has the repository ignore, so a land leaves nothing untracked whatever the project's language (`land-places-ignored`). It prints `landed <sha> <kind>`, `<kind>` being `code`, `spec` or `spec review` as the gate's first line gives it, and exits 0; it exits 1 with the reason when it pushed nothing, and 2 on a git failure. Whether the branch is approved or held by a request for changes, and the signing of approvals and lands, are `land.approval`'s, as is `clm approve`, which `clm-land` re-exports. There is no after-merge review: a land that needs review lands only once approved (`yes-no-review`). `clm reconnect` pushes `main` and the local ledgers to an `origin` that `main` fast-forwards, as when a repository with `host = "none"` is moved onto a real remote or one landed with no remote by an older `clm` gets one. `clm land --check-only` runs the same check command, with every test (no `CLM_SELECTION`), and `clm check` on the current working tree, so a session learns the land check's verdict through a command that names no path (`land-check-command`). It lands nothing; when both pass on a clean working tree whose committed `HEAD` contains `origin/main`, it appends a `checked` record for `HEAD`'s code key, so a land whose merged tree is that tree, `main` not having moved, runs no check of its own (`worktree-check-counts`). The check command of a land, and of a `--check-only` run outside a build place, waits for any other such check of the same clone to end first (`check-runs-one-at-a-time`), so either can take as long as the checks queued before it; the waiting runs take their turns in the order they began to wait, except that a land carrying `P1` work goes first, a land whose code is already checked and runs only the tests that read what it changed goes next, and the rest go by priority (`lock-queue`, `ordered-lock-queue`); a `--check-only` run in a build place waits for none, its tests kept apart from other runs' by its slot (`tests-run-concurrently`). Both run it with `CLM_SLOT` set to the slot of the build place that names the branch checked, as `orchestrator`'s `place_slot` gives it, or 0, so the project's tests key their ports and paths apart from those of the tests other builds run at the same time (`build-slots`, `tests-run-concurrently`). While `origin/main`'s backlog holds a `kind: bug` item with `verify_failures`, a slow test the human took in as broken after the land, `clm land` refuses a code land that changes a file of one of the item's components, unless the branch's commit resolves that item; it warns, and lands, a branch touching the components of a red `verified` result no item has taken in yet (`verify-stop`, `verify-after-land`). A host's run never holds a land by itself (`land-locally`). `clm-land` also offers the runner it runs `[land] check` through, under the clone's check lock, for `verify` to run `[verify] check` the same way (`shared-check-runner`). When a land runs the check command it times the command and each step the check marks, and keeps the times, with the files that selected each step, on the `landed` record it appends (`step-times`); `--check-only` times nothing. After the push of such a land it prints the check's time against `[land] check_target_seconds` and, over it, its costliest steps, the chain that selected each and how recent lands fared, so an agent can judge whether the selection reaches too far or a suite belongs after the land (`check-time-report`); it files nothing, and the report never changes the land's outcome.

## Design
Crate `clm-land`, called from `crates/clm`'s `land`, `approve` (re-exported from `clm-approval`) and `reconnect` subcommands. Every git call goes through `clm-git`, the remote's name taken from `REMOTE` (`git-plumbing-layer`).

**`clm land [<branch>]`**, default the current branch, refused on `main`. Each attempt, up to ten:
1. Fetch `origin` (`main`, the branch, `clm/ledger/*`) and fold the ledger. Take the head, its fingerprint and change IDs from `land.approval`. In a signed repository its key check comes first, then its hold check, then whether the fingerprint is approved under the policy at the fetched base: all before the gate, the lock, the merge and the checks.
2. Run the gate on the head against `origin/main` in the tree this attempt checks in (**Places**, below), adding `--approved` when approved; for a head **Behind in `specs/`** (below), on the merge commit instead, once step 4 has made it. A violation prints as the gate printed it and exits 1; `gate/approval` tells the caller to ask the human for `clm approve <branch>`.
3. `git merge-tree --write-tree origin/main <head>`. A conflict, or `specs/` changed on both sides, exits 1 telling the caller to run `clm rebase`, except for a head **Behind in `specs/`** (below), whose merged `specs/` the land regenerates.
4. The merge commit, `git commit-tree <merged tree> -p origin/main -p <head>`, message `Merge <branch> into main`, with `Change: <id>` on a following line for a spec land that adds or modifies live changes (the first by ID), committed as the repository's git user. A head that already contains `main` is still merged with a merge commit, so a claim's tip reaches `main` only through its land (`merged-claims-released`). The commit is checked out detached into the tree, so the gate and the checks run on a commit. Then **Derived specs** (below).
5. The merged tree's code key (`ledger`). When a `checked` record names it with the same command, it asks **Readers of checked code** (below): when they select no file, only `clm check --no-detectors` runs; when they select some, `[land] check` runs as below with their selection and then `clm check --no-detectors`, and no second `checked` record is appended. When none names it and the CLM root's `clm.toml` differs between the fetched base and the merge commit only in what `clm-gate`'s `settings_unchanged` leaves out (`upgraded`, `declined_migrations`, comments and formatting), it looks up the *settings-free key* instead, in the same place the attempt computes the code key, before the lock (**Places**), so a land it counts as checked takes no lock unless its readers select a file, `ledger`'s `code_key_with(root, <merge>, <prefix>clm.toml, <base>)`: when a `checked` record names that with the same command, the land goes on as one whose code key is checked, as above, and once `clm check --no-detectors` (and the readers' check, when they selected a file) passes it appends a `checked` record for the merged tree's own code key, so the `landed` record names a checked key and a retry finds it (`settings-unchanged-is-checked`); otherwise **Selected tests** (below), `[land] check` through `sh -c` with `CI` unset (so a script that installs a browser only in CI doesn't) and `CARGO_TARGET_DIR` set to `<home>/.clm/land` unless the caller set it non-empty, and then `CARGO_INCREMENTAL=0` unless the caller set that non-empty (`shared-land-target`), then `clm check`, and on success a `checked` record. `<main>` is the first worktree `git worktree list --porcelain` lists and `<home>` the CLM root in it, `<main>/<prefix>` with `<prefix>` what `git rev-parse --show-prefix` prints in the CLM root, computed once; a land from a build's worktree under `.claude/worktrees/` so shares one target (`build-places`). The gate then runs again on the merge commit, since a file both sides touched can change whether it needs review, and in a signed repository `land.approval` signs the `landed` record.
6. `git push origin <merge>:refs/heads/main --force-with-lease=refs/heads/main:<the base step 1 fetched>`. A refusal isn't always `main` moving (`land-push-retry`), so it fetches `origin`'s `main` again, directly rather than through step 1's offline refusal, and judges it: the merge commit means the push went through; neither the merge commit nor the base means `main` moved and the next attempt starts, its `checked` record standing, so a retry with the same code key runs only `clm check --no-detectors`; still the base means a refusal for another reason, such as `[remote rejected] main (failure)` while other sessions push, and the same commit is pushed again with the same lease at most twice more, after fixed waits of 1 and then 2 seconds with no override, these pushes not counting as attempts. Still the base after the last is exit 2 with git's message, including the remote's lines (the push drops `--quiet` if it hides them), and a line saying the failure may be transient and that running `clm land <branch>` again is safe. When the refetch fails the push may have gone through, so it exits 2 with the push's message alone. Ten moved attempts exit 1 saying `main` keeps moving.
7. Append, in one commit (`ledger/append-all`), the `effort` records `clm effort record --branch <branch>` would append for the branch's sessions on this machine, read once before the first attempt so neither the reading nor a second ledger push lengthens the time `main` has moved with no `landed` record (`effort-accounted`; the reader's warnings print prefixed `effort: `, and when it can't read at all it warns `effort not recorded: <why>`), then `landed`: commit, branch, `land_kind` and `review` from the gate's first line, fingerprint, the step-5 code key as `code_key`, the change IDs. When `append` reports `landed` unpushed, it warns `the landed record isn't on origin yet: git push origin clm/ledger/<person>/<clone>` and still exits 0, since `main` has moved. Then `host`'s `branch delete <branch> --lease <head>` for an `impl/*`, `spec/*` or `propose/*` branch (a failure warns), the local branch too when it is checked out nowhere (`clm wrap-up` tidies one that is), and for `impl/*`, **Freeing a build's disk** (below).
8. Print `landed <sha> <kind>`.

**Derived specs** (`derived-specs-fingerprint`). When the head adds a change (`clm-gate`'s `added_changes` over the merge base's and the head's `ls-tree -r -z` listings, the question `gate.fingerprint/fingerprint` asks), or applies one (a change whose `change.md` parses with status `proposed` or `approved` at the base, `origin/main`'s, and in the merge commit has status `applied`, or is absent while `changes/archive/<id>/change.md` has status `done`, as `sync`'s Design defines it: `regenerate-applied-live-changes`), the IDs of the added changes whose `change.md` in the merge commit parses with status `applied` (live) or `done` (new archived), and of the applied ones, sorted, go to `sync.regenerate`'s `regenerated` (crate `clm-regenerate`, which reaches no `clm-health`: `clm/health-isolated`) without its stale-delta merge (`sync.regenerate/regenerated-tree`), run with the tree holding the merge commit as `root` and the fetched base's commit SHA as `onto`, so the scratch `specs/`, the copied `changes/` and `backlog/` and the re-apply are `clm rebase`'s own. A stop exits 1 with rule `land/derived-specs`, the lines it returned and `clm rebase`; otherwise the regenerated `specs/` and the merge commit's are compared byte for byte, a symlink by its target, over the union of their paths in byte order, a path on one side only being a difference, and the first difference exits 1 naming it. It runs before step 5, so before the `--no-detectors` run too. The fingerprint skips `specs/` on a head that adds a change, so this is what keeps an approval from covering a hand edit there. A head that applies a change and adds none keeps `specs/` in its fingerprint, which reads listings alone and can't see a status, so a sync that regenerates its `specs/` onto a `main` that changed them asks for its approval again.

**Behind in `specs/`** (`land-regenerates-behind-specs`, `regenerate-dont-merge`). A head that adds or applies a change, as **Derived specs** reads them, is *behind in `specs/`* when a file under `specs/` changed both between the merge base and the base and between the merge base and the head, and every path `git merge-tree --write-tree --name-only origin/main <head>` reports conflicted, if any, lies under `specs/`. Concurrent spec lands make this the usual case: each regenerates the same `specs/<component>/SPEC.md` and the root `specs/SPEC.md`, and a land whose check runs for minutes can find `main` moved by another's when it pushes. Such a head lands without a `clm rebase` round, its next attempt regenerating again, in two regenerates. Whether a head is behind is decided where the attempt computes the merged tree; the code key and the lock decision use the tree merge-tree wrote, since the code key leaves `specs/` out, and both regenerates run in the attempt after that decision. The merge commit's message takes the `Change: <id>` line, since such a head adds or applies a change and so lands as `spec`. First the land proves the head's own `specs/` derived: `regenerated` from the tree on the head as `root`, onto the merge base, for the changes the head adds whose status there is `applied` or `done` and those it applies over the merge base, compared with the head's `specs/` as **Derived specs** compares; a stop or a difference exits 1 with rule `land/derived-specs`, the path or the lines, and `clm rebase`, so a hand edit under `specs/` is refused here as there. Then it regenerates as **Derived specs** does, onto the base, from a commit of the tree merge-tree wrote (whose `specs/` `regenerated` doesn't read, so conflict markers there don't matter), and the merge commit's tree is that tree with `specs/` replaced by the regenerated one; a stop, such as a delta stale against the base, exits 1 as **Derived specs** does, naming `clm rebase`. **Derived specs**' comparison then holds by construction and isn't run. Step 2's gate runs on that merge commit, which contains the base, so `gate/up-to-date` can't fire, and step 5's gate runs on it again as on any land. A conflict outside `specs/`, or a head that adds and applies no change (an `impl/*` branch's status edit), is still refused as step 3 says. The approval stands: the fingerprint skips `specs/` on a head that adds a change, and on one that only applies a change it holds the head's `specs/`, which the first regenerate has proved derived. What lands under `specs/` differs from the head's only by what the base brought, regenerated as `clm rebase` would have made it.

**Places** (`scratch-outside-git`, `land-tree`, `fixed-land-tree`). Cargo names a workspace's outputs by its path and judges them fresh by file times, so two lands that overlap on one target can check each other's code, and a fresh path per land rebuilt everything (43 GB a day on one machine). Before step 2 an attempt computes the merged tree and its code key (`git merge-tree --write-tree` writes only objects); when its check will compile into `<home>/.clm/land`, it takes the exclusive advisory lock on `<home>/.clm/land.lock` (`land-target-lock`, `<home>/.clm/` created when missing, released by the system when a land is killed), printing `waiting for another land's check on <home>/.clm/land: <n> ahead` on stderr when it begins to wait and again each time `<n>` changes, in the order **The queue** (below) sets, with no time limit, and holds it until the push has gone through and the cap is applied, or the attempt ends; the append, the deletes and the print run outside it. **The cap** (`land-target-cap`): right after a push that went through, still under the lock, it sums the lengths of the regular files under `.clm/land` (no symlink followed, so a sparse file counts its length) and, above `[land] target_cap_gb` gigabytes, removes the directory and prints `cleared .clm/land (<n> GB, cap <cap> GB)`; an unreadable entry or a failed removal only warns. Incremental data and units for feature sets no land builds again are what grow there, and Cargo prunes neither, so the cap trades one cold land for a bounded directory, and each attempt takes it afresh. A merge that conflicts, a checked code key whose readers select nothing, and a caller's `CARGO_TARGET_DIR` take no lock. **The check lock** (`check-lock`): whatever the target, the run of `[land] check` itself, by a land or by a `--check-only` run whose `CLM_SLOT` (`check-slot`) is 0, holds a second exclusive advisory lock, on `<home>/.clm/check.lock`, from just before the command is spawned until it exits, taken with the same helper as the land lock (`lock.rs`, generalised to take the lock file's path and the waiting line) and printing exactly `waiting for another check on <home>/.clm/check.lock: <n> ahead` on stderr when it begins to wait and again each time `<n>` changes, in the order **The queue** sets, with no time limit; a lock file that can't be created or locked is exit 2, as the land lock's is. The lock is taken inside the helper both callers share for running `[land] check`, before the output pipe is read, and dropped when it returns, on every path. The project's tests share the host, not only the target: a test that binds a fixed local port fails in whichever of two overlapping checks starts second, for a reason that isn't its branch's. A `--check-only` run in a build place, whose slot isn't 0, takes no lock: its slot keeps its tests apart from every other run's (`tests-run-concurrently`), and builds' pre-land checks, the clone's most frequent checks, queued behind every land's while it took one (`worktree-check-counts`); runs outside a place share slot 0, which keeps nothing apart, so they still take it. The helper is told whether to take the lock; `verify`'s runner (`shared-check-runner`) always takes it. A land takes it after `land.lock`, never before, and `--check-only` takes at most it, so neither can wait on the other in a cycle; `clm affected`, `clm check` and the gate run outside it. A locked attempt checks out in the land tree `<home>/.clm/land-tree` (checks of a root in a subdirectory run in `<home>/.clm/land-tree/<prefix>`): when `git worktree list --porcelain` lists it (both canonicalised) and it exists, `git checkout --quiet --force --detach <commit>` and then `git clean --quiet -fdx`; otherwise, or when either fails, it removes the path, runs `git worktree prune` and `git worktree add --quiet --detach <path> <commit>`, a failed add being exit 2. The tree stays when the attempt ends; `clm land` and `clm wrap-up` never remove it, and removing it by hand starts the next land cold. Any other attempt uses a fresh `land-*` worktree under `TMPDIR` (read through the injected environment lookup) or else `std::env::temp_dir()`, removed on every exit path, never one under `.claude/worktrees/`, and outside the repository, so no tool that refuses paths under `.git/` (Vite's `server.fs.deny`) or walks up to project files trips on it; a killed land's tree stays until the system clears it.

**The queue** (`lock-queue`, `ordered-lock-queue`). The operating system wakes the waiters of an advisory lock in no stated order, so `lock.rs`'s helper puts an ordered queue in front of each lock and keeps the lock itself for exclusion. Each lock `<home>/.clm/<name>.lock` has its queue in the directory `<home>/.clm/<name>.queue/`. A run that finds the lock free and no live ticket in the queue takes the lock at once, printing nothing, with no ticket. Otherwise it takes a ticket: under a short exclusive lock on `<name>.queue/next.lock`, it reads the counter in `<name>.queue/next` (0 when missing), writes it back one higher, creates the ticket file `<counter, zero-padded to 20 digits>-<lane>-p<n>` (`lane` being `priority` or `ordinary`, `p<n>` the run's priority, `p1` to `p5`), takes an exclusive advisory lock on that file, and only then releases `next.lock`. The ticket's own lock is what makes it live: the system drops it when the run's process dies, however it dies, and a run that reads the queue, under `next.lock`, removes each ticket whose lock it can take, so a killed land never holds a turn and no process ID is read. **Turns.** Among the live tickets the turn goes, first, to the `p1` tickets, a `priority` one before an `ordinary` one and otherwise the oldest; then to the `priority` tickets, the highest priority first and then the oldest; then to the `ordinary` tickets in the same order; except that when the queue's `streak` file (in the same directory, written under `next.lock`) says the last six runs to take the lock below `p1` held `priority` tickets while an `ordinary` ticket was waiting, and no `p1` ticket waits, the `ordinary` ticket that would come first among them takes it. A waiter re-reads the queue at most every 250 milliseconds; only while the turn is its own does it try the lock, without blocking, and once it holds the lock it updates `streak` (unchanged when its ticket is `p1`, one higher when its ticket was `priority` and an `ordinary` ticket is waiting, 0 otherwise) and writes its ticket's name to the queue's `holder` file, both under `next.lock`, keeping the ticket and its lock until it releases the lock, when it removes the ticket. A run that takes the lock with no ticket writes `holder` empty and `streak` 0. A ticket is a file in the queue named by the pattern `<20 digits>-priority-p<1-5>` or `<20 digits>-ordinary-p<1-5>`; `next`, `next.lock`, `streak` and `holder` stay from run to run. `<n>`, in the waiting line, is the number of live tickets held or whose turn comes before the waiter's own, plus one when the lock is held and no live ticket is the one `holder` names; between a holder taking the lock and writing `holder` the count can be one too high for a moment, which only the line shows. **Lanes.** A land takes a `priority` ticket, for `land.lock` and for `check.lock` alike, when its merged tree's code key counts as checked (with the same command, or through the settings-free key) and **Readers of checked code** selected a test, so its check runs only those readers (a readers' selection that failed, so that every test runs, is `ordinary`); every other run, a land of unchecked code, a `--check-only` run with slot 0 and `verify`'s runner (`shared-check-runner`), takes an `ordinary` one. The lane is a parameter of `lock.rs`'s helper and of the crate's internal runner (`run_in`); the library call `run_check` (`shared-check-runner`) always waits with an `ordinary` ticket, so `verify` passes nothing new. A land knows its lane only where it takes the lock for its readers, after **Readers of checked code** has run `clm affected --readers`; its earlier `land.lock` wait, for unchecked code, is always `ordinary`. A land that waited there and, on preparing again, finds its code checked, releases that lock and its ticket and queues afresh with the lane its readers give it, so a land waits with the lane of the check it will run. **Priority.** A land's priority is `orchestrator`'s `land_priority` for its branch, head and the fetched base (`orchestrator/land-priority`), computed once per attempt before its first wait and kept when the land queues afresh within that attempt; every other run, a `--check-only` run and `verify`'s runner, waits at `p5`. A priority that can't be computed, such as specs that don't load at the base, is `p5` with a warning, since the order is never worth failing a land for. Exclusion never rests on the queue: a run that skips it, such as an older `clm`, still waits on the lock itself, and only the order is lost. A queue file that can't be created, read or locked is exit 2, as the lock's is.

**Its own binary** (`land-binary-copy`). A check that rebuilds `clm` can replace the file `clm land` runs from. So before its first subcommand it copies `std::env::current_exe` into a fresh `land-bin-*` directory beside its temporary worktrees, keeping permissions, and runs the gate, `clm affected` and `clm check` from the copy, removed on every exit path; when the copy can't be made it exits 1 naming the path. The lander's binary is `main`'s rule, which the merged tree lives under. `CLM_GATE`, `CLM_CHECK` and `CLM_AFFECTED` override the three commands, run as given, so tests fake them.

**Selected tests** (`graph-test-selection`). Before `[land] check` it runs `clm affected --base <sha>`, `<sha>` the fetched base's commit (the merge commit's merge base with it, so the selection is what the branch brings), in the tree on the merge commit and under the land lock when the check holds it (the check lock covers `[land] check` alone), and writes its stdout unchanged to `selection` in a temporary directory, which `CLM_SELECTION` names. When it fails, `clm affected failed: <the first non-empty line of its stderr, or the spawn error>; the check runs every test`, and the check runs without `CLM_SELECTION`: a failed selection costs time, never a skipped test.

**Readers of checked code** (`doc-readers-marked`). `code_key` leaves the doc-only paths out, so a `checked` record says nothing of the tests that read them: a spec branch's prose can fail such a test (a retired term under `specs/`) on a tree whose code was checked. A land whose code key is checked checks the merge commit out into a temporary worktree, as an attempt with no lock does, and runs `clm affected --base <sha> --readers` there (`check.affected/affected-readers`), `<sha>` as in **Selected tests**. When it prints nothing, the attempt goes on in that worktree with no lock. The gate on the head and **Derived specs** have run by then, in that worktree. When it prints a line, the attempt removes that worktree and goes on as one whose check runs: it takes the lock when the check compiles into `.clm/land`, checks the merge commit out into the land tree (a caller's `CARGO_TARGET_DIR` keeps the temporary worktree, with no lock), so the check reuses the warm target, and runs `[land] check`, so the readers' tests alone, with `CLM_SELECTION` naming a file holding that output unchanged. When it fails, it prints `clm affected failed: <reason>; the check runs every test` as **Selected tests** does and goes on the same way with `CLM_SELECTION` unset. A failing check exits 1 as any land's does.

**The check's output** (`land-check-log`, `check-step-markers`). One pipe (`std::io::pipe`) carries `[land] check`'s stdout and stderr in the order written, copied to stderr and to `<home>/.clm/land-logs/<branch>.log`, the branch's slashes as directories, created and truncated first; the last 40 lines (split on `\n`) are kept in memory. It reads until the pipe closes, so a check must end what it starts. **Steps.** While reading, the runner numbers the output's lines from 1 and follows the steps the check marks in GitHub Actions' form, testing each line with a trailing `\r` removed: a line starting `::group::` starts a step named by the rest of the line, trimmed, at that line, ending any step open at the line before it; a line that is `::endgroup::` once trailing whitespace is trimmed ends the open step at that line; a line of either kind can sit anywhere in the output, a marker split across reads is matched once the line is whole, and a last line with no `\n` counts. The step open when the pipe closes, if any, is the *last step*, spanning its marker line to the output's last line; when every step had ended, or none was marked, there is none. Only the name, the first line and the line count are kept, so the steps cost no memory beyond the tail. A failing check's refusal, printed last as `clm land: <why>`, is `last 40 lines of `<check>`:`, those lines and `` `<check>` fails on the merge with main ``, followed with a last step by `` ; the step open when it ended is `<name>`, lines <first> to <last> of its output `` and with a log by `` ; its output is at <path> ``, and the log stays; a passing check removes it. A log that can't be created warns naming the path, and the refusal then has no `; its output is at` clause; the line numbers then count the output copied to stderr. `clm check`'s output isn't logged, and `--check-only` and `verify` name no step.

**Step times** (`step-times`, `check-time-report`, `land-times-on-the-landed-record`). The runner that follows the check's steps also reads the clock, and `CheckRun` gains the timing beside its verdict, leaving `Step` as it is: a step's time runs from the read of its `::group::` line to the read of the line that ends it, or to the pipe's close for the last step, and the command's time from its start to its exit, each in whole seconds rounded down. Within a step, a line starting `::clm-selected::`, trailing whitespace trimmed, adds the rest of the line to the step's `selected_by`, each path once in the order first seen; such a line outside any step is ignored (`check-selector-lines`). A name may repeat, each step its own entry. Any attempt whose check command ran, a full selection or only the readers of checked code, puts the timing on the `landed` record it appends; a land that runs only `clm check` puts none. `--check-only` and `verify` use the same runner and record no timing. Once the check passes, and while the merged checkout and the land lock are still held, the land computes the chains it may print: for each of the up to three steps with the most seconds (ties in the check's order), `clm affected --why <path> --base <sha>` for the step's first `selected_by` path, run as it runs `clm affected`, against the same base. After step 8's append it prints on stderr `check <n>s, target <t>s`. When `<n>` is greater than `<t>` it then prints those steps, each as `  <name> <n>s` with its chain's lines indented below it, `  <name> <n>s, selected by no file` when it named no path, or `(no chain: <why>)` when the command failed; then `last <k> timed lands: <m> over target; most time in <step> <n>s, ...`, over this land and the last 19 timed lands `Fold::timed_lands(19)` gave in step 1's fold, naming the up to three steps with the most total seconds across them; then `see clm guide ci-and-deployment, where slow tests go`. None of it changes the land's outcome or exit status, and it files nothing: whether a pattern of slow lands calls for a backlog item, a selection fix or a move to `[verify]` is the reader's judgement, which the guide's section informs.

**Freeing a build's disk** (`land-frees-build-target`). After the push and the append of an `impl/*` land it deletes the `target/` at `git rev-parse --show-toplevel` when that is a linked worktree whose `HEAD` is the landed branch, printing `removed <path>`; a missing one is skipped and a failed delete warns. `clm run --finish` removes the worktree itself (`orchestrator.run`).

**`--check-only`** (`land-check-command`, `worktree-check-counts`). Given a branch as well it exits 1 (by hand: clap's `conflicts_with` would exit 2) and runs nothing. On any branch, in the CLM root (`--root`, else found from the current directory), it runs `[land] check` as step 5 does but with `CARGO_TARGET_DIR` as the caller has it, so a build compiles into its own `target/`, then `clm check` with its detectors (`CLM_CHECK` overrides it), on the working tree as it stands, output on stderr. **The record.** Before the check command starts it reads `HEAD`'s commit (`git rev-parse HEAD`) and whether the working tree is clean: `git status --porcelain` in the CLM root's worktree prints nothing, so no tracked file differs from `HEAD`, staged or not, and there is no untracked file git doesn't ignore. After `clm check` passes it reads both again. When the tree was clean both times, `HEAD` was the same commit both times, and the local `origin/main` exists and is an ancestor of `HEAD` (`git merge-base --is-ancestor`), it appends, with `ledger`'s `append`, a `checked` record for `code_key(root, HEAD)` with the command a land's `checked` record names (`[land] check`, or `clm check` when none is set) and prints `recorded <HEAD's full sha> as checked`. Otherwise it appends nothing and prints `not recorded: <why>`, `<why>` being the first that holds of `the working tree has uncommitted changes`, `HEAD moved during the check` and `HEAD doesn't contain origin/main`. An append that fails warns `checked not recorded: <error>` and still passes, since a land then runs its own check; an append reported unpushed is enough, since a land on this clone folds the local writer's branch. It prints `checks pass` last; otherwise exits 1 with `` `<command>` fails `` or `` `clm check` fails ``, the failure messages being a parameter of the helpers a land shares, recording nothing; a `clm.toml` that doesn't load is exit 1 with `config`'s message. Its check command runs under `<home>/.clm/check.lock` (**The check lock**, above) only when its slot is 0, `<home>` found from the main worktree as a land finds it. **Why the record holds for the land** (`worktree-check-counts`). A land looks a `checked` record up by the merged tree's code key alone, an exact match, never by who wrote it or where (`legacy-code-key`). A land of a commit that contains the `main` it fetches makes a merge whose tree is that commit's tree, so its code key is the one recorded; when `main` has moved, the merged tree's code key differs whenever the code differs, and the land runs its usual selected check, the safety net. The record is made by the same `clm` helpers, on the same clone and machine and into the same ledger as a land's, from a clean committed tree that is the commit's own, with every test where a land would run only those `clm affected` selects, so it proves more than a land's record does; the build's full run stays the safeguard `graph-test-selection` counts on for a test the selection would skip. What differs is ignored state only: a build's own `target/`, where a land checks in a tree `git clean -fdx` has emptied of ignored files beside the warm `.clm/land`. A check that passes only because of an ignored file it doesn't make itself is the project check's own defect, which the next land after `main` moves, or `clm verify`, shows. It exists for a build's pre-land check (`orchestrator.run`), which Claude Code's worktree isolation refused as a script path.

**Offline** (`no-local-land-with-remote`, `none-keeps-its-own-remote`). Every host lands through `origin`: with `host = "none"` it is the clone's own remote (`host/none-keeps-a-remote`), so there is no local land and no `(local)` output. `main` has one copy, the remote's, so a step-1 fetch that fails for want of `origin` exits 1 with `offline: origin can't be reached; push and land when it can`. **A fetch that fails is tried again** (`offline-land`): lands from linked worktrees of one clone share its remote-tracking refs, and one land's ledger push or branch deletion can move `refs/remotes/origin/...` between another's fetch reading a ref and updating it, which git refuses with `cannot lock ref ... is at <x> but expected <y>`. So `Repo::fetch` runs `git fetch origin` up to three times, waiting 1 second and then 2 (the waits `land-push-retry` uses), and after the last failure runs `git ls-remote origin refs/heads/main`: when that fails too, `origin` can't be reached and the land is offline; when it succeeds, the land exits 1 with the last fetch's git message instead. `clm reconnect`'s fetch goes the same way. A truly offline land, or reconnect, spends the 3 seconds of waits before it says so. One helper on `Repo` does this for both, taking the fetch's arguments (`clm reconnect` passes `--quiet --prune`), with `ls-remote` run through `Repo::git` like every other git call (`git-plumbing-layer`) and the waits in the constant `push_main` uses, shared rather than copied; the refusal is `Error::Refused` holding the last fetch's `clm_git` error text, and `OFFLINE` stays in `lib.rs`. `clm approve`'s own fetch (`land.approval`) is left as it is (`backlog/clm-approve-calls-a-lost-ref-race-offline`). **`clm reconnect`**, for a repository whose `origin` is newly set, fetches `origin` with `--prune`, so a remote-tracking ref the old remote left doesn't stand for the new one's branch, then pushes `main` with a lease when `origin/main` is missing or an ancestor, then every local `clm/ledger/*` branch, and prints `reconnected <n> lands`; when `origin/main` has moved it pushes only the ledgers, prints `land again <branch>` for each local land (the merge commit's second parent) and exits 1.

## Requirements

### Requirement: land
`clm land [<branch>]` SHALL fetch `origin`, run the gate on the branch's head against `origin/main` (on the merge commit instead for a head `regenerates-behind-specs` covers) with `--approved` when the ledger holds an `approved` record for the head's fingerprint that the `approval` policy accepts, merge the head into `origin/main` in a scratch tree, run `[land] check` and then `clm check` on the merged tree unless a `checked` record names its code key with the same command (then `clm check --no-detectors`, preceded by `[land] check` only when the readers `selected-tests` asks for select a file), run the gate on the merged tree, make a merge commit of the head into `origin/main`, push it to `main` without force and with a lease on the fetched `origin/main`, append `landed` naming the merged tree's code key and whether the land needed review, and print `landed <sha> <kind>`. When the push is refused because `main` moved it SHALL start again from the fetch, up to ten attempts. It SHALL push nothing and exit 1, naming why, when the gate blocks, the merge conflicts outside what `regenerates-behind-specs` regenerates, a check fails, or every attempt is refused; and SHALL append `checked` whenever the check command and `clm check` pass on a tree, even when the push is later refused.


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

#### Needs
- clm/renamed-commands

### Requirement: landed-branch-cleanup
After a successful push, `clm land` SHALL delete the landed branch on `origin` through `host`'s `branch delete`, leased on the head it landed, when the branch is `impl/*`, `spec/*` or `propose/*`, warn without failing when that delete fails, and leave any other branch.

#### Scenario: impl branch removed
- GIVEN a landed `impl/a` whose tip on `origin` is the head it landed
- WHEN `clm land impl/a` finishes
- THEN `origin` has no `impl/a` and it exits 0

#### Scenario: moved head kept
- GIVEN a check command that pushes a new commit to `origin/impl/a` while the land runs
- WHEN `clm land impl/a` finishes
- THEN `origin/impl/a` is still that commit, it printed a warning, and it exits 0

### Requirement: offline-land
`clm land` SHALL land through `origin` on every host, `none` included, whose `origin` is the clone's own remote (`host/none-keeps-a-remote`), and SHALL never land on the local `main` alone. When `origin` can't be reached, `clm land` SHALL exit 1 saying it is offline and to land when `origin` is reachable, land nothing locally and append nothing. A fetch of `origin` by `clm land` or `clm reconnect` that fails SHALL be tried again, at most twice more, waiting 1 second before the first of these and 2 seconds before the second, so that a land whose fetch lost a race with another land of the same clone updating a remote-tracking ref (git's `cannot lock ref`) goes on as if the first fetch had succeeded; when the last try fails it SHALL say it is offline only when `origin` can't be reached either, and otherwise exit 1 with git's message from the last fetch and no `offline`, landing nothing and appending nothing. `clm reconnect` SHALL fetch `origin`, pruning the remote-tracking refs of branches `origin` no longer has, then push `main` when `origin/main` doesn't exist or is an ancestor of it, then every local ledger branch, and print `reconnected <n> lands`; when `origin/main` has moved it SHALL push only the ledgers, print `land again <branch>` for each local land's branch, and exit 1.

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
- GIVEN a passing code branch `impl/a`, and a script named `git` in a directory put first on `PATH` that, for the first `fetch` naming `origin`, prints `error: cannot lock ref 'refs/remotes/origin/main'` on stderr and exits 1 without running the real `git`, and runs the real `git` for every other call; the test runs the `clm` binary as a child process with that `PATH` (`crates/clm/tests`)
- WHEN `clm land impl/a` runs
- THEN it exits 0 printing `landed <sha> code`, `main` on `origin` is that merge commit, and its stderr doesn't contain `offline`

#### Scenario: a fetch that keeps failing with origin reachable
- GIVEN a passing code branch `impl/a`, and a script named `git` first on `PATH`, as in *a fetch that loses a ref race is tried again*, that fails every `fetch` naming `origin` that way
- WHEN `clm land impl/a` runs
- THEN it exits 1, its message contains `cannot lock ref` and not `offline`, `main` on `origin` is what it was before the land, and the ledger has no new record

#### Needs
- host/none-keeps-a-remote

#### Serves
- capability/git-host
- capability/work-concurrently
- architecture/no-local-land-with-remote

### Requirement: git-through-layer
`clm-land` SHALL list `clm-git` among its dependencies and SHALL run no `git` or `gh` process itself: every call goes through `clm-git`'s `git`, `run` or repo queries (`git-plumbing-layer`), and host operations through `host`.

#### Scenario: covered by the scan
- GIVEN the workspace
- WHEN a test in `clm-land` reads `crates/clm-land/Cargo.toml` and every file under `crates/clm-land/src`
- THEN `clm-git` is a dependency and no file holds `Command::new("git")` or `Command::new("gh")`

#### Serves
- architecture/reuse-first

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
When the caller hasn't set `CARGO_TARGET_DIR`, or set it empty, `clm land` SHALL run `[land] check` with `CARGO_TARGET_DIR` set to `.clm/land` in the CLM root of the repository's main worktree (the first worktree `git worktree list --porcelain` lists), whichever worktree of the repository it runs in, and SHALL hold the clone's lock on `.clm/land.lock` there (`land-target-lock`) while it does. Such a check SHALL also run with `CARGO_INCREMENTAL` set to `0` unless the caller set `CARGO_INCREMENTAL` non-empty, since incremental data in a target shared by every land only grows; a check whose caller set `CARGO_TARGET_DIR` SHALL get `CARGO_INCREMENTAL` as the caller has it.

#### Scenario: land from a linked worktree
- GIVEN a repository whose `[land] check` records `CARGO_TARGET_DIR`, and a linked worktree of it with a branch checked out
- WHEN `clm land` runs in the linked worktree with `CARGO_TARGET_DIR` unset
- THEN the check saw `CARGO_TARGET_DIR` set to the main worktree's `.clm/land`, not the linked worktree's

#### Scenario: CLM root in a subdirectory
- GIVEN a git repository whose CLM root is its subdirectory `tool/`, and a `[land] check` that records `CARGO_TARGET_DIR`
- WHEN `clm land` runs on a branch with `CARGO_TARGET_DIR` unset
- THEN the check saw `CARGO_TARGET_DIR` set to the main worktree's `tool/.clm/land`

#### Scenario: incremental off in the shared target
- GIVEN a `[land] check` that records `CARGO_INCREMENTAL`
- WHEN `clm land` runs with `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL` unset
- THEN the check saw `CARGO_INCREMENTAL` set to `0`

#### Scenario: caller's incremental kept
- GIVEN the same check
- WHEN `clm land` runs with `CARGO_TARGET_DIR` unset and `CARGO_INCREMENTAL=1`, and then with `CARGO_TARGET_DIR` set to a directory of the caller's and `CARGO_INCREMENTAL` unset
- THEN the first check saw `CARGO_INCREMENTAL` set to `1` and the second saw it unset

#### Serves
- capability/work-concurrently
- capability/land-safely

### Requirement: land-target-lock
A `clm land` whose `[land] check` will run with `CARGO_TARGET_DIR` set to the main worktree's `.clm/land` SHALL hold an exclusive advisory lock on `.clm/land.lock` beside it from before it checks the head out into the land tree until the push of the merge commit has gone through and the target's cap has been applied (`land-target-cap`), or the attempt ends sooner, so that the gate on the merge commit, `clm check`, the push and the cap run under it, releasing it on every exit path. A land that waits for the lock SHALL say so on stderr, naming `.clm/land`. A land that runs no `[land] check`, because its code key was already checked or no check is set, or whose caller set `CARGO_TARGET_DIR`, SHALL take no lock.

#### Scenario: overlapping lands
- GIVEN two branches `impl/a` and `impl/b` of one clone, no `checked` records, and a `[land] check` that appends `start <branch>` to a log, sleeps 2 seconds and appends `end <branch>`
- WHEN `clm land impl/a` and `clm land impl/b` start at once, from two linked worktrees
- THEN in the log every `start <branch>` is followed by the `end` of that same branch before any other `start`, so the two checks never interleave, and the land that waited printed `waiting` naming `.clm/land` on stderr

#### Scenario: held through the gate on the merge
- GIVEN the same two branches, a `[land] check` that appends `check <branch>` to a log and sleeps 2 seconds, and a fake `CLM_GATE` that appends `gate` to it and passes
- WHEN `clm land impl/a` and `clm land impl/b` start at once
- THEN the log reads `gate`, `check <one branch>`, `gate` and then `gate`, `check <the other>`, `gate`: both of a land's gate runs fall between its lock's taking and release, never inside the other land's

#### Scenario: checked code takes no lock
- GIVEN a branch that changes only `specs/x/SPEC.md`, a `checked` record for `origin/main`'s code key, and another process holding the lock
- WHEN `clm land` runs
- THEN it lands without waiting and prints no `waiting` line

#### Scenario: lock released after a failing check
- GIVEN a `[land] check` that exits 1
- WHEN `clm land` runs and then a second `clm land` runs
- THEN the first exits 1 and the second doesn't print `waiting`

#### Serves
- capability/work-concurrently
- capability/land-safely

### Requirement: land-binary-copy
Before running its first subcommand, `clm land` SHALL copy its own binary into a fresh directory under the system's temporary directory and run the gate and `clm check` from that copy, unless `CLM_GATE` or `CLM_CHECK` names the command; it SHALL remove the copy on every exit path. When the copy can't be made it SHALL exit 1 naming the path, pushing and recording nothing.

#### Scenario: check deletes the binary
- GIVEN a `clm` binary copied to a scratch path, a repository whose `[land] check` deletes that scratch path, and `CLM_GATE` and `CLM_CHECK` unset
- WHEN the scratch `clm land` runs on a branch
- THEN it lands and prints `landed <sha> code`

#### Scenario: copy removed
- GIVEN an empty `TMPDIR` of the test's own
- WHEN `clm land` runs on a branch and lands
- THEN `TMPDIR` is empty again

### Requirement: land-frees-build-target
After a successful push of an `impl/*` branch, `clm land` SHALL delete the `target/` directory at the top of the worktree it runs in when that worktree is a linked one whose `HEAD` is the landed branch, and print `removed <path>`. It SHALL NOT delete the main worktree's `target/`, delete anything after a `spec/*` or `propose/*` land or a land that pushed nothing, or fail the land when the delete fails.

#### Scenario: build's target freed
- GIVEN a linked worktree with `impl/a` checked out and a `target/` directory holding a file, and a passing check
- WHEN `clm land impl/a` runs in that worktree
- THEN it exits 0, the worktree's `target/` is gone, the worktree itself is still there, and it printed `removed` with the `target/` path

#### Scenario: main worktree kept
- GIVEN the main worktree with `impl/a` checked out and a `target/` directory holding a file
- WHEN `clm land impl/a` runs there
- THEN it exits 0 and `target/` and its file are still there

#### Scenario: spec land kept
- GIVEN a linked worktree with `spec/y` checked out and a `target/` directory
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

#### Serves
- capability/wrap-up-session

### Requirement: land-push-retry
When the push of the merge commit to `main` is refused, `clm land` SHALL fetch `origin`'s `main` again and judge it: when it is the merge commit, it SHALL treat the push as done and go on to append `landed`; when it is neither the merge commit nor the fetched base, it SHALL start the next attempt (`land`); when it is still the base, it SHALL push the same merge commit again with the same lease, at most twice more in that attempt, waiting 1 second before the first of these and 2 seconds before the second, judging each refusal the same way, without counting these pushes as attempts. When `main` is still at the base after the last push it SHALL exit 2 with git's message for the refused push, including what the remote printed, and a line saying the failure may be transient and that running `clm land <branch>` again is safe, having appended no `landed` record. When the fetch after a refusal fails it SHALL exit 2 with the refused push's git message and no such line. When `append` reports the `landed` record unpushed, on any host, it SHALL warn naming the writer's ledger branch `clm/ledger/<person>/<clone>` and `git push origin` with that branch, and exit 0.

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
- GIVEN a passing code branch `impl/a`, and a script named `git` in a directory put first on `PATH` that runs the real `git` and, for a `push` naming `refs/heads/main`, then exits 1 whatever the real one did; the test runs the `clm` binary as a child process with that `PATH` (`crates/clm/tests`), since the in-process `clm_land::land` takes the process's own `PATH`
- WHEN `clm land impl/a` runs
- THEN it exits 0 printing `landed <sha> code`, `main` on `origin` is that merge commit, whose second parent is the head, and the ledger has one `landed` record for it

#### Scenario: ledger push refused
- GIVEN a passing code branch `impl/a`, and a `pre-receive` hook on `origin` that declines every push naming a `refs/heads/clm/ledger/` ref and accepts every other push
- WHEN `clm land impl/a` runs
- THEN it exits 0 printing `landed <sha> code`, `main` on `origin` is the merge commit, the local ledger branch has the `landed` record, and it warned naming that ledger branch and `git push origin`

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
`clm land --check-only` SHALL run, in the working tree of the CLM root found from the current directory, `[land] check` when `clm.toml` sets it, the way a land runs it (through `sh -c`, with `CI` unset, and `CARGO_TARGET_DIR` left as the caller's environment has it, so a build's check compiles into its worktree's own `target/` (`build-places`)), and then `clm check`. It SHALL run the check command with `CLM_SELECTION` unset, so every test runs. It SHALL print `checks pass` as its last line and exit 0 when both pass; otherwise it SHALL exit 1 naming the command that failed, append nothing, and SHALL NOT run `clm check` after a failing check command. When both pass, the working tree had no change to a tracked file and no untracked file git doesn't ignore both before the check command started and after `clm check` ended, `HEAD` was the same commit both times, and the local `origin/main` is an ancestor of it, it SHALL append a `checked` record for `HEAD`'s code key with the command a land's `checked` record names (`[land] check`, or `clm check` when none is set) and print `recorded <HEAD> as checked` on stdout; otherwise it SHALL append nothing and print `not recorded: <reason>` on stdout, either line coming before `checks pass`. An append that fails SHALL only warn. It SHALL fetch nothing, make no commit and push nothing but that record. Given a branch as well, it SHALL exit 1 without running either command. When `clm.toml` can't be loaded, it SHALL exit 1 with the reason, running neither command. Its build checks that `crates/clm/tests/capability/land_safely.rs`, whose branch runs `--check-only` before `main` moves under it, still sees its land run the check on the merge and refuse it.

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

#### Serves
- capability/work-concurrently
- capability/land-safely

### Requirement: shared-plumbing
`clm-land` SHALL take the remote's name from `clm-git`'s `REMOTE` wherever it names the remote to git (a fetch, a push, a ref such as `refs/remotes/<remote>/main` or `<remote>/main`, the check that the remote exists), building each such argument from `REMOTE` rather than spelling it: no line of a file under `crates/clm-land/src`, apart from those whose first non-blank characters are `//`, SHALL hold `"origin"`, `"origin/` or `remotes/origin` (`remote-plumbing-scan`). Messages for a person or a session that name `origin`, such as ``origin's `main` has moved: ...``, ``no `origin`: add the remote, then reconnect`` and `offline: origin can't be reached; push and land when it can`, SHALL keep their wording. `clm land`, `clm approve` and `clm reconnect` SHALL keep their output and exit codes.

#### Scenario: no copy of the remote's name
- GIVEN the workspace
- WHEN a test in `clm-land` reads every file under `crates/clm-land/src`, skipping each line whose first non-blank characters are `//`
- THEN no line holds `"origin"`, `"origin/` or `remotes/origin`

#### Scenario: messages keep naming origin
- GIVEN the workspace
- WHEN the same test reads `crates/clm-land/src/lib.rs` and `crates/clm-land/src/reconnect.rs`
- THEN `lib.rs` still holds `origin can't be reached` and `reconnect.rs` still holds ``origin's `main` has moved``

#### Serves
- architecture/reuse-first

### Requirement: derived-specs
When the head adds a change, as `gate.fingerprint/fingerprint` defines it (`clm-gate`'s `adds_change` over the merge base's and the head's listings), or applies one, a change whose `changes/<id>/change.md` has status `proposed` or `approved` at the base and, in the merge commit, has status `applied`, or is gone while `changes/archive/<id>/change.md` has status `done`, `clm land` SHALL, after the merge commit and before any check, regenerate `specs/` with `sync.regenerate`'s `regenerated`, without its stale-delta merge, onto the base, `origin/main`'s on every host (`land/offline-land`), from the CLM root in the tree holding the merge commit, for the added changes whose status is `applied`, or `done` for a new archived one, and the applied ones, in the order `regenerated` chooses (`sync.regenerate/regenerated-tree`), a `change.md` that doesn't parse regenerating nothing, and compare the result with the merge commit's `specs/` byte for byte over the union of both sides' paths in byte order, a path on one side only being a difference. When they differ, or `regenerated` stops, it SHALL push nothing and exit 1 with rule `land/derived-specs`, naming the first differing path or the lines `regenerated` returned, and `clm rebase`. A head that adds and applies no change SHALL land without the comparison, since its `specs/` edits are its own. No file under `crates/clm-land/src` SHALL call `clm_apply::reapply` or `clm_model::load`: the regenerate is `sync.regenerate`'s alone. `clm-land` SHALL call it from crate `clm-regenerate`, never through `clm-sync`, which reaches `clm-health` (`clm/health-isolated`).

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

#### Scenario: no copy of the regenerate
- GIVEN the workspace
- WHEN a test in `clm-land` reads every file under `crates/clm-land/src`
- THEN no file holds `clm_apply::reapply` or `clm_model::load`

#### Serves
- capability/catch-drift
- architecture/regenerate-dont-merge
- architecture/humans-review-intent

#### Needs
- clm/renamed-commands
- gate.fingerprint/fingerprint
- sync.regenerate/regenerated-tree
- land.approval/approve

### Requirement: land-tree
An attempt of `clm land` that holds the lock on `.clm/land.lock` SHALL check the head and the merge commit out in `.clm/land-tree` in the CLM root of the main worktree, a detached worktree of the repository that it SHALL keep when the attempt ends. Before each checkout it SHALL make that tree exist and clean: when git lists the path as a worktree and its directory exists, `git checkout --force --detach` there and then `git clean -fdx`; otherwise, or when either fails, it SHALL remove whatever is at the path, prune git's worktree list and add the worktree afresh. Everything a land keeps in the main worktree, the land tree, `.clm/land`, `.clm/land.lock`, `.clm/check.lock` and `.clm/land-logs/`, SHALL be under that CLM root's `.clm/`, so that a repository whose `.gitignore` lists only the entries `clm init` adds (`init/init-ignores-local-settings`) shows none of it in `git status`.

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
- GIVEN a repository whose committed `.gitignore` lines are `clm.local.toml`, `.clm/` and `.claude/worktrees/` and nothing else, a branch `impl/a` with a passing `[land] check` that writes a file into `CARGO_TARGET_DIR`, and a branch `impl/b` whose `[land] check` exits 1
- WHEN `clm land impl/a` runs and then `clm land impl/b`
- THEN the first lands, the second exits 1, and `git status --porcelain` in the main worktree prints nothing, with no `target/` directory there

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
After the push to `main`, `clm land` SHALL append, in the commit that appends `landed` and before it, the `effort` records `clm effort record --branch <branch>` would append for the landed branch's sessions on this machine, read once before its first attempt from the transcripts directory its environment gives (`effort/record-command`). When they can't be read it SHALL warn, naming why, and land as before.

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
After the push of a land whose check ran with `CARGO_TARGET_DIR` set to the main worktree's `.clm/land`, and before it releases the lock on `.clm/land.lock`, `clm land` SHALL sum the lengths of the regular files under `.clm/land`, following no symbolic link, and when the sum is greater than `[land] target_cap_gb` gigabytes (10^9 bytes each) SHALL remove `.clm/land` and print on stderr `cleared .clm/land (<n> GB, cap <cap> GB)`, `<n>` the sum in whole gigabytes rounded down. When any entry can't be read the measure fails and nothing is removed; a failure to measure or remove SHALL print a warning naming the path and leave the land's outcome unchanged. A land that took no lock SHALL neither measure nor remove it.

#### Scenario: over the cap
- GIVEN a `clm.toml` with `[land]`, a `check` that creates a sparse file of 2,000,000,000 bytes in `CARGO_TARGET_DIR`, and `target_cap_gb = 1`, and a branch `impl/a`
- WHEN `clm land impl/a` runs with `CARGO_TARGET_DIR` unset
- THEN it prints `landed`, stderr holds `cleared .clm/land (2 GB, cap 1 GB)`, and the main worktree's `.clm/land` doesn't exist

#### Scenario: under the cap
- GIVEN the same, with a check that creates a file of 10 bytes and the cap left at its default
- WHEN `clm land impl/a` runs with `CARGO_TARGET_DIR` unset
- THEN it lands, prints no `cleared` line, and the file is still under `.clm/land`

#### Scenario: failed check keeps the target
- GIVEN the over-the-cap check, made to exit 1 after creating its file
- WHEN `clm land impl/a` runs
- THEN it exits 1 and `.clm/land` still holds the file

#### Scenario: caller's target untouched
- GIVEN the over-the-cap setup
- WHEN `clm land impl/a` runs with `CARGO_TARGET_DIR` set to a directory of the caller's
- THEN it lands, prints no `cleared` line, and the caller's directory still holds the file

#### Serves
- capability/work-concurrently
- capability/land-safely

#### Needs
- config/land-check-setting

### Requirement: reconnect-names-rebase
When `origin/main` has moved, the message `clm reconnect` prints SHALL tell the person to run `clm rebase` on each branch landed locally and then land it again, keeping its ``origin's `main` has moved`` start (`shared-plumbing`), and no file under `crates/clm-land/src` SHALL name `clm` followed by a space and one of the old subcommands `sync`, `checkpoint` or `page`.

#### Scenario: moved while offline
- GIVEN a repository that landed `impl/a` on its local `main` while offline, and an `origin/main` that has moved since
- WHEN `clm reconnect` runs
- THEN it prints ``origin's `main` has moved`` with `clm rebase`, prints `land again impl/a`, and exits 1

#### Serves
- architecture/plain-terms

### Requirement: spec-branch-holds-spec-paths
`clm land <branch>`, for a branch named `spec/*`, SHALL exit 1 when `gate/files-outside-spec-paths` lists any file for its head against the fetched base, naming each file and saying a spec branch holds only spec paths. It SHALL do so before it takes the land's lock, merges or runs any check, and it SHALL push and append nothing. A branch with any other name SHALL land as before.

#### Scenario: code on a spec branch
- GIVEN a branch `spec/x` that changes only `crates/x/src/lib.rs`, whose gate prints `code`, and a check command that records that it ran
- WHEN `clm land spec/x` runs
- THEN it exits 1 naming `crates/x/src/lib.rs`, the check command never ran, and `main` on `origin` and the ledger are unchanged

#### Scenario: a suggestion on a spec branch
- GIVEN a branch `spec/x` that only adds `backlog/x.md`, whose gate prints `code`
- WHEN `clm land spec/x` runs
- THEN it lands

#### Scenario: code on an impl branch
- GIVEN a branch `impl/x` that changes only `crates/x/src/lib.rs`
- WHEN `clm land impl/x` runs
- THEN this rule refuses nothing and it lands

#### Needs
- gate/files-outside-spec-paths

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
Before merging, `clm land` SHALL take the *held* components: the `components` of every `kind: bug` item on the fetched `origin/main`'s backlog whose `verify_failures` is not empty. When the branch's code changes a file under the `paths` of a held component, as the gate judges code, it SHALL push nothing and exit 1 with rule `land/verify-stop`, naming each held component it touches and the item that holds it, and saying that the branch fixing it lands once a commit it adds carries the trailer `Resolves: backlog/<slug>` for that item; a branch one of whose commits carries that trailer for every item holding a component it touches SHALL land as before. It SHALL also read the red `verified` results on `origin/main`'s first-parent chain after the newest passing one, the newest record for each commit (`ledger/verified-record`), and for each that no backlog item's `verify_failures` names and whose `components` the branch's code touches, print on stderr `warning: slow tests failed after <sha>, culprit <land|unknown>, on <component>...; run /clm:triage to take it in` and land as before. A spec land, and a code land touching no held component, SHALL land as before.

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

#### Scenario: an untriaged red warns
- GIVEN no item with `verify_failures`, a red `verified` record for `origin/main`'s head naming component `b`, and a code branch changing `b`'s file
- WHEN `clm land` runs
- THEN it prints a warning naming the red commit and `b` and `/clm:triage`, and lands

#### Scenario: a red record alone holds nothing
- GIVEN a red `verified` record naming `b` written by a host's pipeline, and no backlog item naming it
- WHEN `clm land` runs on a branch changing `b`'s file
- THEN it lands

#### Needs
- ledger/verified-record
- model/parse-backlog

#### Serves
- capability/slow-tests-after-land

### Requirement: shared-check-runner
`clm-land` SHALL offer as a library call the runner that `clm land` and `clm land --check-only` run `[land] check` through: it takes the command, the directory to run it in, an optional selection file for `CLM_SELECTION`, the environment to set and unset, and the log file's path, runs the command through `sh -c` under the clone's check lock (`check-lock`), waiting for it with an ordinary ticket and printing its waiting line as `lock-queue` says when it waits, copies its output to stderr and the log as `land-check-log` says, and returns whether it passed with the last 40 lines, whether the log holds the whole output, and the step open when its output ended, if any, with that step's name and first and last line numbers as `check-log` counts them, leaving the log for the caller to keep or remove. `clm land` and `--check-only` SHALL call it with the environment they set today, so their behaviour is unchanged apart from `check-log`'s step in a land's refusal.

#### Scenario: a caller waits on the check lock
- GIVEN a command that writes `ok` and exits 0, and `.clm/check.lock` held by another process that releases it after 1 second
- WHEN the runner runs the command
- THEN it printed `waiting for another check` ending `: 1 ahead`, ran the command after the lock was released, wrote `ok` to the log, and returned a pass

#### Scenario: land unchanged
- GIVEN a `[land] check` that fails printing 50 numbered lines
- WHEN `clm land` runs
- THEN its refusal holds lines 11 to 50 and the log's path, as `land-check-log` says

#### Scenario: the open step returned
- GIVEN a command that prints `::group::a`, a line, `::group::b` and two lines, and exits 1
- WHEN the runner runs the command
- THEN it returns a failure with the step `b`, lines 3 to 5

#### Serves
- capability/slow-tests-after-land

### Requirement: settings-unchanged-is-checked
When no `checked` record names the merged tree's code key with the same command, `clm land` SHALL count the tree as checked when the bytes of the CLM root's `clm.toml` differ between the fetched base and the merge commit (a missing file read as `None`), `clm-gate`'s `settings_unchanged` is true of its bytes at the two, and a `checked` record names, with the same command, the key `ledger`'s `code_key_with(root, <merge>, <prefix>clm.toml, <base>)` returns. It SHALL then run what a land whose code key is checked runs (`clm check --no-detectors`, preceded by `[land] check` only when the readers `clm affected --base <sha> --readers` prints select a file, a test marked `clm:reads clm.toml` among them) and, once that passes, SHALL append a `checked` record for the merged tree's own code key with that command before the push. The lookup SHALL be made where the attempt computes the merged tree's code key, before it decides on the land lock, so such a land takes the lock only when its readers select a file. Its build adds to `crates/clm/tests/capability/upgrade_clm.rs` a land of a stamp-only branch that runs no `[land] check`. Any other difference in `clm.toml`, or no `checked` record for that key, SHALL leave the land to run its check as before.

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
Every run of `[land] check`, by `clm land <branch>` or by `clm land --check-only`, SHALL set `CLM_SLOT` in the command's environment to what `orchestrator`'s `place_slot` returns for a branch: for a land, the branch it lands; for `--check-only`, the branch the checkout has checked out, or 0 without calling it when the checkout's `HEAD` is detached. It SHALL replace any `CLM_SLOT` the caller's environment holds, setting `CLM_SLOT=0` too. When `place_slot` fails, it SHALL exit 2 with its error and run no check. The check lock (`check-lock`) and `CLM_SELECTION` SHALL be unchanged by it.

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
- architecture/regenerate-dont-merge

### Requirement: lock-queue
Each of the clone's two locks, `.clm/land.lock` (`land-target-lock`) and `.clm/check.lock` (`check-lock`), SHALL go to the runs waiting for it in an order CLM sets, not in the operating system's: a run that must wait takes a ticket in a queue beside the lock in the main worktree's `.clm/`, and tries the lock only when its turn comes. A ticket SHALL carry a *lane*, priority or ordinary, and a priority from `P1` to `P5`. A `clm land` takes a priority-lane ticket when its merged tree's code key counts as checked and its readers select a test (`selected-tests`), so that its `[land] check` runs only those readers, and an ordinary one otherwise, a land whose readers' selection failed, so that every test runs, included; its priority is `orchestrator`'s `land_priority` for the branch (`orchestrator/land-priority`), `P5` when that can't be computed. Every other run takes an ordinary ticket at `P5`. The turn SHALL go first to the `P1` tickets, a priority-lane one before an ordinary one and otherwise the one taken first; then to the other priority-lane tickets, the highest priority first and then the one taken first; then to the other ordinary tickets in the same order. When six runs in a row below `P1` have taken the lock with priority-lane tickets while an ordinary ticket was waiting, and no `P1` ticket is waiting, the ordinary ticket whose turn would come first among the ordinary ones SHALL take the next turn. A ticket SHALL hold no turn once its run has ended, however it ended, and no ticket of an ended run SHALL stay in the queue past the next run that reads it. A waiting run SHALL print its waiting line on stderr when it begins to wait and again each time the count changes, ending `: <n> ahead`, `<n>` counting the run that holds the lock and the waiting runs whose turn comes before its own. A run that finds the lock free and no ticket waiting SHALL take the lock at once, printing nothing. The order SHALL decide only which run takes the lock next: a run that takes the lock without the queue, such as one by an older `clm`, still never holds it beside another.

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

### Requirement: fetch-retry-in-layer
`clm land` and `clm reconnect` SHALL make the fetches of `origin` that `offline-land` tries again through `clm-git`'s `fetch_retrying` (`vcs/fetch-retries`), mapping `FetchError::Offline` to the offline refusal and `FetchError::Git` to git's message, so `clm land`, `clm reconnect` and `clm approve` retry one way. No file under `crates/clm-land/src` SHALL define a function named `fetch_retrying`.

#### Scenario: one retry
- GIVEN the workspace
- WHEN a test in `clm-land` reads every file under `crates/clm-land/src`
- THEN one of them calls `clm_git::fetch_retrying`, and none defines `fn fetch_retrying`

#### Needs
- vcs/fetch-retries

#### Serves
- architecture/reuse-first

### Requirement: overlap-recheck
A `clm land` attempt whose merged tree's code key no `checked` record names with the land's command, directly or through the settings-free key (`settings-unchanged-is-checked`), SHALL check only the overlap when all of these hold: `[land] check` is set; a `checked` record with the same command names the code key of the branch's head `H`; one names the code key of the fetched base `B`; and `H` doesn't contain `B`. It SHALL run `clm affected --overlap <H> <B>` (`check.affected/overlap-selection`) in the tree on the merge commit, and then:
- when it prints nothing, run no `[land] check`, take neither the lock on `.clm/land.lock` nor the one on `.clm/check.lock`, run `clm check` with its detectors on the merge commit in a temporary worktree as an attempt with no lock does, and print `re-check: none, the branch and main since <short merge base> affect no test in common` on stderr;
- when it prints a line, go on as an attempt whose code key isn't checked, with `CLM_SELECTION` naming a file holding that output unchanged in place of `selected-tests`' selection, and print `re-check: overlap, <n> files` on stderr, `<n>` the number of its `file` lines, or `re-check: overlap, every test` when it prints an `all` line;
- when it fails, print `clm affected failed: <the first non-empty line of its stderr, or the spawn error>; the land checks its own selection` and go on as `selected-tests` says.

When `clm check` and any `[land] check` it ran pass, the land SHALL append a `checked` record for the merged tree's code key with the land's command and a `basis` naming the two code keys and the selected files (`ledger/composed-check`), and its `landed` record SHALL say how it re-checked (`ledger/landed-recheck`): `recheck: none`, or `overlap` with `recheck_files` the number of `file` lines, null for an `all` line. Every other land SHALL say it too: `checked` when its code key counted as checked, and `selected` when it ran `selected-tests`' selection, with `recheck_files` counted the same way, null when every test ran. The gate on the merge commit and the push SHALL run as for any land. An attempt for which any condition fails SHALL run as before this rule.

#### Scenario: main moved on an unrelated component
- GIVEN components `a` and `b` with no `depends_on` between them, a `[land] check` that appends to a log outside the repository, a branch `impl/a` whose head changes only `crates/a/src/lib.rs` and has a `checked` record, and `main` moved since by a land changing only `crates/b/src/lib.rs`, whose code key has a `checked` record
- WHEN `clm land impl/a` runs while another process holds the lock on `.clm/check.lock`
- THEN it prints `landed`, without waiting, the log is empty, stderr holds `re-check: none`, and the ledger has a `checked` record for the merged tree's code key whose `basis` names both keys and no file

#### Scenario: a dependant of both
- GIVEN the same, and a component `c` with `depends_on: [a, b]` and a test `crates/c/tests/t.rs`, and a `[land] check` that copies the file `CLM_SELECTION` names
- WHEN `clm land impl/a` runs
- THEN the copy holds `file crates/c/tests/t.rs` and no file of `a` or `b`, stderr holds `re-check: overlap, 1 files`, and the `checked` record's `basis` names that file

#### Scenario: the head was never checked whole
- GIVEN the first scenario's repository with no `checked` record for the head
- WHEN `clm land impl/a` runs
- THEN the check runs with `clm affected --base`'s selection, as `selected-tests` says, and stderr holds no `re-check:` line

#### Scenario: main's head not checked
- GIVEN the first scenario's repository, with `main`'s head a commit pushed outside `clm land`, whose code key no `checked` record names
- WHEN `clm land impl/a` runs
- THEN the check runs with `clm affected --base`'s selection and stderr holds no `re-check:` line

#### Needs
- check.affected/overlap-selection
- ledger/composed-check
- ledger/landed-recheck

### Requirement: land-dir-env
Every run of `[land] check` by `clm land` that holds the lock on `.clm/land.lock` SHALL set `CLM_LAND_DIR` in the command's environment to the `.clm/land` directory of the main worktree's CLM root, the place `shared-land-target` names, replacing any value the caller's environment holds, so a project's check can keep its build output there whatever its toolchain. Every other run of `[land] check`, by a land that holds no such lock or by `clm land --check-only`, SHALL run with `CLM_LAND_DIR` removed from its environment. Until `backlog/land-still-sets-cargo-variables` is resolved, `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL` SHALL still be set as `shared-land-target` says, the code that sets them marked `clm:remove-by 20261108-toolchain-neutral-land-builds`.

#### Scenario: a land in the land tree
- GIVEN a `[land] check` that writes `$CLM_LAND_DIR` to a file outside the repository, a branch `impl/a` with no `checked` record, and `CLM_LAND_DIR=/elsewhere` in the caller's environment
- WHEN `clm land impl/a` runs from a linked worktree
- THEN the file holds the path of the main worktree's `.clm/land`

#### Scenario: a check-only run
- GIVEN the same `[land] check` and `CLM_LAND_DIR=/elsewhere` in the caller's environment
- WHEN `clm land --check-only` runs
- THEN the file holds an empty line
