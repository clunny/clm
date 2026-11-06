---
title: Run
status: active
owners: ["@clunny"]
depends_on: [orchestrator, context, check, config, vcs, model]
paths: ["crates/clm-run/**"]
successor: ""
serves: [capability/build-from-spec]
---

## Contract
`clm run <component>` claims a component through `orchestrator`'s `claim_local`, which records the claim in the local branch `impl/<component>` before pushing it, or picks up its unfinished build, so a launch interrupted at any point is resumed or claimed afresh by the next `clm run <component>`, never held until its claim lapses, and writes the prompt for a session that builds its pending requirements and lands them with `clm land` (`land-locally`) to `.clm/prompts/<component>.md` in the checkout it runs in. On stdout it prints a short launcher naming that file by its absolute path, which `build` gives to a background subagent of the human's session in its own Claude Code worktree (`builds-in-subagents`), and which the subagent follows by reading the file (`build-prompt-file`). It fetches `origin` first and takes `--expiry <hours>` (default 4) and `--review-model <model>`, the alias of the model the build's final review runs on, which `build` sets to its own session's model (`model-per-launch`). Each build holds one of the clone's `max_builds` places (`orchestrator`'s `build-places`) from when `clm run` starts it until it lands, is finished or its claim lapses. When it refuses a component, it says why. It exits 0 on success, 1 when the work isn't possible (with the reason), 2 when git fails, and 3 when the clone has no free place. `clm run --finish <component>`, which `build` runs whenever a build's subagent returns, frees the component's place and tidies the worktree of a landed build, printing `landed impl/<component>` when the branch has landed (`unclosed-build-reads-finish`); a build that hasn't landed keeps its claim and pushed work, so the next `clm run <component>` resumes. A build compiles into its own worktree's `target/` (`build-places`). `clm superpowers <skill>` prints the path of `skills/<skill>/SKILL.md` in the pinned superpowers copy, fetching it first if needed, so builds and interactive skills follow one version (`design-brainstorming`, `unattended-superpowers`). The pin is moved by hand, on an `impl/orchestrator.run` branch (`cut-evals`).

A component whose pending requirements name a file Claude Code protects, such as `.claude/settings.json` or an `.npmrc`, is built like any other: Claude Code asks the human to confirm each such write (`builds-in-subagents`).

## Design
Crate `clm-run`. The queue, the claim and the pending list are `orchestrator`'s; the bundle the prompt points at is `context`'s. The check command the prompt names is `config`'s `[land] check`, read with its `load` from the checkout `clm run` runs in. This component owns what the session is told: the prompt's text, its constants (the superpowers pin, the skills it names, the requirement budget) and the resume logic.

