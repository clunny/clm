---
component: orchestrator.run
base: { design: "e42bda19186b3551" }
---

## MODIFIED Design
Crate `clm-run`. The queue, the claim and the pending list are `orchestrator`'s; the bundle the prompt points at is `context`'s. The check command the prompt names is `config`'s `[land] check`, read with its `load` from the checkout `clm run` runs in. This component owns what the session is told: the prompt's text, its constants (the superpowers pin, the skills it names, the requirement budget) and the resume logic.

`clm run` prepares builds as `builds-in-subagents` and `unattended-superpowers` decide; it starts no session itself.
- **Where a build runs.** `build` launches each build with the Agent tool's `isolation: worktree`, so Claude Code gives the subagent its own worktree, starting from `origin/main`, in which every Bash call and every subagent it launches starts: a build can't stray into the human's checkout, even by a relative path. `clm run` makes no worktree. It leaves a local branch `impl/<component>` at the claim, tracking `origin/impl/<component>`, which the build switches to, and keeps `.clm/` (the plan and the pull request body) out of git through `.git/info/exclude`, which every worktree shares. CLM finds a worktree by the branch it has checked out (`git worktree list --porcelain`), never by its path. Claude Code puts an `isolation: worktree` subagent's worktree under the main worktree's `.claude/worktrees/` (`agent-*`), even when the launching session runs in a linked worktree. So `clm run --finish` looks for the build's worktree under the main worktree's `.claude/worktrees/`, whichever worktree it runs in.
- **Resume and leftovers** (`merged-claims-released`, `run`). A component comes back after its branch landed whenever its spec moves, so an earlier build's local branch `impl/<component>`, or a worktree that has it checked out, may still be there; `run` says what `clm run` removes, resumes and refuses (`git worktree remove` after unlocking, since Claude Code locks its worktrees). The landed test comes first, so a landed branch is never resumed. It comes before the place because of how a place is judged: `orchestrator`'s Design says a build's place has landed when each `impl/<component>` that exists has its tip in `origin/main`, and `clm land` deletes `origin/impl/<component>` but not the local branch, so a place taken while a landed local `impl/<component>` is still there would read as landed, and count for nothing, until the new claim is pushed; another command taking a place in that window would delete it and could take the last free one, running more than `max_builds` builds. With the landed branch gone the component has no branch, which a place reads as not landed, so the new place counts by its `taken` time, and nothing new is recorded. A landed leftover holds nothing `origin/main` lacks, so removing it is harmless even when the run then finds no free place. A dirty worktree refuses the run before any place is taken, since that check writes nothing. A landed `origin/impl/<component>` that still exists, which happens only when `clm land`'s delete of it failed or the branch reached `main` some other way, still makes the new place read as landed until the claim is pushed: that window is accepted, since closing it would take a push before the place. **An interrupted launch** (`claim-local-before-push`). `clm run` claims through `orchestrator`'s `claim_local`, which makes the local branch at the claim commit before pushing it, so no interruption leaves a pushed claim with no local branch. Interrupted after the push, the local branch is the claim's tip, so the next run resumes it by the rule below, whether the claim is still live or has lapsed. Interrupted before the push, or after a push whose outcome `claim_local` couldn't read, the local branch's tip is an *unpushed claim*: a commit with one parent, the same tree as that parent, the parent an ancestor of `origin/main`, on a branch that has never held `origin/impl/<component>`'s tip as below (or with no such branch). It holds no work, so the leftovers step removes it, like a landed branch, when no worktree has it checked out, and the run then claims as usual; when another clone has claimed the component meanwhile, that claim refuses the run as for any live claim. The test is by structure, not by the commit's message: a commit that adds nothing to a tree on `main` holds nothing to lose, whoever made it. Another clone's claim is never resumed, since this clone has no local branch at it. **A stopped build's unpushed work** (`resume-what-push-accepts`). A build rewrites its branch at every rebase point (`clm rebase` rebases it onto `origin/main`, the claim commit included) and only then pushes it, so a build that stops between a rebase and its push leaves a local `impl/<component>` that no longer contains `origin/impl/<component>`, though it holds what that tip held and the build's newer commits. `clm run` resumes such a branch when the local branch has held `origin/impl/<component>`'s tip: the tip is an ancestor of the local branch's tip or of one of the entries of its reflog (`git reflog` of `refs/heads/impl/<component>`), the test git's `--force-if-includes` makes. It is the test `clm branch push` passes before it overwrites the remote branch (`host`'s `branch-push`), so every branch `clm run` resumes is one the resumed build's next push accepts while `origin` stays where the fetch found it, and a branch the push would refuse is never resumed: a branch whose `origin` tip this clone never held is another clone's, or misses commits pushed from elsewhere, and resuming it would only fail at the build's first push. Containing the tip is the case where nothing was rewritten. Every value the branch has held is the new value of one of its reflog entries, the tip being the newest, so listing those (`git reflog show --format=%H`) and testing each with `git merge-base --is-ancestor` is the whole test. A build that stopped after rebasing a bare claim, before any work, leaves a tip shaped like an unpushed claim; since the branch held `origin/impl/<component>`'s tip, it isn't one, so it is resumed like any other held branch rather than deleted and then refused by its own live claim. A branch's reflog is kept in the git common directory, so every worktree of the clone reads the same one; a branch deleted and made again starts a new one. A local `impl/<component>` that isn't landed and has never held `origin/impl/<component>`'s tip, or has no `origin/impl/<component>` at all, may hold commits nobody pushed (other than an unpushed claim) or miss commits pushed from elsewhere, so `clm run` leaves it and exits 1 naming the branch and saying that it has never held `origin/impl/<component>`'s tip; only someone who can tell whose commits they are can reconcile or delete it, so `build` tells the human what it printed and launches nothing for the component, as for any exit 1. `clm run` trusts that no build of the component is running: `build` calls it only for components `clm next` lists, which have no live claim, and for its own builds once they have returned. A worktree with uncommitted changes or untracked files that aren't ignored may hold work nobody committed.
- **Superpowers.** Builds follow superpowers at a pinned release. The pin is `crates/clm-run/superpowers.pin`, one line `<tag> <commit>`, which `clm-run` reads at compile time with `include_str!` into its `SUPERPOWERS_TAG` and `SUPERPOWERS_COMMIT`; the copy is fetched once into `${XDG_CACHE_HOME:-~/.cache}/clm/superpowers/<commit>`. Moving the pin is a one-line edit of that file, made by hand on an `impl/orchestrator.run` branch after reading the release's notes and landed through `clm land` like any code (`cut-evals`); nothing runs an eval or watches for releases. The skills the prompt names are a constant: `superpowers:writing-plans`, `superpowers:test-driven-development` and `superpowers:verification-before-completion`. Changing the skills is a code change. `$CLM_SUPERPOWERS_DIR` overrides the pin, for tests and for trying a release before moving the pin. `clm superpowers <skill>` resolves the same directory, the override included, and prints `<dir>/skills/<skill>/SKILL.md`. A build runs in the human's session and has its plugins, not the pin, so the prompt gives each skill it names with the path `clm superpowers <skill>` prints, and says to read and follow that file and never to invoke an installed copy.
- **Prompt file** (`build-prompt-file`). The prompt holds the component's whole `clm context` bundle, tens of KB, so `clm run` writes it to `<root>/.clm/prompts/<component>.md`, where `<root>` is the checkout `clm run` runs in; `.clm/` is already outside git. On stdout it prints only the launcher, a few lines: read `<absolute path>` with the Read tool, all of it (in parts when it is too large to read at once), before anything else, and follow it as your instructions; it is the prompt for building `<component>`, and its first step is `git switch impl/<component>`. The orchestrating session passes the launcher unchanged as the Agent tool's prompt, so it never reads or repeats the bundle, and the subagent, whose worktree is elsewhere, reads the file by that absolute path. The file stays in the orchestrator's checkout, which no build removes, until the next `clm run` of the component replaces it; a refused run leaves an earlier file alone, and with no launcher printed nothing reads it. The prompt depends on the claimed tree, so it is written after the claim. **The tree it is read from** (`pending-prompt`) is the merge of `origin/main` and `impl/<component>`, as `git merge-tree --write-tree origin/main impl/<component>` writes it after the fetch, the way `land` and `gate` compute a merged tree (`reuse-first`); it writes only objects, so the session's checkout, which `build` never brings up to date (`views-name-their-commit`), is never read. For a fresh claim the merge is the claim's own tree, since the claim's parent is `origin/main`. For a resumed branch cut from an older `main`, it holds what has landed since, so a requirement another component's build has made current since then does not block, and the lists agree with `clm next`, which reads `origin/main`. The branch never edits `specs/`, so the merge's specs are `origin/main`'s, and the earlier texts of a `behind` requirement are looked up in `origin/main`'s history of the component's `SPEC.md`. When the merge conflicts, the lists come from `origin/main`'s tree, which lacks the branch's own markers, so the requirements the branch built show as pending: the prompt says the lists were read from `origin/main` because the branch conflicts with it, and that `clm pending <component>` after the first `clm rebase` is the list to build from; `.clm/prompts/` is created before the claim, beside the exclude, so a missing or unwritable directory refuses the run before anything is claimed, and a write that still fails after the claim leaves the claim to the next `clm run`, which resumes it. The launcher is a fixed template.
- **Places** (`build-places`, `run-takes-place`). `clm run` takes a place named `<component>` with `orchestrator`'s `take_place`, its branch `impl/<component>`, with `max_builds` from `config`'s `load` of the checkout it runs in and its `--expiry`. A place `<component>` that counts is reused, so a build that resumes its component's branch, or a retry after a build stopped, holds the place it had; otherwise a new place needs one to be free. The exit-3 message on stderr is `no free build place: <held> of max_builds = <n> in use` (only `max_builds`, its value and the number held are binding; a hint may follow), and an exit 3 leaves nothing behind but the landed leftovers it removed: no claim, no new branch, no exclude line, no prompt. A build of another component that is already running holds its own place, so `build` can't start more builds than the cap however many components `clm next` printed before.
- **Finishing a build** (`build-places`, `run-finish`). `clm run --finish <component>` frees the place with `free_place`, printing `freed <component>` when there was one. A branch has landed by the place's test (`orchestrator`'s Design: every one of `origin/impl/<component>` and the local `impl/<component>` that exists has its tip in `origin/main`, and one of them exists). A worktree is unlocked first, since Claude Code locks the worktrees it makes, ignoring the error for one that isn't locked, then removed with `git worktree remove`. A worktree elsewhere, such as the human's own checkout, is never touched. When the branch hasn't landed, the claim, the local branch and the worktree stay, so a retry resumes them. It doesn't delete the local branch: `clm wrap-up`, or the next `clm run <component>`'s leftovers step, does. It reads no queue state, so it works for any component, converged or removed. The orchestrating session runs it rather than `git worktree remove` because the build's worktree is another worktree, where a worktree-isolated session may not run git, and the build's own session can't remove the worktree it runs in; `clm land` has already deleted that worktree's `target/` (`land`'s `land-frees-build-target`). **What a build left unwritable** (`run-finish`). A build can leave an ignored directory without its owner's write permission, for instance by unpacking an archive; `git status` still reads the worktree as clean, but `git worktree remove` can't delete what that directory holds. So `--finish` removes a worktree it judged clean with `vcs`'s `remove_worktree`, which unlocks it, makes it removable without following a symbolic link and runs `git worktree remove` (`vcs`'s Design), and holds no removal of its own: `clm wrap-up` removes build worktrees through the same function. What git still can't remove (a file owned by another user, a worktree git refuses) is reported, not failed on: the line `left <path>: <error>; delete the directory by hand, then run git worktree prune` replaces the `removed` line and names the path, the function's error joined onto one line (`<path>: still on disk` when git succeeded but the directory remains), and the steps that clear it whether or not git has already unregistered the worktree, and `--finish` goes on to the other worktrees and exits 0, so the `freed` and `landed` lines it gathered are printed. Only a worktree's removal is reported this way; any other git failure still exits 2.
- **Own target** (`build-places`). The prompt doesn't name `CARGO_TARGET_DIR`, so a build compiles into its own worktree's `target/`, its check command included, and the plain check command (`prompt-check-command`) needs no variable. A target directory shared between checkouts is wrong, not only slow: cargo names a workspace crate's outputs by its path relative to the workspace, so every worktree of a repository writes the same files there, and it judges them fresh by file times, so a build whose untouched sources are older than another build's last compile would test that build's code. When `CARGO_TARGET_DIR` is set in the human's session, a build inherits it, as any command does; CLM sets nothing.
- **Prompt.** The prompt's fixed text is source in `crates/clm-run/src/`.
  - **Unattended preamble.** The session works alone: no questions in the conversation, a `question` or `human-task` backlog item for anything only the human can answer or do, and a note that Claude Code may ask the human to confirm a tool call, which the build waits for.
  - **The job** (`pending-prompt`, `removed-requirements-build-out`). The pending requirements with their versions, what blocks each blocked one and the requirement budget per run, then the step plan, which is executed in the same session.
  - **Removals another component still uses** (`contract-removal-needs`). A build never fixes another component's code or tests to absorb its own change (`coupled-changes-sequence`): each user moves in a requirement of its own, and when an optional component must move off a required one's removal, which no required component may `Need` (`optional-nodes`), the removal waits as a contract step (`contract-step-deferred`). The kept code of an `obsolete` requirement gets no `clm:remove-by` marker, since the removing change may archive first: the item records it. The root's dispatch files (`crates/clm/src/main.rs`, `Cargo.toml`, `Cargo.lock`) are not another component's code for this rule: the root's Design gives every build that adds or changes a command those edits.
  - **Rebase points** (`versioned-markers`). After each step of its step plan, the session commits, runs `clm rebase`, resolves what it reports until it prints `rebase ok`, and pushes with `clm branch push` (`host/branch-push`), which also keeps its claim alive. Before landing it does the same. A push the remote rejects for a transient reason is usually caused by other builds pushing at the same moment, so the session pushes again at most twice more (three pushes in all, as `orchestrator/claim` does), before it stops and reports git's message. This covers every push the session makes, and the prompt says it once, where it first names `clm branch push`. A hook that declines the push and a lease rejection are never retried that way: the first is a decision, and the second means the branch moved, which `clm rebase` handles. When `clm rebase` reports `spec changed <component>` (`sync/spec-changed`), the session reads the lines that follow, runs `clm pending <component>`, and adapts its work to the new text or discards and rebuilds the part it affects, whichever fits; it records what changed and which way it went under `## Sync` in the pull request body. Nothing is re-planned: the pending list is derived from the markers each time. A change `clm rebase` archived on the branch (`sync/archive-complete`) is committed with the rest.
  - **Checks** (`check-before-land`, `prompt-check-command`). The prompt tells the session to pass the repository's checks in two places: after fixing the review's findings, and before landing. Both name `clm land --check-only` (`land/check-only`); the first also gives the one-plain-command rule, which is about that one command, so the prompt still says nothing general about `cd` or one command per call. The prompt never tells the session to run the configured command by its path: Claude Code's worktree isolation, which refuses a command it can't show stays inside the worktree, refused `.github/scripts/ci-check.sh` in a build, and `clm land --check-only` names no path for it to judge (`land-check-command`). `clm land --check-only` is a `clm` command, which never gets `CARGO_TARGET_DIR` (`build-places`), and it sets none either, so the check compiles into the worktree's own `target/` as every other command of the build does. A failing check is fixed and run again. When Claude Code refuses or denies the command, the session lists the refusal under `## CLM defects` in the pull request body and lands with `clm land`, which runs the whole command on the merged tree and refuses the land when it fails (`defect-reports`). The session may run parts of the check while it works, to fail faster, but never reports them as the check. **The recorded check** (`worktree-check-counts`). A passing `clm land --check-only` on a clean, committed `HEAD` that contains `origin/main` appends a `checked` record (`land/check-only`), so a land made before `main` moves runs no check of its own and takes no lock. The ship step already orders it so: commit everything, `clm rebase` until `rebase ok`, the check, then the gate, the push, the pull request body (outside git) and `clm land`, none of which moves `HEAD` or touches a tracked file. The prompt says why that order matters, so a session doesn't commit between the check and the land, and says that `not recorded:` costs only the land's own check. Because the check runs in the build's own place, outside the clone's check lock (`land/check-lock`), it starts at once, but it still runs every test; so the prompt has the session run the formatter and linter the check runs, and the tests of the code it changed, by hand first, catching a slip in seconds. When the repository sets no `[land] check`, the prompt says "the repo's checks" and names no command. `clm run` reads the setting before it claims, so a `clm.toml` that `load` rejects makes it exit 1 with `config`'s message, claiming nothing and printing nothing on stdout.
  - **Setup** (`prompt-setup`). When `clm.toml` sets `[build] setup`, the prompt has the session run `clm setup` right after it switches to its branch and checks it, before any test run, and again whenever what the setup installs from may have changed: after its own edit to the project's dependency declarations, and after a `clm rebase` that brought such an edit in from `origin/main`. A setup that fails because of the branch's own edits is fixed like a failing check; one that fails otherwise, or that Claude Code refuses, leaves no test result the session can trust, so it reports what failed (`clm defect` for a refusal) and stops. When `clm.toml` sets none, the prompt says the repository declares no setup, and that a test run that resolves code or dependencies from outside the worktree is a defect of the project, which the session records as a `bug` item naming `tests-run-in-their-own-tree` and works around by installing what its tests need into its own worktree by hand before it trusts a red or a green.
  - **Workaround** (`workaround-flags`, `prompt-workaround`). The prompt is read by whatever `clm` the session has, which may lack the command (`dogfood-from-main`'s older build, a pinned release), so it says what to do when the binary on `PATH` lacks `clm workaround`. A flag is evidence (`workaround-flags`), so a missing flag command is never a reason to stop; the shared-binary rule's stop (below) is for commands the work itself needs. That is why `prompt-workaround` has no `#### Needs` on `check.health/workaround-command`: the prompt is right against any binary, and the need would have held a built requirement behind a command whose absence changes nothing about the build. A flag whose requirement moves under the build (`clm rebase` reports `spec changed` and `clm check` rejects the flag at its old version) is re-recorded at the new version, or deleted when the new text removes the workaround: the build added the file, and editing its own addition keeps the branch a code branch. The reconsider questions the flags may lead to are filed by `clm rebase`, never by the session.
  - **Shared binary.** A build that replaced the `clm` binary on its `PATH` would change the commands the human's session, which orchestrates the builds, and the other builds rely on mid-session. The installed `clm` changes only outside builds, such as the main checkout's rebuild after a merge (`dogfood-from-main`).
  - **Pull request and landing** (`own-pr-body`, `land-locally`, `prompt-pr-body`). The pull request is opened before landing so the human has a diff to read wherever the host shows one. `Write` replaces `.clm/pr-body.md`, so a body left by an earlier run never carries over. The body has `## Plan`, `## Review`, `## Sync`, `## Workarounds`, `## Fixes in other components`, `## Left pending` and `## CLM defects`. The session goes by `clm land`'s last line: `landed <sha> ...` ends the build; `gate/up-to-date` or a conflict means `clm rebase`, resolve, push and land again; a check failure is fixed and landed again; anything else is reported.
  - **Resume.** When `clm run` resumes a branch that has commits `origin/main` lacks, the prompt says so: the session starts with `clm rebase`, handles what it reports as above, and lands the same branch. The branch's pull request, if one was opened, is found by `clm pr open`, which prints the open pull request's URL instead of opening another.
  - Every land goes to `main`.
- **Review** (`advisory-code-review`). Before landing, a session reviews its diff with two fresh-context reviewers: `/code-review` at a fixed level for bugs, and the `clm` plugin's `spec-auditor` subagent for the spec, given the component. It fixes the findings it agrees with, re-runs the checks, and lists every finding with what it did about it under `## Review` in the pull request body. The level and the reviewer names are constants in `clm-run`, like the pin. The review advises and never gates, but it always runs: a session judging its own diff too small to review (only markers, two tests, a few lines) is the judgment the fresh-context reviewers exist to replace, and a diff of markers alone is where `spec-auditor` finds a test that cites a requirement it doesn't prove. No program checks that it ran: the record under `## Review` is the visibility `advisory-code-review` chose, and the prompt says plainly that no diff is exempt. The reviewers start in the build's worktree, as every subagent a build launches does, so they read its diff.
- **Permissions** (`builds-in-subagents`). A build runs under the permission mode of the human's session, which Claude Code doesn't let a subagent override, so `clm run` sets none and ships no allowlist. In manual mode a build's permission prompts reach the human's session. Claude Code asks the human before writing a protected path (`.git/`, `.claude/`, `.npmrc` and the rest of its list) in every mode, and that build waits for the answer while the others carry on. The guard runs on a build's calls through the `clm` plugin's `PreToolUse` hook, which Claude Code runs for subagents (`guard-blocks-push-to-main`). In auto mode Claude Code's classifier judges each command no rule allows, and it denied a raw force-push of a rebased branch as destructive, stopping builds with a checked branch they couldn't land. So every step that writes to `origin` is a `clm` command (`unattended-steps-run-clm`): the build pushes with `clm branch push` and lands with `clm land`, and the project's allow rules, which `clm init` seeds (`init/init-seeds-permissions`), let those run without the classifier. The prompt names no `git push`, so no rule for a raw push is ever needed, and the prompt itself needs no rule and states none: the `build` skill checks the project's rules before the first build.
- **Setup** (`tests-run-in-their-own-tree`, `build-setup-step`, `setup-command`). A build's worktree is fresh, under `.claude/worktrees/` inside the main checkout, with nothing installed, so a toolchain whose lookup walks up the parent directories, or an environment the checkouts share, would find the main checkout's packages or source. `clm setup` runs the command `[build] setup` names, which the project owns and which holds whatever its toolchains need; CLM names no toolchain. It loads the settings with `config`'s `load` from the CLM root found from the current directory, as every command's `--root` defaults (`clm`'s Design), and runs the command through `sh -c` in that root, with its standard input closed and its output passed through, as `clm land --check-only` runs `[land] check`, but under no lock and setting no variable: it installs, it doesn't test, so it takes no host resource another run shares. It is a `clm` command rather than the command itself in the prompt for the reason `prompt-check-command` gives: Claude Code's worktree isolation refuses a script path it can't show stays in the worktree, and `clm setup` names none. With no `[build] setup` it prints `no [build] setup` and exits 0, so a session or a human can run it in any repository. `[land] check` never runs it: CI, `[verify]` and a land's own tree run the check in trees no build prepared, so the check stays a command that needs nothing before it.
- **Parallelism.** Each build has its own worktree, so `build` builds several components at once, at most `max_builds` (above). Components' files are disjoint (`paths`, with `tests` carved out of them), so two builds edit the same files only under `path-ownership`'s exceptions: a capability test, a template's `upgrade` migration, re-citing a test in a file no `tests` glob owns, and the root's dispatch files; `clm land`'s merge of `main` then reconciles like any other conflict.

## MODIFIED Requirements

### Requirement: run <!-- base: 6635d3892b92075a -->
`clm run <component>` SHALL first remove leftovers of an earlier build of the component that are already landed: a worktree that has `impl/<component>` checked out, has no uncommitted changes or untracked files that aren't ignored, and whose `HEAD` is an ancestor of `origin/main`, unlocking it first; and then a local branch `impl/<component>` that no worktree has checked out and whose tip is an ancestor of `origin/main` or is an unpushed claim: a commit with one parent, the same tree as that parent, the parent an ancestor of `origin/main`, on a branch that has never held `origin/impl/<component>`'s tip as below, or with no `origin/impl/<component>`. When the local branch `impl/<component>` then exists and isn't landed, it SHALL exit 1 naming the branch, without claiming, unless `origin/impl/<component>` exists and the local branch has held its tip, which is so when that tip is an ancestor of the local branch's tip or of one of the entries of the local branch's reflog, the test git's `--force-if-includes` makes, in which case it SHALL resume without claiming, first removing a worktree that has the branch checked out when that worktree has no uncommitted changes or untracked files that aren't ignored, unlocking it first. Otherwise it SHALL claim the component through `orchestrator`'s `claim_local`, so that the local branch `impl/<component>` is at the claim commit, tracking `origin/impl/<component>`, before the claim is pushed, exiting 1 when the claim is refused and 2 when it fails in git. When a worktree that has `impl/<component>` checked out has uncommitted changes or untracked files that aren't ignored, it SHALL exit 1 naming its path, without claiming. It SHALL keep `.clm/` out of git through `.git/info/exclude`, create no worktree, and start no `claude` process.

It SHALL then write the prompt to `.clm/prompts/<component>.md` under the directory it runs in, replacing any earlier file, print on stdout only a launcher of at most five lines that names that file by its absolute path, says to read all of it before anything else and follow it, and names `git switch impl/<component>` as the file's first step, and exit 0. It SHALL create `.clm/prompts/` before claiming, refusing with exit 1 when it can't; when writing the file fails after the claim, it SHALL exit 1 naming the path. The prompt's first line SHALL say to run `git switch impl/<component>`. It SHALL name the skills `superpowers:writing-plans`, `superpowers:test-driven-development` and `superpowers:verification-before-completion`, each with the path `clm superpowers <skill>` prints; SHALL name the plan path `.clm/plans/<component>.md`; SHALL say when resuming a branch with unlanded commits that the branch exists; and SHALL end with the `clm context` bundle. When it exits 1 or 2, it SHALL print nothing on stdout.

#### Scenario: node prepared
- GIVEN a ready component `b`, `CLM_SUPERPOWERS_DIR` set to a directory holding the three skills, and a fake `claude` on PATH that records whether it ran
- WHEN `clm run b` runs
- THEN it exits 0, `origin/impl/b` exists, the local branch `impl/b` is at the claim and tracks it, no worktree has `impl/b` checked out, the fake never ran, stdout is at most five lines, names the absolute path of `.clm/prompts/b.md` and `git switch impl/b`, and doesn't contain `b`'s context bundle, and that file's first line names `git switch impl/b`, and the file has the three skill names each with its `SKILL.md` path under `CLM_SUPERPOWERS_DIR` and the plan path `.clm/plans/b.md`, and ends with `b`'s context bundle

#### Scenario: claimed elsewhere
- GIVEN `origin/impl/b` committed 1 hour ago, and no local branch `impl/b`
- WHEN `clm run b` runs
- THEN it exits 1 and prints nothing on stdout

#### Scenario: resume with unlanded commits
- GIVEN a clean worktree left by an earlier build with `impl/b` checked out, its branch pushed with a commit `origin/main` lacks
- WHEN `clm run b` runs
- THEN no claim commit is pushed, that worktree is gone, the local branch `impl/b` still has the commit, and the prompt it writes says the branch exists

#### Scenario: landed leftover worktree
- GIVEN a clean worktree with local branch `impl/b` checked out, left by an earlier build whose branch landed on `origin/main` with a merge commit, with `origin/impl/b` still at that tip, the worktree locked, and a ready component `b`
- WHEN `clm run b` runs
- THEN the worktree is gone, it pushes a new claim commit whose parent is `origin/main` to `origin/impl/b`, the local branch `impl/b` is at that commit, and it exits 0

#### Scenario: landed leftover branch
- GIVEN a local branch `impl/b` with no worktree, landed on `origin/main`, no `origin/impl/b`, and a ready component `b`
- WHEN `clm run b` runs
- THEN it claims `b`, the local branch `impl/b` is at the claim, and it exits 0

#### Scenario: unpushed leftover branch
- GIVEN a local branch `impl/b`, whose reflog never held `origin/impl/b`'s tip, with a commit that changes a file and that neither `origin/main` nor `origin/impl/b` has, where `origin/impl/b` is a lapsed claim, and a ready component `b`
- WHEN `clm run b` runs
- THEN it exits 1 naming `impl/b`, pushes no claim commit, prints nothing on stdout, and `impl/b` still has the commit

#### Scenario: stopped after a rebase
- GIVEN a ready component `b` whose claim this clone pushed, a clean, locked worktree at `.claude/worktrees/agent-1` with the local `impl/b` checked out, and the local `impl/b` then rebased onto a newer `origin/main`, so that it no longer contains `origin/impl/b`, and given a commit that changes a file, none of it pushed
- WHEN `clm run b` runs
- THEN it exits 0, pushes no claim commit, `origin/impl/b` is unchanged, that worktree is gone, the local `impl/b` still has the commit, and the prompt it writes says the branch exists; and `clm branch push`, run on that branch, then pushes it

#### Scenario: claim rebased before any work
- GIVEN a ready component `b` whose claim this clone pushed, the local `impl/b`, checked out nowhere, rebased onto a newer `origin/main` and holding nothing else, so that its tip is a commit with one parent, the parent's tree, and not contained in `origin/impl/b`
- WHEN `clm run b` runs
- THEN it exits 0, pushes no claim commit, `origin/impl/b` is unchanged, and the local `impl/b` is still that commit

#### Scenario: interrupted after the push
- GIVEN a ready component `b` whose claim this clone pushed 1 minute ago, with the local branch `impl/b` at that claim commit, checked out nowhere, and nothing else done
- WHEN `clm run b` runs
- THEN it exits 0, pushes no claim commit, the local `impl/b` and `origin/impl/b` are still that commit, and it writes `b`'s prompt

#### Scenario: interrupted before the push
- GIVEN a ready component `b`, no `origin/impl/b`, and a local branch `impl/b`, checked out nowhere, at a commit whose parent is `origin/main` and whose tree is `origin/main`'s
- WHEN `clm run b` runs
- THEN it exits 0, `origin/impl/b` is a new claim commit whose parent is `origin/main`, and the local `impl/b` is at it

#### Scenario: another clone claimed meanwhile
- GIVEN the local `impl/b` of "interrupted before the push", and `origin/impl/b` another clone's claim committed 1 hour ago
- WHEN `clm run b` runs
- THEN it exits 1, prints nothing on stdout, and `origin/impl/b` is unchanged

#### Scenario: dirty leftover
- GIVEN the leftover worktree as in "landed leftover worktree", with an untracked file `notes.txt`
- WHEN `clm run b` runs
- THEN it exits 1 naming the worktree's path, pushes no claim commit, prints nothing on stdout, and `notes.txt` is still there

#### Scenario: node that writes a protected path
- GIVEN a ready component `w` whose `paths` are `["web/**"]` and whose pending requirement names `web/.npmrc`
- WHEN `clm next` and `clm run w` run
- THEN `clm next` prints `w`, and `clm run w` exits 0 and writes `w`'s prompt

#### Needs
- clm/renamed-commands
- orchestrator/claim-local

### Requirement: pending-prompt <!-- base: dea4856a6d9878e5 -->
The `clm run` prompt SHALL list the component's pending requirements, most urgent first, each with its ID, version and state; for a `behind` requirement, the diff from the text at each version its markers cite other than the current one to the current text, taken from the history of the component's `SPEC.md`, or a line saying the earlier text wasn't found, and the locations of the markers that cite it at another version or without one, saying to re-cite each one wherever its file sits, another component's `paths` included, after checking that its test proves the current text (`path-ownership`, `stale-markers-keep-behind`); and for an `obsolete` one, the locations of its markers, saying to remove the code and tests they mark. It SHALL leave blocked requirements out of the list and name them with what blocks each: the backlog items, or the pending requirement they need. It SHALL take the lists, the earlier texts and the context bundle from the tree `git merge-tree --write-tree origin/main impl/<component>` writes once `clm run` has fetched, never from the checkout it runs in, so that a requirement made current on `origin/main` since a resumed branch was cut neither shows as pending nor blocks another; when that merge conflicts, it SHALL take them from `origin/main`'s tree and say in the prompt that they were read from `origin/main` because the branch conflicts with it, and that `clm pending <component>` after the first `clm rebase` is the list to build from.

#### Scenario: behind and obsolete
- GIVEN component `b` whose requirement `b/x` is cited at version `11111111`, which `specs/b/SPEC.md` had at an earlier commit, and is now at `22222222`; whose requirement `b/y` is uncited; and a marker at `t.rs:9` citing `b/gone`, which a live change removed
- WHEN `clm run b` runs
- THEN the prompt it writes lists `b/x` as `behind` with a diff from the earlier text to the current, `b/y` as `unbuilt`, and `b/gone` as `obsolete` at `t.rs:9`

#### Scenario: stale marker in another component's files
- GIVEN component `b` whose requirement `b/x` is at version `22222222`, cited at `crates/b/t.rs:3` at `22222222` and at `crates/c/t.rs:8` at `11111111`, which `specs/b/SPEC.md` had at an earlier commit
- WHEN `clm run b` runs
- THEN the prompt it writes lists `b/x` as `behind` with a diff from the `11111111` text to the current, and names `crates/c/t.rs:8` as a marker to re-cite, and not `crates/b/t.rs:3`

#### Scenario: blocked requirement left out
- GIVEN component `b` with pending requirements `b/x` and `b/y`, `b/y` in the `blocks` of `backlog/q.md`
- WHEN `clm run b` runs
- THEN the prompt it writes lists `b/x` to build, and names `b/y` and `backlog/q` as blocked

#### Scenario: needing requirement left out
- GIVEN component `b` with pending requirements `b/x` and `b/y`, `b/y` needing uncited `c/z`
- WHEN `clm run b` runs
- THEN the prompt it writes lists `b/x` to build, and names `b/y` as blocked by `c/z`

#### Scenario: resumed branch behind main
- GIVEN an earlier build's local branch `impl/b` containing `origin/impl/b`, with a commit `origin/main` lacks, cut when `b/y` needed uncited `c/z`, and a commit on `origin/main` since then adding a test that cites `c/z` at its current version; and a checkout `clm run` runs in that is at the older commit of `main`
- WHEN `clm run b` runs
- THEN the prompt it writes lists `b/y` to build and doesn't name it as blocked

#### Scenario: resumed branch that conflicts with main
- GIVEN the resumed branch of "resumed branch behind main", whose commit and a later commit on `origin/main` change the same line of one file
- WHEN `clm run b` runs
- THEN it exits 0, and the prompt it writes lists `b/y` to build, says its lists were read from `origin/main` because the branch conflicts with it, and names `clm pending b` after the first `clm rebase`

#### Needs
- clm/renamed-commands
- check.coverage/pending-report
- check.coverage/state-names

### Requirement: prompt-pr-body <!-- base: ac48f44f0d607cba -->
`clm run`'s prompt SHALL name `clm pr open impl/<component>`, with the component's own branch.

#### Scenario: prompt
- GIVEN a ready component `b`
- WHEN `clm run b` runs
- THEN the prompt it writes contains `clm pr open impl/b`

#### Serves
- capability/build-from-spec
- capability/land-safely

#### Needs
- clm/renamed-commands
- host/pr-open

### Requirement: prompt-check-command <!-- base: 6b68f4d55bb13dc4 -->
When `clm.toml` sets `[land] check`, `clm run`'s prompt SHALL tell the session, wherever it tells it to pass or re-run the repository's checks, to run `clm land --check-only`, saying that it runs that command, named as `clm.toml` spells it. It SHALL NOT tell the session to run the configured command by its path. When `clm.toml` sets no `[land] check`, the prompt SHALL NOT name a check command or `clm land --check-only`. The prompt SHALL NOT name `CARGO_TARGET_DIR` or a target directory, whatever the environment holds, so the check command and every cargo command compile into the worktree's own `target/` (`build-places`). When `clm.toml` can't be loaded, `clm run` SHALL exit 1 with the reason before claiming, printing nothing on stdout.

#### Scenario: check command named
- GIVEN a ready component `b` and a `clm.toml` whose `[land] check` is `scripts/check.sh`
- WHEN `clm run b` runs
- THEN the prompt it prints names `clm land --check-only` as the command to run and `scripts/check.sh` as what it runs, contains neither ``run `scripts/check.sh` `` nor ``with `scripts/check.sh` ``, and names `clm land --check-only` in both places it says to pass the checks

#### Scenario: no check command
- GIVEN a ready component `b` and a `clm.toml` with no `[land]` table
- WHEN `clm run b` runs
- THEN it exits 0 and the prompt it prints doesn't contain `--check-only`

#### Scenario: no target directory
- GIVEN a ready component `b`, a `clm.toml` whose `[land] check` is `scripts/check.sh`, and `clm run` run in a linked worktree of the repository, once with `CARGO_TARGET_DIR` unset and once set to `/elsewhere`
- WHEN `clm run b` runs each time
- THEN neither prompt contains `CARGO_TARGET_DIR` or `target/build`

#### Scenario: unreadable settings
- GIVEN a ready component `b` and a `clm.toml` that isn't valid TOML
- WHEN `clm run b` runs
- THEN it exits 1 naming `clm.toml`, prints nothing on stdout, and pushes no claim

#### Serves
- capability/land-safely
- capability/prove-by-tests

#### Needs
- land/check-only

### Requirement: prompt-serves <!-- base: bf9d2c222427e89d -->
`clm run`'s prompt SHALL say, only for the component that builds the capability tests (the graph's capability component, else its root), that each capability under `# Capabilities served` in its context bundle is one its tests prove at the project's public boundary.

#### Scenario: prompt
- GIVEN a ready component `b`
- WHEN `clm run b` runs
- THEN the prompt it writes doesn't contain `public boundary`

#### Scenario: capability node's prompt
- GIVEN the graph's capability component `capability`, ready
- WHEN `clm run capability` runs
- THEN the prompt it writes contains `public boundary`

#### Serves
- architecture/traced-rules
- capability/build-from-spec

#### Needs
- model/capability-tests-component

### Requirement: review-model <!-- base: 601bdd455bad1a72 -->
`clm run` SHALL take `--review-model <model>`, a model alias it passes into the prompt unchecked (`model-per-launch`). With it, the prompt SHALL tell the session to launch the `spec-auditor` subagent with the Agent tool's `model` set to `<model>`; without it, with `model` set to the alias of the session's own model, in the words `your own model`, since `clm run` has no alias to write.

#### Scenario: review model given
- GIVEN a ready component `b`
- WHEN `clm run b --review-model opus` runs
- THEN it exits 0, and the prompt's paragraph that names `spec-auditor` contains `model: "opus"`

#### Scenario: no review model
- GIVEN a ready component `b`
- WHEN `clm run b` runs
- THEN it exits 0, and the prompt's paragraph that names `spec-auditor` contains `your own model` and no `model: "`, and the prompt doesn't contain `model: "opus"`

#### Serves
- capability/right-model-per-task

### Requirement: prompt-workaround <!-- base: 70f301265c17c872 -->
`clm run`'s prompt SHALL name `clm workaround <component>/<slug>`, with the component's ID.

#### Scenario: prompt
- GIVEN a ready component `b`
- WHEN `clm run b` runs
- THEN the prompt it writes contains `clm workaround b/`

#### Serves
- capability/know-health
- architecture/redesign-before-workaround

### Requirement: capability-waits-for-changed-rule <!-- base: 3f55c108c3f8f059 -->
The `clm run` prompt for the component that builds the capability tests (`prompt-serves`) SHALL tell the session that when a capability's test can't pass because a rule it exercises is still `behind`, its rebuild not yet landed, the session stops at that capability and leaves it pending (`serving-rules-order-capabilities`). A prompt for any other component SHALL NOT hold this text.

#### Scenario: capability tests component
- GIVEN the prompt `clm run` builds for the component that builds the capability tests
- WHEN the tests read it
- THEN it says that a capability whose test can't pass because a rule it exercises is still `behind` is left pending, and it contains `serving-rules-order-capabilities`

#### Scenario: other component
- GIVEN the prompt `clm run` builds for a component that doesn't build the capability tests
- WHEN the tests read it
- THEN it holds no such text

#### Serves
- capability/build-from-spec
- capability/prove-by-tests

### Requirement: prompt-slot <!-- base: c8307c1b29ce984b -->
The `clm run` prompt SHALL name the build's slot, what `place_slot` returns for `impl/<component>` once `clm run` has taken its place (`orchestrator/place-slots`), as `CLM_SLOT=<slot>`, in every prompt, fresh or resuming.

#### Scenario: prompt
- GIVEN a ready component `b` and a place `a` with slot 1 that counts, once fresh and once resuming a branch with unlanded commits while the place `b` the first run took still counts
- WHEN `clm run b` runs
- THEN each prompt it writes contains `CLM_SLOT=2`

#### Needs
- orchestrator/place-slots

#### Serves
- architecture/tests-run-concurrently
- capability/work-concurrently

### Requirement: prompt-setup <!-- base: 15d55f22557bbf4f -->
When `clm.toml` sets `[build] setup`, `clm run`'s prompt SHALL tell the session to run `clm setup` before it runs any test, in every prompt, fresh or resuming. When `clm.toml` sets no `[build] setup`, the prompt's own text, outside the context bundle it embeds, SHALL NOT name `clm setup`, and SHALL tell the session, in the words `declares no setup step`, that the repository has none.

#### Scenario: setup declared
- GIVEN a ready component `b` and a `clm.toml` whose `[build] setup` is `scripts/setup.sh`
- WHEN `clm run b` runs
- THEN the prompt it writes names `clm setup` before the first place it tells the session to run a test

#### Scenario: resuming
- GIVEN the repository of "setup declared" and an earlier build's local branch `impl/b` that `clm run b` resumes, with a commit `origin/main` lacks
- WHEN `clm run b` runs
- THEN the prompt it writes names `clm setup`

#### Scenario: nothing declared
- GIVEN a ready component `b` and a `clm.toml` with no `[build]` table
- WHEN `clm run b` runs
- THEN the prompt's own text, outside the context bundle, doesn't contain `clm setup`, and the prompt contains `declares no setup step`

#### Needs
- config/build-setup-setting

#### Serves
- architecture/tests-run-in-their-own-tree
- capability/build-from-spec

## REMOVED Requirements

### Requirement: prompt-commands <!-- base: 9008204d64876107 -->
### Requirement: ask-by-backlog <!-- base: 7db6f6f961dca5c2 -->
### Requirement: delete-fixed-items <!-- base: 7f2d8d6c0ae8459f -->
### Requirement: no-shared-install <!-- base: 4715e1bb3636cf98 -->
### Requirement: background-work-stopped <!-- base: 050f5861ad7436fd -->
### Requirement: check-tree-still <!-- base: c2b254024b054773 -->
### Requirement: scratch-files <!-- base: a7e8c4bd9da4ba32 -->
### Requirement: prompt-spot-checks <!-- base: 96c6ba17c188e6f4 -->
### Requirement: review-every-diff <!-- base: 12a07b11d7108daf -->
### Requirement: capability-tests-follow-rule <!-- base: 37f21ffa5aa1df95 -->
### Requirement: prompt-pushes-through-clm <!-- base: 8e9fbdf3e13e308d -->
### Requirement: prompt-recorded-check <!-- base: 6391bb974d549829 -->
### Requirement: bounded-test-runs <!-- base: 986bf514dbd17eda -->
### Requirement: template-migration-fix <!-- base: 0e61edbca1ef8974 -->
### Requirement: prompt-unopened-pr <!-- base: 0e07e5be31cfb258 -->
### Requirement: prompt-doc-readers <!-- base: 9d550ef326cebeae -->
### Requirement: prompt-question-choices <!-- base: dfacdb562f83c92e -->
### Requirement: stays-in-its-component <!-- base: f555fb6d25cb6f18 -->