`clm run` prepares builds as `builds-in-subagents` and `unattended-superpowers` decide; it starts no session itself.
- **Where a build runs.** `build` launches each build with the Agent tool's `isolation: worktree`, so Claude Code gives the subagent its own worktree, starting from `origin/main`, in which every Bash call and every subagent it launches starts: a build can't stray into the human's checkout, even by a relative path. `clm run` makes no worktree. It leaves a local branch `impl/<component>` at the claim, tracking `origin/impl/<component>`, which the build switches to, and keeps `.clm/` (the plan and the pull request body) out of git through `.git/info/exclude`, which every worktree shares. CLM finds a worktree by the branch it has checked out (`git worktree list --porcelain`), never by its path. Claude Code puts an `isolation: worktree` subagent's worktree under the main worktree's `.claude/worktrees/` (`agent-*`), even when the launching session runs in a linked worktree. So `clm run --finish` looks for the build's worktree under the main worktree's `.claude/worktrees/`, whichever worktree it runs in.
- **Resume and leftovers** (`merged-claims-released`, `run`). A component comes back after its branch landed whenever its spec moves, so an earlier build's local branch `impl/<component>`, or a worktree that has it checked out, may still be there; `run` says what `clm run` removes, resumes and refuses (`git worktree remove` after unlocking, since Claude Code locks its worktrees). The landed test comes first, so a landed branch is never resumed. It comes before the place because of how a place is judged: `orchestrator`'s Design says a build's place has landed when each `impl/<component>` that exists has its tip in `origin/main`, and `clm land` deletes `origin/impl/<component>` but not the local branch, so a place taken while a landed local `impl/<component>` is still there would read as landed, and count for nothing, until the new claim is pushed; another command taking a place in that window would delete it and could take the last free one, running more than `max_builds` builds. With the landed branch gone the component has no branch, which a place reads as not landed, so the new place counts by its `taken` time, and nothing new is recorded. A landed leftover holds nothing `origin/main` lacks, so removing it is harmless even when the run then finds no free place. A dirty worktree refuses the run before any place is taken, since that check writes nothing. A landed `origin/impl/<component>` that still exists, which happens only when `clm land`'s delete of it failed or the branch reached `main` some other way, still makes the new place read as landed until the claim is pushed: that window is accepted, since closing it would take a push before the place. **An interrupted launch** (`claim-local-before-push`). `clm run` claims through `orchestrator`'s `claim_local`, which makes the local branch at the claim commit before pushing it, so no interruption leaves a pushed claim with no local branch. Interrupted after the push, the local branch is the claim's tip, so the next run resumes it by the containment rule below, whether the claim is still live or has lapsed. Interrupted before the push, or after a push whose outcome `claim_local` couldn't read, the local branch's tip is an *unpushed claim*: a commit with one parent, the same tree as that parent, the parent an ancestor of `origin/main`, and not contained in `origin/impl/<component>` (or no such branch). It holds no work, so the leftovers step removes it, like a landed branch, when no worktree has it checked out, and the run then claims as usual; when another clone has claimed the component meanwhile, that claim refuses the run as for any live claim. The test is by structure, not by the commit's message: a commit that adds nothing to a tree on `main` holds nothing to lose, whoever made it. Another clone's claim is never resumed, since this clone has no local branch at it. A clone left with a pushed claim and no local branch by a `clm` from before `claim_local` waits for the claim to lapse, as before: that state no longer arises. A local `impl/<component>` that isn't landed and doesn't contain `origin/impl/<component>`, or has no `origin/impl/<component>` at all, may hold commits nobody pushed (other than an unpushed claim), so `clm run` leaves it. `clm run` trusts that no build of the component is running: `build` calls it only for components `clm next` lists, which have no live claim, and for its own builds once they have returned. A worktree with uncommitted changes or untracked files that aren't ignored may hold work nobody committed.
- **Superpowers.** Builds follow superpowers at a pinned release. The pin is `crates/clm-run/superpowers.pin`, one line `<tag> <commit>`, which `clm-run` reads at compile time with `include_str!` into its `SUPERPOWERS_TAG` and `SUPERPOWERS_COMMIT`; the copy is fetched once into `${XDG_CACHE_HOME:-~/.cache}/clm/superpowers/<commit>`. Moving the pin is a one-line edit of that file, made by hand on an `impl/orchestrator.run` branch after reading the release's notes and landed through `clm land` like any code (`cut-evals`); nothing runs an eval or watches for releases. The skills the prompt names are a constant: `superpowers:writing-plans`, `superpowers:test-driven-development` and `superpowers:verification-before-completion`. Changing the skills is a code change. `$CLM_SUPERPOWERS_DIR` overrides the pin, for tests and for trying a release before moving the pin. `clm superpowers <skill>` resolves the same directory, the override included, and prints `<dir>/skills/<skill>/SKILL.md`. A build runs in the human's session and has its plugins, not the pin, so the prompt gives each skill it names with the path `clm superpowers <skill>` prints, and says to read and follow that file and never to invoke an installed copy.
- **Prompt file** (`build-prompt-file`). The prompt holds the component's whole `clm context` bundle, tens of KB, so `clm run` writes it to `<root>/.clm/prompts/<component>.md`, where `<root>` is the checkout `clm run` runs in; `.clm/` is already outside git. On stdout it prints only the launcher, a few lines: read `<absolute path>` with the Read tool, all of it (in parts when it is too large to read at once), before anything else, and follow it as your instructions; it is the prompt for building `<component>`, and its first step is `git switch impl/<component>`. The orchestrating session passes the launcher unchanged as the Agent tool's prompt, so it never reads or repeats the bundle, and the subagent, whose worktree is elsewhere, reads the file by that absolute path. The file stays in the orchestrator's checkout, which no build removes, until the next `clm run` of the component replaces it; a refused run leaves an earlier file alone, and with no launcher printed nothing reads it. The prompt depends on the claimed tree, so it is written after the claim. **The tree it is read from** (`pending-prompt`) is the merge of `origin/main` and `impl/<component>`, as `git merge-tree --write-tree origin/main impl/<component>` writes it after the fetch, the way `land` and `gate` compute a merged tree (`reuse-first`); it writes only objects, so the session's checkout, which `build` never brings up to date (`views-name-their-commit`), is never read. For a fresh claim the merge is the claim's own tree, since the claim's parent is `origin/main`. For a resumed branch cut from an older `main`, it holds what has landed since, so a requirement another component's build has made current since then no longer blocks, and the lists agree with `clm next`, which reads `origin/main`. The branch never edits `specs/`, so the merge's specs are `origin/main`'s, and the earlier texts of a `behind` requirement are looked up in `origin/main`'s history of the component's `SPEC.md`. When the merge conflicts, the lists come from `origin/main`'s tree, which lacks the branch's own markers, so the requirements the branch built show as pending: the prompt says the lists were read from `origin/main` because the branch conflicts with it, and that `clm pending <component>` after the first `clm rebase` is the list to build from; `.clm/prompts/` is created before the claim, beside the exclude, so a missing or unwritable directory refuses the run before anything is claimed, and a write that still fails after the claim leaves the claim to the next `clm run`, which resumes it. The launcher is a fixed template.
- **Places** (`build-places`, `run-takes-place`). `clm run` takes a place named `<component>` with `orchestrator`'s `take_place`, its branch `impl/<component>`, with `max_builds` from `config`'s `load` of the checkout it runs in and its `--expiry`. A place `<component>` that counts is reused, so a build that resumes its component's branch, or a retry after a build stopped, holds the place it had; otherwise a new place needs one to be free. The exit-3 message on stderr is `no free build place: <held> of max_builds = <n> in use` (only `max_builds`, its value and the number held are binding; a hint may follow), and an exit 3 leaves nothing behind but the landed leftovers it removed: no claim, no new branch, no exclude line, no prompt. A build of another component that is already running holds its own place, so `build` can't start more builds than the cap however many components `clm next` printed before.
- **Finishing a build** (`build-places`, `run-finish`). `clm run --finish <component>` frees the place with `free_place`, printing `freed <component>` when there was one. A branch has landed by the place's test (`orchestrator`'s Design: every one of `origin/impl/<component>` and the local `impl/<component>` that exists has its tip in `origin/main`, and one of them exists). A worktree is unlocked first, since Claude Code locks the worktrees it makes, ignoring the error for one that isn't locked, then removed with `git worktree remove`. A worktree elsewhere, such as the human's own checkout, is never touched. When the branch hasn't landed, the claim, the local branch and the worktree stay, so a retry resumes them. It doesn't delete the local branch: `clm wrap-up`, or the next `clm run <component>`'s leftovers step, does. It reads no queue state, so it works for any component, converged or removed. The orchestrating session runs it rather than `git worktree remove` because the build's worktree is another worktree, where a worktree-isolated session may not run git, and the build's own session can't remove the worktree it runs in; `clm land` has already deleted that worktree's `target/` (`land`'s `land-frees-build-target`). **What a build left unwritable** (`run-finish`). A build can leave an ignored directory without its owner's write permission, for instance by unpacking an archive; `git status` still reads the worktree as clean, but `git worktree remove` unregisters the worktree and then can't delete what that directory holds. So, once a worktree is judged clean, and before `git worktree remove`, `--finish` gives the owner read, write and execute permission on the worktree's directory and every directory under it that lacks them, setting a directory's permission before it reads its entries so one its owner can't list is still walked, and clears the read-only attribute of a file where the platform needs that to delete it (Windows), walking with `std::fs::symlink_metadata` and never following or changing a symbolic link, so nothing outside the worktree changes. The worktree is about to be deleted, so the permissions it changes are never seen again. What git still can't remove (a file owned by another user, a file in use on Windows, a worktree git refuses) is reported, not failed on: the line `left <path>: <error>; delete the directory by hand, then run git worktree prune` replaces the `removed` line and names the path, git's error joined onto one line (or `still on disk` when git succeeded but the directory remains), and the steps that clear it whether or not git has already unregistered the worktree, and `--finish` goes on to the other worktrees and exits 0, so the `freed` and `landed` lines it gathered are printed. Only a worktree's removal is reported this way; any other git failure still exits 2.
- **Own target** (`build-places`). The prompt doesn't name `CARGO_TARGET_DIR`, so a build compiles into its own worktree's `target/`, its check command included, and the plain check command (`prompt-check-command`) needs no variable. A target directory shared between checkouts is wrong, not only slow: cargo names a workspace crate's outputs by its path relative to the workspace, so every worktree of a repository writes the same files there, and it judges them fresh by file times, so a build whose untouched sources are older than another build's last compile would test that build's code. When `CARGO_TARGET_DIR` is set in the human's session, a build inherits it, as any command does; CLM sets nothing.
- **Prompt.**
  - **Unattended preamble.** The session works alone: no questions in the conversation, a `question` or `human-task` backlog item for anything only the human can answer or do (`ask-by-backlog`), and a note that Claude Code may ask the human to confirm a tool call, which the build waits for.
  - **The job** (`pending-prompt`, `removed-requirements-build-out`). The pending requirements with their versions, what blocks each blocked one and the requirement budget per run, then the step plan, which is executed in the same session.
  - **Removals another component still uses** (`contract-removal-needs`, `removal-breaks-dependant`). The forced compile-only and test-fixture fixes are how an optional component's tests follow a required component's removal, since no required component may `Need` an optional one (`optional-nodes`). The kept code of an `obsolete` requirement gets no `clm:remove-by` marker, since the removing change may archive first: the item records it. The root's dispatch files (`crates/clm/src/main.rs`, `Cargo.toml`, `Cargo.lock`) are not another component's code for this rule: the root's Design gives every build that adds or changes a command those edits.
  - **Rebase points** (`versioned-markers`). After each step of its step plan, the session commits, runs `clm rebase`, resolves what it reports until it prints `rebase ok`, and pushes with `clm branch push` (`host/branch-push`, `prompt-pushes-through-clm`), which also keeps its claim alive. Before landing it does the same. A push the remote rejects for a transient reason is usually caused by other builds pushing at the same moment, so the session pushes again at most twice more (three pushes in all, as `orchestrator/claim` does), before it stops and reports git's message (`prompt-commands`). This covers every push the session makes, and the prompt says it once, where it first names `clm branch push`. A hook that declines the push and a lease rejection are never retried that way: the first is a decision, and the second means the branch moved, which `clm rebase` handles. When `clm rebase` reports `spec changed <component>` (`sync/spec-changed`), the session reads the lines that follow, runs `clm pending <component>`, and adapts its work to the new text or discards and rebuilds the part it affects, whichever fits; it records what changed and which way it went under `## Sync` in the pull request body. Nothing is re-planned: the pending list is derived from the markers each time. A change `clm rebase` archived on the branch (`sync/archive-complete`) is committed with the rest.
  - **Checks** (`check-before-land`, `prompt-check-command`). The prompt tells the session to pass the repository's checks in two places: after fixing the review's findings, and before landing. Both name `clm land --check-only` (`land/check-only`); the first also gives the one-plain-command rule, which is about that one command, so the prompt still says nothing general about `cd` or one command per call (`prompt-commands`). The prompt never tells the session to run the configured command by its path: Claude Code's worktree isolation, which refuses a command it can't show stays inside the worktree, refused `.github/scripts/ci-check.sh` in a build, and `clm land --check-only` names no path for it to judge (`land-check-command`). `clm land --check-only` is a `clm` command, which never gets `CARGO_TARGET_DIR` (`shared-build-target`), and it sets none either, so the check compiles into the worktree's own `target/` as every other command of the build does. A failing check is fixed and run again. When Claude Code refuses or denies the command, the session lists the refusal under `## CLM defects` in the pull request body and lands with `clm land`, which runs the whole command on the merged tree and refuses the land when it fails (`defect-reports`). The session may run parts of the check while it works, to fail faster, but never reports them as the check. When the repository sets no `[land] check`, the prompt says "the repo's checks" and names no command. `clm run` reads the setting before it claims, so a `clm.toml` that `load` rejects makes it exit 1 with `config`'s message, claiming nothing and printing nothing on stdout.
  - **Workaround** (`workaround-flags`, `prompt-workaround`). The prompt is read by whatever `clm` the session has, which may lack the command (`dogfood-from-main`'s older build, a pinned release), so it says what to do when the binary on `PATH` lacks `clm workaround`. A flag is evidence (`workaround-flags`), so a missing flag command is never a reason to stop; the shared-binary rule's stop (below) is for commands the work itself needs. That is why `prompt-workaround` has no `#### Needs` on `check.health/workaround-command`: the prompt is right against any binary, and the need would have held a built requirement behind a command whose absence changes nothing about the build. A flag whose requirement moves under the build (`clm rebase` reports `spec changed` and `clm check` rejects the flag at its old version) is re-recorded at the new version, or deleted when the new text removes the workaround: the build added the file, and editing its own addition keeps the branch a code branch. The reconsider questions the flags may lead to are filed by `clm rebase`, never by the session.
  - **Shared binary** (`no-shared-install`). A build that replaced the `clm` binary on its `PATH` would change the commands the human's session, which orchestrates the builds, and the other builds rely on mid-session. The installed `clm` changes only outside builds, such as the main checkout's rebuild after a merge (`dogfood-from-main`).
  - **Pull request and landing** (`own-pr-body`, `land-locally`, `prompt-pr-body`). The pull request is opened before landing so the human has a diff to read wherever the host shows one. `Write` replaces `.clm/pr-body.md`, so a body left by an earlier run never carries over. The body has `## Plan`, `## Review`, `## Sync`, `## Workarounds`, `## Fixes in other components`, `## Left pending` and `## CLM defects`. The session goes by `clm land`'s last line: `landed <sha> ...` ends the build; `gate/up-to-date` or a conflict means `clm rebase`, resolve, push and land again; a check failure is fixed and landed again; anything else is reported.
  - **Resume.** When `clm run` resumes a branch that has commits `origin/main` lacks, the prompt says so: the session starts with `clm rebase`, handles what it reports as above, and lands the same branch. The branch's pull request, if one was opened, is found by `clm pr open`, which prints the open pull request's URL instead of opening another.
  - Every land goes to `main`.
- **Review** (`advisory-code-review`). Before landing, a session reviews its diff with two fresh-context reviewers: `/code-review` at a fixed level for bugs, and the `clm` plugin's `spec-auditor` subagent for the spec, given the component. It fixes the findings it agrees with, re-runs the checks, and lists every finding with what it did about it under `## Review` in the pull request body. The level and the reviewer names are constants in `clm-run`, like the pin. The review advises and never gates, but it always runs (`review-every-diff`): a session judging its own diff too small to review (only markers, two tests, a few lines) is the judgment the fresh-context reviewers exist to replace, and a diff of markers alone is where `spec-auditor` finds a test that cites a requirement it doesn't prove. No program checks that it ran: the record under `## Review` is the visibility `advisory-code-review` chose, and the prompt says plainly that no diff is exempt. The reviewers start in the build's worktree, as every subagent a build launches does, so they read its diff.
- **Permissions** (`builds-in-subagents`). A build runs under the permission mode of the human's session, which Claude Code doesn't let a subagent override, so `clm run` sets none and ships no allowlist. In manual mode a build's permission prompts reach the human's session. Claude Code asks the human before writing a protected path (`.git/`, `.claude/`, `.npmrc` and the rest of its list) in every mode, and that build waits for the answer while the others carry on. The guard runs on a build's calls through the `clm` plugin's `PreToolUse` hook, which Claude Code runs for subagents (`guard-blocks-push-to-main`). In auto mode Claude Code's classifier judges each command no rule allows, and it denied a raw force-push of a rebased branch as destructive, stopping builds with a checked branch they couldn't land. So every step that writes to `origin` is a `clm` command (`unattended-steps-run-clm`): the build pushes with `clm branch push` and lands with `clm land`, and the project's allow rules, which `clm init` seeds (`init/init-seeds-permissions`), let those run without the classifier. The prompt names no `git push`, so no rule for a raw push is ever needed, and the prompt itself needs no rule and states none: the `build` skill checks the project's rules before the first build (`plugin/unattended-permissions-check`).
- **Parallelism.** Each build has its own worktree, so `build` builds several components at once, at most `max_builds` (above). Component `paths` are disjoint, so two builds edit the same files only through a forced fix in another component (above), which `clm land`'s merge of `main` then reconciles like any other conflict.

## Requirements

### Requirement: run
`clm run <component>` SHALL first remove leftovers of an earlier build of the component that are already landed: a worktree that has `impl/<component>` checked out, has no uncommitted changes or untracked files that aren't ignored, and whose `HEAD` is an ancestor of `origin/main`, unlocking it first; and then a local branch `impl/<component>` that no worktree has checked out and whose tip is an ancestor of `origin/main` or is an unpushed claim: a commit with one parent, the same tree as that parent, the parent an ancestor of `origin/main`, and not contained in `origin/impl/<component>`, or with no `origin/impl/<component>`. When the local branch `impl/<component>` then exists and isn't landed, it SHALL exit 1 naming the branch, without claiming, unless it contains `origin/impl/<component>`, in which case it SHALL resume without claiming, first removing a worktree that has the branch checked out when that worktree has no uncommitted changes or untracked files that aren't ignored. Otherwise it SHALL claim the component through `orchestrator`'s `claim_local`, so that the local branch `impl/<component>` is at the claim commit, tracking `origin/impl/<component>`, before the claim is pushed, exiting 1 when the claim is refused and 2 when it fails in git. When a worktree that has `impl/<component>` checked out has uncommitted changes or untracked files that aren't ignored, it SHALL exit 1 naming its path, without claiming. It SHALL keep `.clm/` out of git through `.git/info/exclude`, create no worktree, and start no `claude` process.

It SHALL then write the prompt to `.clm/prompts/<component>.md` under the directory it runs in, replacing any earlier file, print on stdout only a launcher of at most five lines that names that file by its absolute path, says to read all of it before anything else and follow it, and names `git switch impl/<component>` as the file's first step, and exit 0. It SHALL create `.clm/prompts/` before claiming, refusing with exit 1 when it can't; when writing the file fails after the claim, it SHALL exit 1 naming the path. The prompt's first line SHALL say to run `git switch impl/<component>` and check the branch before anything else. It SHALL then have the unattended preamble; SHALL say that Claude Code may ask the human to confirm a tool call; SHALL tell the session to end its final message with one line, `landed <sha>` or `stopped: <reason>`; SHALL name the skills `superpowers:writing-plans`, `superpowers:test-driven-development` and `superpowers:verification-before-completion`, each with the path `clm superpowers <skill>` prints and saying to read and follow that file; SHALL state the job (`pending-prompt`): the step plan written to `.clm/plans/<component>.md` and executed in the same session without `executing-plans`, `subagent-driven-development`, `using-git-worktrees` or `finishing-a-development-branch`, and summarised under `## Plan` in the pull request body; the review, `/code-review` and the `spec-auditor` subagent, with findings under `## Review`; the rebase points, `clm rebase` after each step of the plan and before landing, with `clm pending <component>` and adapting or rebuilding on `spec changed`, recorded under `## Sync`; and the landing, `clm land` and what to do with each of its outcomes; SHALL say when resuming a branch with unlanded commits that the branch exists and is to be landed; and SHALL end with the `clm context` bundle. When it exits 1 or 2, it SHALL print nothing on stdout.

#### Scenario: node prepared
- GIVEN a ready component `b`, `CLM_SUPERPOWERS_DIR` set to a directory holding the three skills, and a fake `claude` on PATH that records whether it ran
- WHEN `clm run b` runs
- THEN it exits 0, `origin/impl/b` exists, the local branch `impl/b` is at the claim and tracks it, no worktree has `impl/b` checked out, the fake never ran, stdout is at most five lines, names the absolute path of `.clm/prompts/b.md` and `git switch impl/b`, and doesn't contain `b`'s context bundle, and that file's first line names `git switch impl/b`, and the file has the preamble, `landed <sha>` and `stopped:`, the three skill names each with its `SKILL.md` path under `CLM_SUPERPOWERS_DIR`, the plan path `.clm/plans/b.md`, `executing-plans` among the skills not to invoke, the review step naming `/code-review` and `spec-auditor`, the rebase-point step naming `clm rebase`, `spec changed`, `clm pending` and `## Sync`, the landing step naming `clm land`, and ends with `b`'s context bundle

#### Scenario: claimed elsewhere
- GIVEN `origin/impl/b` committed 1 hour ago, and no local branch `impl/b`
- WHEN `clm run b` runs
- THEN it exits 1 and prints nothing on stdout

#### Scenario: resume with unlanded commits
- GIVEN a clean worktree left by an earlier build with `impl/b` checked out, its branch pushed with a commit `origin/main` lacks
- WHEN `clm run b` runs
- THEN no claim commit is pushed, that worktree is gone, the local branch `impl/b` still has the commit, and the prompt it writes says the branch exists and to start with `clm rebase` and land the same branch

#### Scenario: landed leftover worktree
- GIVEN a clean worktree with local branch `impl/b` checked out, left by an earlier build whose branch landed on `origin/main` with a merge commit, with `origin/impl/b` still at that tip, the worktree locked, and a ready component `b`
- WHEN `clm run b` runs
- THEN the worktree is gone, it pushes a new claim commit whose parent is `origin/main` to `origin/impl/b`, the local branch `impl/b` is at that commit, and it exits 0

#### Scenario: landed leftover branch
- GIVEN a local branch `impl/b` with no worktree, landed on `origin/main`, no `origin/impl/b`, and a ready component `b`
- WHEN `clm run b` runs
- THEN it claims `b`, the local branch `impl/b` is at the claim, and it exits 0

#### Scenario: unpushed leftover branch
- GIVEN a local branch `impl/b` with a commit that changes a file and that neither `origin/main` nor `origin/impl/b` has, where `origin/impl/b` is a lapsed claim, and a ready component `b`
- WHEN `clm run b` runs
- THEN it exits 1 naming `impl/b`, pushes no claim commit, prints nothing on stdout, and `impl/b` still has the commit

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
- land/land
- orchestrator/claim-local

### Requirement: pending-prompt
The `clm run` prompt SHALL list the component's pending requirements, most urgent first, each with its ID, version and state; for a `behind` requirement, the diff from the text at each version its markers cite other than the current one to the current text, taken from the history of the component's `SPEC.md`, or a line saying the earlier text wasn't found, and the locations of the markers that cite it at another version or without one, saying to re-cite each one wherever its file sits, another component's `paths` included, after checking that its test proves the current text (`path-ownership`, `stale-markers-keep-behind`); and for an `obsolete` one, the locations of its markers, saying to remove the code and tests they mark. It SHALL leave blocked requirements out of the list and name them with what blocks each: the backlog items, or the pending requirement they need. It SHALL tell the session to cite each requirement it builds as `req:<id>@<version>` with the current version, to build the most urgent first, that it may open its pull request and land once the tree is green with at most the budget `clm-run` sets built, listing what it left under `## Left pending`, and that a `spec changed` report from `clm rebase` is handled by reading the change and `clm pending <component>`, never by re-planning. It SHALL take the lists, the earlier texts and the context bundle from the tree `git merge-tree --write-tree origin/main impl/<component>` writes once `clm run` has fetched, never from the checkout it runs in, so that a requirement made current on `origin/main` since a resumed branch was cut neither shows as pending nor blocks another; when that merge conflicts, it SHALL take them from `origin/main`'s tree and say in the prompt that they were read from `origin/main` because the branch conflicts with it, and that `clm pending <component>` after the first `clm rebase` is the list to build from.

#### Scenario: behind and obsolete
- GIVEN component `b` whose requirement `b/x` is cited at version `11111111`, which `specs/b/SPEC.md` had at an earlier commit, and is now at `22222222`; whose requirement `b/y` is uncited; and a marker at `t.rs:9` citing `b/gone`, which a live change removed
- WHEN `clm run b` runs
- THEN the prompt it writes lists `b/x` as `behind` with a diff from the earlier text to the current, `b/y` as `unbuilt`, and `b/gone` as `obsolete` at `t.rs:9`, and contains `req:<id>@<version>`, `## Left pending` and `clm pending b`

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

#### Scenario: budget in land terms
- GIVEN a ready component `b`
- WHEN `clm run b` runs
- THEN the prompt it writes says the session may land once the tree is green with at most the budget built, and doesn't contain `open its PR` or `open your PR`

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

### Requirement: prompt-commands
`clm run`'s prompt SHALL NOT tell the session to run one command per Bash call or never to start one with `cd`. Wherever it says to run `clm rebase` until it passes, it SHALL say to go on its last line, `rebase ok`, and SHALL NOT tell the session to read its exit code. It SHALL tell the session that when a write is refused, it writes nothing another way and handles the write as a step only a human can take. It SHALL tell the session that when the remote rejects a push as `remote rejected` with `(failure)`, or with `remote: fatal error`, it waits a few seconds and pushes again, at most twice more, before it stops, and that it never retries a hook's decline or a lease rejection (`stale info`) that way. It SHALL name every command by the name `clm/renamed-commands` gives it, never by an old name that requirement keeps as an alias, and call a branch's pull request a pull request.

#### Scenario: prompt
- GIVEN a ready component `b`
- WHEN `clm run b` runs
- THEN the prompt it writes contains `rebase ok`, `rebase failed`, `remote rejected` and `stale info`, and doesn't contain `one command per Bash call`, `exits 0` or `exits 2`

#### Scenario: no old command names
- GIVEN a ready component `b` and a resumed branch `impl/c` with commits `origin/main` lacks
- WHEN `clm run b` and `clm run c` run
- THEN neither prompt names `clm` followed by one of the old subcommands `sync`, `checkpoint` or `page`, nor the word `page`

#### Needs
- clm/renamed-commands

### Requirement: prompt-pr-body
`clm run`'s prompt SHALL tell the session to write the pull request body with `Write` to `.clm/pr-body.md`, to open the pull request with `clm pr open impl/<component> --title <title> --body-file .clm/pr-body.md`, to correct the body by rewriting that file and running `clm pr body .clm/pr-body.md`, and to land with `clm land`. It SHALL NOT name `gh pr create`, `--body `, `gh pr edit`, `clm watch` or `gh run rerun`.

#### Scenario: prompt
- GIVEN a ready component `b`
- WHEN `clm run b` runs
- THEN the prompt it writes contains `clm pr open impl/b`, `--body-file .clm/pr-body.md`, `clm pr body .clm/pr-body.md` and `clm land`, and doesn't contain `gh pr create`, `--body `, `gh pr edit`, `clm watch` or `gh run rerun`

#### Serves
- capability/build-from-spec
- capability/land-safely

#### Needs
- clm/renamed-commands
- land/land
- host/pr-open
- host/pr-body

### Requirement: ask-by-backlog
The `clm run` prompt SHALL tell the session that when the work needs a fact only a human knows, or a step only a human can take, it adds a `question` or `human-task` backlog item instead of guessing, with `blocks` naming the requirements that can't be built without it, and builds the rest (`requirement-blocks`).

#### Scenario: prompt
- GIVEN the prompt `clm run` builds for a component
- WHEN the tests read it
- THEN it names the `question` and `human-task` kinds and `blocks`, and says to build the rest

#### Serves
- capability/keep-a-backlog
- capability/build-from-spec

#### Needs
- model/parse-backlog

### Requirement: superpowers-path
`clm superpowers <skill>` SHALL print the absolute path of `skills/<skill>/SKILL.md` in the pinned superpowers directory builds follow: `$CLM_SUPERPOWERS_DIR` when it is set and not empty, otherwise the pinned commit's cache directory, fetched first when absent. It SHALL exit 1, naming the skill, when that file doesn't exist, and 2 with the reason when fetching the pin fails. No word SHALL be reserved: `upgrade` and `eval` are resolved as skills like any other, and `clm-run` SHALL NOT read, run or name anything under `plugin/evals/` (`cut-evals`).

#### Scenario: pinned copy
- GIVEN `XDG_CACHE_HOME` holding `clm/superpowers/<pinned commit>/skills/brainstorming/SKILL.md`
- WHEN `clm superpowers brainstorming` runs
- THEN it prints that file's path and exits 0, without fetching

#### Scenario: override
- GIVEN `CLM_SUPERPOWERS_DIR` set to a directory holding `skills/brainstorming/SKILL.md`
- WHEN `clm superpowers brainstorming` runs
- THEN it prints the path under that directory

#### Scenario: unknown skill
- GIVEN the same override
- WHEN `clm superpowers no-such-skill` runs
- THEN it exits 1 naming `no-such-skill`, and prints no path

#### Scenario: no reserved words
- GIVEN the same override, holding no `skills/upgrade/` or `skills/eval/`
- WHEN `clm superpowers upgrade` and `clm superpowers eval out` run
- THEN the first exits 1 naming `upgrade` as it names any skill it can't find, and the second exits non-zero with the usage error any extra argument gives, and neither runs `claude` or `git ls-remote`

#### Scenario: no eval in the crate
- GIVEN the source of `crates/clm-run`
- WHEN the tests read it
- THEN no file under its `src/` names `plugin/evals`

#### Serves
- capability/pin-working-method

### Requirement: delete-fixed-items
The `clm run` prompt SHALL tell the session that when its work fixes an item in a live change's `resolves`, it deletes `backlog/<slug>.md` on its own branch, so the requirements the item blocked are released when that branch lands. It SHALL also tell the session that when its branch does the work of a `deferred` item whose `components` name its component, such as removing code a build kept for another component that no longer uses it, it deletes `backlog/<slug>.md` on its own branch in a commit whose trailer block holds `Resolves: backlog/<slug>` (`deferred-work-resolves-in-code`), and that a `question`, `human-task` or `idea` item it can't delete that way goes under `## Backlog` in its pull request body, as any other backlog edit it can't make.

#### Scenario: prompt
- GIVEN the prompt `clm run` builds for a component
- WHEN the tests read it
- THEN it says to delete a fixed item of a change's `resolves` on its own branch, and doesn't contain `own PR`

#### Scenario: deferred work done
- GIVEN the prompt `clm run` builds for a component
- WHEN the tests read it
- THEN it names the `deferred` kind and the trailer `Resolves: backlog/<slug>`, and says to delete such an item on the session's own branch

#### Serves
- capability/keep-a-backlog

#### Needs
- model/parse-backlog

### Requirement: removal-breaks-dependant
The `clm run` prompt SHALL tell the session never to edit another component's code or tests to absorb what its own component removes from or adds to its Contract, except for the compile-only and test-fixture fixes the removal or the addition forces (`architecture/path-ownership`, `forced-fixes-cover-additions`): where the other component's code or tests only name what is removed (a `match` arm on a removed variant, a fixture that sets a removed field, an assertion on a removed field or state) and nothing that component's code does depends on it, or only build a value that gains a field (a struct literal, in code or in a fixture, that must now name the added field) and the fix sets that field to the value under which the provider behaves as before, so the other component's behaviour doesn't change, the session SHALL make the fix and name each one under `## Fixes in other components` in its pull request body. It SHALL say that a rename is not such a fix (`rename-adds-then-removes`), and neither is an arm for an added variant that another component matches exhaustively, nor an added field with no value under which the provider behaves as before, since those decide what the other component does. When building a requirement would break another component's code or tests because that component still uses what the requirement removes, or because what it adds needs more there than such a fix (a rename, an arm for an added variant or a field with no such value), the session SHALL keep the old path working and that component building as it stands, leave the requirement unbuilt, and add a `question` backlog item whose `components` name both components, with `blocks` naming the requirement and saying which component uses what; for an `obsolete` requirement it SHALL remove the markers and tests, keep the code the other component uses, with no `clm:remove-by` marker, and add a `deferred` item without `blocks`, whose `components` name both components and whose body names the kept code, the component that uses it, and that the code goes once that component no longer uses it (`contract-removal-needs`, `deferred-work-resolves-in-code`). The text SHALL be in every prompt, beside `pending-prompt`'s instruction to remove an obsolete requirement's code, which it qualifies and leaves in place.

#### Scenario: prompt
- GIVEN the prompt `clm run` builds for a component
- WHEN the tests read it
- THEN it says not to edit another component to absorb a removal or an addition except for the compile-only and test-fixture fixes it forces, named under `## Fixes in other components`, to keep the old path, and to add a `question` item with `blocks` naming the requirement

#### Scenario: addition forces a fix
- GIVEN the prompt `clm run` builds for a component
- WHEN the tests read it
- THEN it says a struct literal in another component that must name an added field is fixed by setting the field to the value under which the provider behaves as before, and that a rename, an arm for an added variant or a field with no such value is not such a fix

#### Scenario: obsolete requirement
- GIVEN the prompt `clm run` builds for a component with no `obsolete` requirement
- WHEN the tests read it
- THEN it says to keep code another component still uses and to add a `deferred` item without `blocks`

#### Serves
- architecture/path-ownership
- capability/no-dead-weight

### Requirement: prompt-check-command
When `clm.toml` sets `[land] check`, `clm run`'s prompt SHALL tell the session, wherever it tells it to pass or re-run the repository's checks, to run `clm land --check-only`, saying that it runs that command, named as `clm.toml` spells it, and then `clm check`, as `clm land` does; and SHALL tell the session to run `clm land --check-only` from the worktree's top level as one plain command, exactly as written, adding no `cd`, absolute path, variable, pipe or redirection. Beside that rule it SHALL give the reason for the pipe and the redirection: a pipe, such as one to `tail`, reports the last command's result instead of the check's and cuts the check's output, so a failing check can read as passing; the command's last line, `checks pass` or the name of the command that failed, is its verdict, and Claude Code keeps a long command's output whole, so there is nothing to gain by cutting it. It SHALL NOT tell the session to run the configured command by its path, nor make it an exception to the rule of setting no `CARGO_TARGET_DIR` on a `clm` command. It SHALL tell the session that when Claude Code refuses or denies that command, it doesn't count a part of the check as the check: it reports the refusal with `clm defect`, lists it under `## CLM defects`, and lands with `clm land`, which runs the whole command. When `clm.toml` sets no `[land] check`, the prompt SHALL NOT name a check command or `clm land --check-only`, or say to run one as one plain command. The prompt SHALL NOT name `CARGO_TARGET_DIR` or a target directory, whatever the environment holds, so the check command and every cargo command compile into the worktree's own `target/` (`build-places`). When `clm.toml` can't be loaded, `clm run` SHALL exit 1 with the reason before claiming, printing nothing on stdout.

#### Scenario: check command named
- GIVEN a ready component `b` and a `clm.toml` whose `[land] check` is `scripts/check.sh`
- WHEN `clm run b` runs
- THEN the prompt it prints names `clm land --check-only` as the command to run and `scripts/check.sh` as what it runs, says to run it from the worktree's top level as one plain command, contains neither ``run `scripts/check.sh` `` nor ``with `scripts/check.sh` ``, names `clm land --check-only` in both places it says to pass the checks, and says that when it is refused the session reports it with `clm defect`, lists it under `## CLM defects` and lands with `clm land`

#### Scenario: why no pipe
- GIVEN a ready component `b` and a `clm.toml` whose `[land] check` is `scripts/check.sh`
- WHEN `clm run b` runs
- THEN the paragraph of the prompt that contains `exactly as written` also contains `tail`, `can read as passing`, `last line` and `checks pass`

#### Scenario: no check command
- GIVEN a ready component `b` and a `clm.toml` with no `[land]` table
- WHEN `clm run b` runs
- THEN it exits 0 and the prompt it prints contains neither `one plain command`, `--check-only` nor `can read as passing`

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

### Requirement: no-shared-install
The `clm run` prompt SHALL tell the session never to install or replace the `clm` binary on its `PATH`, naming `cargo install` as one way it must not, and saying that the orchestrating session and the other builds run that binary. It SHALL tell the session that when that binary is missing or lacks a command the work needs, it stops and says why rather than installing one; a missing `clm workaround` is not such a case, which `prompt-workaround` covers. The rule SHALL NOT forbid running the `clm` the branch builds without installing it (such as with `cargo run`), nor installing other tools.

#### Scenario: prompt
- GIVEN a ready component `b`
- WHEN `clm run b` runs
- THEN the prompt it prints says never to install or replace the `clm` binary, names `cargo install`, says to stop when the `clm` binary is missing or lacks a command the work needs, and says to carry on when it only lacks `clm workaround`

#### Serves
- capability/work-concurrently

### Requirement: run-takes-place
`clm run <component>` SHALL take a place named `<component>` with `take_place`, using `max_builds` from the settings it loads and its `--expiry`, after fetching, loading the settings and removing the landed leftovers that `run` names (a clean worktree whose `HEAD`, and a local `impl/<component>` whose tip, is an ancestor of `origin/main`), and before it writes or removes anything else: `.git/info/exclude`, `.clm/prompts/`, other worktrees or branches, or the claim. When a worktree that has `impl/<component>` checked out has uncommitted changes or untracked files that aren't ignored, landed or not, it SHALL exit 1 naming its path without taking a place. When the clone's place `<component>` counts, it SHALL reuse it. When no place is free it SHALL exit 3, print nothing on stdout, print on stderr a message naming `max_builds`, its value and the number of places held, and claim, resume and write nothing, and remove nothing but the landed leftovers. When it exits 1 or 2 after taking a new place, it SHALL free that place; a place it reused SHALL stay.

#### Scenario: cap reached
- GIVEN a ready component `b`, `CLM_MAX_BUILDS=1`, a fresh clone with no `.git/info/exclude` line for `.clm/`, and a place `a` for a live claim `origin/impl/a`
- WHEN `clm run b` runs
- THEN it exits 3, stdout is empty, stderr names `max_builds`, `1` and `1` held, `origin` has no `impl/b`, the clone has no place `b`, `.git/info/exclude` doesn't name `.clm/`, and `.clm/prompts/` doesn't exist

#### Scenario: landed leftover gone before the place
- GIVEN a ready component `b`, `CLM_MAX_BUILDS=1`, a place `a` for a live claim `origin/impl/a`, and a local branch `impl/b` with no worktree, whose tip is an ancestor of `origin/main`, and no `origin/impl/b`
- WHEN `clm run b` runs
- THEN it exits 3, the local branch `impl/b` is gone, and the clone has no place `b`

#### Scenario: place taken
- GIVEN a ready component `b` and no places
- WHEN `clm run b` runs
- THEN it exits 0 and the clone's place `b`, under the git common directory's `clm/builds/`, holds `kind build` and `branch impl/b`

#### Scenario: resume keeps its place
- GIVEN `CLM_MAX_BUILDS=1`, a place `b` that counts, and an earlier build's local branch `impl/b` containing `origin/impl/b` with a commit `origin/main` lacks
- WHEN `clm run b` runs
- THEN it exits 0, resumes without claiming, and `b` is still the clone's only place

#### Scenario: refused claim frees the place
- GIVEN `origin/impl/b` committed 1 hour ago by another clone, no local branch `impl/b`, and no places
- WHEN `clm run b` runs
- THEN it exits 1 and the clone has no place `b`

#### Serves
- capability/work-concurrently

### Requirement: run-finish
`clm run --finish <component>` SHALL fetch `origin`, free the place `<component>`, and, when `impl/<component>` has landed (at least one of `origin/impl/<component>` and the local `impl/<component>` exists, and the tip of each that exists is an ancestor of `origin/main`), print `landed impl/<component>` after the line for the freed place and before any line about worktrees, also when no worktree has the branch checked out, then remove each worktree under the main worktree's `.claude/worktrees/` that has `impl/<component>` checked out and no uncommitted changes or untracked files that aren't ignored, unlocking it first, printing `removed <path>`, and print `kept <path>: uncommitted changes` for one that has them. It SHALL remove such a worktree whole, ignored files included, also when the build left a directory in it without its owner's write permission, changing no permission outside the worktree. When a worktree it set out to remove can't be removed, or is left partly on disk, it SHALL print `left <path>: <error>; delete the directory by hand, then run git worktree prune` in place of `removed <path>`, with the error on one line, go on with the other worktrees, and still print every other line and exit 0. When the branch hasn't landed it SHALL print no `landed` line, remove no worktree and push or delete no branch. The `landed impl/<component>` line is how `build` learns that a build whose subagent ended without a closing line landed (`unclosed-build-reads-finish`). It SHALL never remove a worktree outside `.claude/worktrees/`, and SHALL exit 0, also when there was no place, and 2 when any other git command fails.

#### Scenario: landed build tidied
- GIVEN a place `b`, and a clean, locked worktree at `.claude/worktrees/agent-1` with local `impl/b` checked out, whose tip is the second parent of a merge on `origin/main`, and no `origin/impl/b`
- WHEN `clm run --finish b` runs
- THEN it exits 0, the clone has no place `b`, the worktree is gone, and it printed `landed impl/b` and, after it, `removed` with its path

#### Scenario: unwritable directory removed
- GIVEN the landed build of "landed build tidied" with an ignored directory `cache/` in its worktree holding a file, `cache/` without its owner's write permission
- WHEN `clm run --finish b` runs
- THEN it exits 0, the worktree's directory is gone from disk and `git worktree list` doesn't name it, and it printed `freed b`, `landed impl/b` and `removed` with its path

#### Scenario: worktree git can't remove
- GIVEN the landed build of "landed build tidied" whose worktree `git worktree remove` refuses or can't delete
- WHEN `clm run --finish b` runs
- THEN it exits 0, the clone has no place `b`, and it printed `freed b`, `landed impl/b` and a line starting `left ` with the worktree's path and `git worktree prune`, and no `removed` line

#### Scenario: stopped build kept
- GIVEN a place `b`, `origin/impl/b` with a commit `origin/main` lacks, and a clean worktree at `.claude/worktrees/agent-1` with `impl/b` checked out at that commit
- WHEN `clm run --finish b` runs
- THEN it exits 0, printed no line starting `landed`, the clone has no place `b`, the worktree and `origin/impl/b` are unchanged; and a following `clm run b` resumes the branch without claiming

#### Scenario: dirty worktree kept
- GIVEN the landed build of "landed build tidied" with an untracked file `notes.txt` in its worktree
- WHEN `clm run --finish b` runs
- THEN it exits 0, the worktree and `notes.txt` are still there, and it printed `landed impl/b` and `kept` with its path

#### Scenario: checkout outside the worktrees directory
- GIVEN local `impl/b`, landed, checked out in a clean linked worktree at `../other` outside `.claude/worktrees/`
- WHEN `clm run --finish b` runs
- THEN that worktree is still there

#### Scenario: nothing to finish
- GIVEN no place `b` and no branch `impl/b`
- WHEN `clm run --finish b` runs
- THEN it exits 0 and prints no line starting `landed`

#### Serves
- capability/work-concurrently

### Requirement: pin-file
`clm-run`'s superpowers pin SHALL be the one line `<tag> <commit>` in `crates/clm-run/superpowers.pin`, read at compile time, and nothing else in the crate SHALL name the pinned tag or commit. The pinned commit `clm superpowers <skill>` and `clm run` resolve SHALL be that file's commit.

#### Scenario: one pin
- GIVEN `crates/clm-run/superpowers.pin`
- WHEN a test reads it and `clm-run`'s pinned tag and commit
- THEN the file is one line `v<MAJOR>.<MINOR>.<PATCH> <40 hex digits>`, and the tag and commit equal its two fields

#### Serves
- capability/pin-working-method

### Requirement: background-work-stopped
The `clm run` prompt SHALL tell the session to wait for a command it runs in the background, such as `clm land --check-only`, through that command's own completion notice, and never through a loop, watch or monitor that matches processes by name (such as `pgrep -f`), since such a match also finds other builds' processes and outlives the command it waits for; running a command in the background is a setting of the tool call, not a change to the command, so it doesn't break the rule of running the check as one plain command. It SHALL tell the session that a `clm land` it started, and when `clm.toml` sets `[land] check` a `clm land --check-only` it started, is waited for until its completion notice arrives, however long that takes, since lands queue behind other builds' lands and can take many minutes; that it never writes its final message, landed or stopped, while one of those is still running, and never stops one to end the build, since the orchestrating session reads the final line as the build's end and then frees its place and tidies its worktree. It SHALL tell the session that when its turn ends while a command it started in the background still runs, because waiting for that command's completion notice is all that is left, the message ending that turn is not its final message, since the orchestrating session receives it as the build's result: it carries no closing line, neither `landed <sha>` nor `stopped: <reason>`, and ends with the line `waiting: <command>`, naming the command it waits for; the final message comes in a later turn, once the command has ended. It SHALL tell the session that before it writes its final message it stops every other background task it started that is still running (background shells, monitors, watches) and waits for every subagent it launched to return, so that no task of a finished build wakes the orchestrating session later. It SHALL name no tool for stopping a task. These instructions SHALL be in every prompt, in the paragraph that tells the session to end its final message with `landed <sha>` or `stopped: <reason>`, and the text SHALL keep to the wording the component's other requirements allow (it names no `cd`, exit code or `clm watch`, and names `clm land --check-only` only when `clm.toml` sets `[land] check`).

#### Scenario: prompt
- GIVEN a ready component `b`, with and without a `[land] check` in `clm.toml`, and once resuming a branch with unlanded commits
- WHEN `clm run b` runs
- THEN in each prompt it writes, the paragraph that contains `stopped: <reason>` also contains `completion notice`, `pgrep` and `background task`, and says to stop those tasks before the final message

#### Scenario: a land still running
- GIVEN a ready component `b`, with and without a `[land] check` in `clm.toml`
- WHEN `clm run b` runs
- THEN in each prompt it writes, the paragraph that contains `stopped: <reason>` also contains `clm land`, `queue`, `many minutes` and `never stop one`, and says never to write the final message while a `clm land` the session started is still running

#### Scenario: a turn ends while waiting
- GIVEN a ready component `b`, with and without a `[land] check` in `clm.toml`
- WHEN `clm run b` runs
- THEN in each prompt it writes, the paragraph that contains `stopped: <reason>` also contains `waiting: <command>` and `not your final message`, and says that the message ending such a turn carries neither `landed <sha>` nor `stopped: <reason>`

#### Serves
- capability/work-concurrently

#### Needs
- plugin/build-waiting-builds

### Requirement: prompt-serves
`clm run`'s prompt SHALL say that the component's rules serve the capabilities under `# Capabilities served` in its context bundle, and, for the component that builds the capability tests (the graph's capability component, else its root), that each capability under that heading is one its tests prove at the project's public boundary, the capability component having no rules to serve them, that an architectural constraint is proven by the rules its `#### Enforced by` names and is never cited by a test (`human-layer-proof`), and that a requirement it asks a spec branch for under `## Backlog` names the entry it serves.

#### Scenario: prompt
- GIVEN a ready component `b`
- WHEN `clm run b` runs
- THEN the prompt it writes contains `# Capabilities served`, `#### Enforced by` and `serves`, and `architectural constraint`, and neither `req:architecture/` nor the word `architecture` followed by the word `requirement`

#### Scenario: capability node's prompt
- GIVEN the graph's capability component `capability`, ready
- WHEN `clm run capability` runs
- THEN the prompt it writes contains `# Capabilities served` and `public boundary`

#### Serves
- architecture/traced-rules
- capability/build-from-spec

#### Needs
- model/capability-tests-component

### Requirement: review-model
`clm run` SHALL take `--review-model <model>`, a model alias it passes into the prompt unchecked (`model-per-launch`). With it, the prompt SHALL tell the session to launch the `spec-auditor` subagent with the Agent tool's `model` set to `<model>`; without it, with `model` set to the alias of the session's own model, in the words `your own model`, since `clm run` has no alias to write. The prompt SHALL tell the session to name `model: "sonnet"` for any other subagent it launches, and never to launch a subagent without naming its model.

#### Scenario: review model given
- GIVEN a ready component `b`
- WHEN `clm run b --review-model opus` runs
- THEN it exits 0, and the prompt's paragraph that names `spec-auditor` contains `model: "opus"`, and the prompt contains `model: "sonnet"`

#### Scenario: no review model
- GIVEN a ready component `b`
- WHEN `clm run b` runs
- THEN it exits 0, and the prompt's paragraph that names `spec-auditor` contains `your own model` and no `model: "`, and the prompt contains `model: "sonnet"` and doesn't contain `model: "opus"`

#### Serves
- capability/right-model-per-task

### Requirement: check-tree-still
When `clm.toml` sets `[land] check`, `clm run`'s prompt SHALL tell the session that while `clm land --check-only` runs, in the foreground or the background, it changes nothing in its worktree: no commit, no edit to a tracked file, no `clm rebase` and no `git` command that moves `HEAD`. It SHALL give the reason: the command judges the working tree as it stands, so a change made while it runs can fail a check that reads `HEAD` and leaves a verdict on a tree the session no longer has. It SHALL say that work outside git, such as writing `.clm/pr-body.md`, may go on meanwhile, and that a change the session makes after the check started is followed by running `clm land --check-only` again once the check has ended. When `clm.toml` sets no `[land] check`, the prompt SHALL NOT give this instruction.

#### Scenario: tree held during the check
- GIVEN a ready component `b` and a `clm.toml` whose `[land] check` is `scripts/check.sh`
- WHEN `clm run b` runs
- THEN the prompt it prints contains ``while `clm land --check-only` runs``, says that the session commits nothing and edits no tracked file then, names `.clm/pr-body.md` as work that may go on, and says to run `clm land --check-only` again after a change made once the check started

#### Scenario: no check command
- GIVEN a ready component `b` and a `clm.toml` with no `[land]` table
- WHEN `clm run b` runs
- THEN the prompt it prints doesn't contain ``while `clm land --check-only` runs``

#### Needs
- clm/renamed-commands

#### Serves
- capability/land-safely

### Requirement: scratch-files
The `clm run` prompt SHALL tell the session to keep the scratch files it writes while working (context excerpts, notes, intermediate output) under its own worktree's `.clm/`, beside its plan and pull request body, which git ignores, and never in a scratchpad or temporary directory its system prompt names or in any other directory shared with other sessions, since the builds and drafts launched beside it are given the same directory and would overwrite one another's files. It says so in the paragraph that tells the session it is running unattended, in every prompt.

#### Scenario: prompt
- GIVEN a ready component `b`, once fresh and once resuming a branch with unlanded commits
- WHEN `clm run b` runs
- THEN in each prompt it writes, the paragraph that contains `running unattended` also contains `.clm/`, `scratch` and `scratchpad`

### Requirement: prompt-spot-checks
The `clm run` prompt SHALL tell the session, once the requirements it built pass their tests and before the review, to spot-check them by mutation: for each key rule it built, break the rule in the code on purpose and confirm that a test citing the requirement fails, adding or fixing a test when none does. It SHALL say how to run them safely: commit the work first, so that restoring a file from that commit loses nothing; count a mutant only when the code still compiles with it, since a build error fails every test and proves none; count it as killed only when a named test fails, read from the run's list of failures rather than its exit status; restore each mutated file from the commit and `touch` it, so a build tool that judges freshness by file times rebuilds it rather than keeping the mutant in the binary; run a script that loops over mutants with `sh`, not `zsh`, which doesn't split an unquoted list into words; mutate nothing while the repository's checks run; and end with `git status` showing no change to a tracked file. It SHALL tell the session to list each mutant, with the test that failed or the test it added, under `## Review` in the pull request body. The text SHALL be in every prompt, fresh or resuming, and keep to the wording the component's other requirements allow (it names no `cd`, exit code or `clm watch`).

#### Scenario: prompt
- GIVEN a ready component `b`, once fresh and once resuming a branch with unlanded commits
- WHEN `clm run b` runs
- THEN in each prompt it writes, the paragraph that contains `mutation` also contains `commit the work first`, `compiles`, `named test`, `touch`, `zsh`, `git status` and `## Review`, names no `spec-auditor`, and comes before the paragraph that names `spec-auditor`

#### Serves
- capability/prove-by-tests

### Requirement: review-every-diff
The `clm run` prompt SHALL tell the session that the review, `/code-review` and the `spec-auditor` subagent both, runs before every land whatever the diff's size or kind (only markers, only tests, a few lines), and that a small diff is never a reason to skip either reviewer. It SHALL tell the session to name both reviewers under `## Review` in the pull request body, each with its findings and what it did about them, or `no findings`. It SHALL name no case in which a reviewer may be skipped. The text SHALL be in every prompt, fresh or resuming, in the paragraph that names `spec-auditor`, and keep to the wording the component's other requirements allow (it names no `cd`, exit code or `clm watch`).

#### Scenario: prompt
- GIVEN a ready component `b`, once fresh and once resuming a branch with unlanded commits
- WHEN `clm run b` runs
- THEN in each prompt it writes, the paragraph that names `spec-auditor` also contains `/code-review`, `whatever the diff's size`, `never a reason to skip` and `no findings`

#### Serves
- capability/build-from-spec
- capability/prove-by-tests

### Requirement: prompt-workaround
`clm run`'s prompt SHALL name the four workaround triggers (a workaround to satisfy a scenario, a test that fakes something the requirement doesn't mention, a Design change in another component needed to build it, and a stop to raise a spec change or a scenario the code can't observe), tell the session to run `clm workaround <component>/<slug> --title <title>` with the situation on stdin when one happens and to commit the file with its work, say that a flag never changes the work or blocks the build except as the next sentence says, tell it to re-record a flag at the new version, or delete it, when its requirement moves under the build, and tell it to list its flags under `## Workarounds` in the pull request body. When `clm workaround` exits 3 (`check.health/workaround-command`: the capability has `workaround_flag` unsettled workarounds), the one exception, also when re-recording a flag, the prompt SHALL say the session keeps the flag, and the session SHALL stop working on that requirement, file a `question` item that names the capability and blocks the requirement, and build the rest (`redesign-before-workaround`, `requirement-blocks`). It SHALL tell the session that when the `clm` on its `PATH` lacks `clm workaround`, it lists the flag under `## Workarounds` alone, writes no flag file by hand, and carries on, so a missing flag command never stops the build.

#### Scenario: prompt
- GIVEN a ready component `b`
- WHEN `clm run b` runs
- THEN the prompt it writes contains `clm workaround b/`, `workaround`, `## Workarounds` and `never blocks`

#### Scenario: too many workarounds
- GIVEN a ready component `b`
- WHEN `clm run b` runs
- THEN the prompt it writes says that when `clm workaround` exits 3, the session stops working on that requirement, files a `question` item naming the capability and blocking the requirement, and builds the rest

#### Scenario: binary without the command
- GIVEN a ready component `b`
- WHEN `clm run b` runs
- THEN the prompt it writes says that when the `clm` on `PATH` lacks `clm workaround`, the session lists the flag under `## Workarounds` alone, writes no flag file by hand, and carries on

#### Serves
- capability/know-health
- architecture/redesign-before-workaround

### Requirement: says-component
The prompt `clm run` writes and the messages it prints SHALL say component, components and ancestors, never the old words, as `component-words` defines them: the prompt says ``Bring component `<id>`'s code up to its spec``, names the branch as claiming component `<id>`, says `## Fixes in other components`, and names the backlog key `components`; the messages say ``no component `<id>` `` and ``` `<id>` isn't a component ID ```. The names of decisions and architectural constraints the prompt cites keep their spelling. A test reads the string literals in `crates/clm-run/src/` and fails naming the file, the line and the word for each old word `component-words` doesn't keep.

#### Scenario: a build prompt
- GIVEN a ready component `a` with an uncited requirement
- WHEN `clm run a` runs
- THEN the prompt file it writes holds ``component `a` ``, and no old word but in a decision's or constraint's name

#### Serves
- architecture/plain-terms

### Requirement: capability-tests-follow-rule
The `clm run` prompt SHALL tell the session that when a requirement it builds changes what a test in the `capability` component's `paths` observes at the product's boundary (a screen, a path, a message or an output the test expects), it updates that test in its own branch, since its land runs the test and the `capability` component holds no requirement that could carry the move (`path-ownership`, `capability-tests-follow-rules`): it changes only the test's expectations, and the steps that reach them, where the requirement changed them, to what the requirement states; it keeps the test's citation as it is and drops no check the capability's scenarios, as `CAPABILITIES.md` now states them, state, so when the change edited the entry the test follows the entry's current text while its citation stays at the old version, leaving the capability `behind` for its own build to re-cite (`proven-by-tests`); and it names each such fix under `## Fixes in other components` in its pull request body. It SHALL tell the session that a requirement under which a capability entry, as `CAPABILITIES.md` now states it, no longer holds is not such a fix: the session leaves the requirement unbuilt, keeping the behaviour the test checks, and adds a `question` backlog item whose `components` name both components, with `blocks` naming the requirement, and which names the entry, so a spec change edits it. The text SHALL be in every prompt, beside `removal-breaks-dependant`'s, which it doesn't change. When the repository has no `capability` component, the text SHALL still be there, since it names no path.

#### Scenario: prompt
- GIVEN the prompt `clm run` builds for a component
- WHEN the tests read it
- THEN it says that a requirement that changes what a capability test observes updates that test's expectations, keeps its citation, names the fix under `## Fixes in other components`, and contains `capability-tests-follow-rules`

#### Scenario: entry no longer holds
- GIVEN the prompt `clm run` builds for a component
- WHEN the tests read it
- THEN it says that when a capability entry as written no longer holds, the session leaves the requirement unbuilt and adds a `question` item with `blocks` naming the requirement and naming the entry

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

### Requirement: prompt-pushes-through-clm
`clm run`'s prompt SHALL tell the session to push its branch with `clm branch push`, wherever it says to push, and its own text, outside the context bundle it embeds, SHALL NOT contain `git push` or `--force-with-lease`.

#### Scenario: fresh and resumed builds
- GIVEN a ready component `b` and a resumed branch `impl/c` with commits `origin/main` lacks
- WHEN `clm run b` and `clm run c` run
- THEN each prompt contains `clm branch push`, and its text outside the embedded context bundle doesn't contain `git push` or `--force-with-lease`

#### Needs
- host/branch-push
