---
component: orchestrator.runner
base: { design: "9e67d557833f201f" }
---

## MODIFIED Design

Crate `clm-runner`. It drives the other commands as subprocesses, through their Contracts: `clm next` for the work (one component ID per line, already capped at the free places), `clm run <component>` and `clm run --finish <component>` for preparing and finishing a build, by their exit codes and output. So the claim, the resume logic, the prompt, the places and the cleanup stay `orchestrator.run`'s and `orchestrator`'s, and a runner build is exactly the build `build` launches. The crate's entry takes the program it runs as `clm`, which `crates/clm` passes as its own executable, and the program it runs as `claude` (`claude` on `PATH`), so its tests run the library with fake scripts for both and need no real repository for the loop; the worktree requirements use a real git repository and a fake `clm`. `crates/clm`'s dispatch of `clm runner` is the root's file, edited by this component's build as for every new command. Its git calls (the worktrees, `git status --porcelain` for cleanliness) go through `clm-git`'s `git(root, args)` and `worktrees(root)`; it runs no `git` itself, which `vcs`'s scan test checks for every crate that depends on `clm-git` (`git-plumbing-layer`). Every `clm` subprocess runs with `--root <root>`, where `<root>` is the CLM repository the runner resolved.

The process-group and signal code sits under `#[cfg(unix)]`; elsewhere the entry only prints the refusal and exits 1. CLM is released only for Linux and macOS (`no-windows-support`).

**Starting.** It checks, in order, that `--permission-mode` was given (clap takes it as optional, so a missing one is the runner's exit 1, not clap's usage error) and that `claude` resolves on `PATH`, exiting 1 naming the first missing one before running anything. The mode's value is not checked.

**The loop.** Repeatedly: it runs `clm next --expiry <h>`. On the first look, a `clm next` exit 1 makes the runner exit 1 and an exit 2 makes it exit 2, each printing `clm next`'s stderr; on later looks it prints `clm next failed: <stderr>` and waits for the next look. For each component `clm next` prints, in order, that this runner isn't already running, it runs `clm run <component> --expiry <h>`:
- exit 0: it launches the session (below) and goes on to the next component;
- exit 3 (no free place): it launches nothing more until a session exits or the interval passes;
- exit 1: it prints `skipped <component>: <reason>` and skips the component until the next look;
- exit 2: it prints `git failed: <reason>` and launches nothing more until the next look.
`<reason>` is `clm run`'s stderr with each line's leading `clm run: ` removed and the lines joined by `; `. It then waits until one of its sessions exits, `--interval` passes or a signal arrives, and looks again. A component it is already running is never prepared twice, even when its own claim has lapsed mid-session and `clm next` lists it again. With `--once`, it exits 0 when a look started nothing and no session of its own is running. `clm next` and `clm run` fetch, so each look sees `origin` as it is.

**The worktree.** Each session runs in its own worktree at `<main>/.claude/worktrees/agent-runner-<component>`, where `<main>` is the main worktree (the first `git worktree list --porcelain` lists). There `clm run --finish` and `clm run`'s leftovers step find it by the branch it has checked out, as they find a subagent's, and the `agent-` prefix is what `clm wrap-up` matches (`orchestrator.checkpoint/checkpoint`), so every tidy-up that serves subagents serves it with no change to those components. The runner creates `.claude/worktrees/` when missing and adds the worktree detached at `origin/main` (`git worktree add --detach <path> origin/main`), as Claude Code starts a subagent's worktree from `origin/main`; the prompt's first step switches it to `impl/<component>`. When that path already holds a worktree (one `clm run` didn't tidy: detached or on another branch, such as a session that died before switching), the runner removes it first when it has no uncommitted changes or untracked files that aren't ignored, with `vcs`'s `remove_worktree`, which unlocks it and makes it removable first, so a directory the earlier session left without its owner's write permission doesn't stop it, and otherwise doesn't launch: it prints `kept <absolute path>: uncommitted changes` and runs `clm run --finish <component>`, which frees the place and keeps the claim; when the removal fails, it doesn't launch either: it prints `left <absolute path>: <error>; delete the directory by hand, then run git worktree prune`, the error joined onto one line, as `clm run --finish` does, and runs `clm run --finish <component>`, so the component comes back once the directory is gone; the component comes back when the claim lapses, and is skipped again until the human looks. When adding the worktree fails, it prints `git failed: <message>`, runs `clm run --finish <component>`, and goes on. In a CLM repository that is a subdirectory of its git repository, the session's current directory is the same subdirectory of the worktree.

**The session.** It starts `claude -p <prompt> --model sonnet --permission-mode <mode> --add-dir <root>/.clm/prompts` with the worktree as its current directory, so the session may read the prompt file `clm run` wrote there (`build-prompt-file`) though it lies outside the worktree. `<prompt>` is a fixed paragraph, a constant in this crate, then a blank line, then the launcher `clm run` printed, unchanged. The paragraph says: nobody watches this session, so Claude Code denies, rather than asks about, a tool call that needs confirmation (a command the settings don't allow, a write to a protected path such as `.claude/settings.json` or `.npmrc`, the guard's `ask` decisions); this overrides the prompt file's note that Claude Code may ask the human to confirm a call. The session treats such a denial as a step only the human can take: it never writes the file or runs the command another way, leaves the requirement that needs it unbuilt with a `human-task` backlog item whose `blocks` names it (`requirement-blocks`), builds the rest, and lands its branch with `clm land` even when the item is all it holds, so the item reaches `main` and the component settles; a denied `clm defect` is listed under `## CLM defects` in the pull request body instead. A component whose only pending requirements need such a step so ends settled, with the item waiting on the human, and the runner doesn't start it again. `--model sonnet` names the build's model, the cheaper one builds run on (`model-per-launch`); the runner has no main session, so it runs `clm run <component>` without `--review-model` and the build's final review runs on the build's own model. The runner adds no other flag and sets no variable: no settings file, no credential. Each session starts in a process group of its own, so a signal reaches the commands it started. Its stdout and stderr go, interleaved, to `<root>/.clm/logs/<component>.log`, replacing an earlier log of the component, `.clm/logs/` created when missing; `.clm/` is outside git (`orchestrator.run`'s exclude).

**When a session exits.** The runner runs `clm run --finish <component>`, passing its output through to its own stdout (a `--finish` exit 2 is printed as `git failed: <reason>` and the runner goes on), and prints `finished <component>: <last line>`, the last non-empty line of the session's log (`landed <sha>` or `stopped: <reason>` when the session followed its prompt), or `finished <component>: exit <code>` when the log has none. Then it looks for work at once. A session that ended without landing keeps its claim and pushed work, so the component comes back to `clm next` when the claim lapses and the runner, or any builder, resumes it. The runner sets no time limit on a session: the claim's expiry already hands a stuck build's component to another builder once it stops pushing.

**Stopping.** An interrupt or terminate signal (caught with a signal-handling crate such as `signal-hook`, since the standard library can't) ends a wait at once. The runner launches nothing more, sends each running session's process group a terminate signal, waits up to 30 seconds for each and then kills the group, runs `clm run --finish` for each component, and exits 0. Their claims and pushed work stay.

**Concurrency and subscriptions.** The runner starts at most what `clm next` prints and `clm run` admits, so `max_builds` bounds it. A machine signed in with a Pro or Max subscription, whose limits assume ordinary individual use, should set `max_builds = 1` in its `clm.local.toml`; that is the human's setting, not a rule the runner enforces.

## MODIFIED Requirements

### Requirement: runner-worktree-leftover <!-- base: 9bb429a7842f72e9 -->
When `<main>/.claude/worktrees/agent-runner-<component>` already holds a worktree, the runner SHALL remove it with `vcs`'s `remove_worktree` before adding the new one when it has no uncommitted changes and no untracked files that aren't ignored, so it goes whole also when a session left a directory in it without its owner's write permission; otherwise it SHALL start no session for the component, print `kept <absolute path>: uncommitted changes`, and run `clm run --finish <component>`. When that removal fails, it SHALL start no session for the component, print `left <absolute path>: <error>; delete the directory by hand, then run git worktree prune` with the error on one line, and run `clm run --finish <component>`.

#### Needs
- vcs/remove-worktree

#### Scenario: clean leftover replaced
- GIVEN a clean worktree at `<main>/.claude/worktrees/agent-runner-a`, detached, left by an earlier runner, and a fake `clm` whose `run a` succeeds
- WHEN the runner starts a session for `a`
- THEN the old worktree is removed and the session runs in a new one at that path, detached at `origin/main`

#### Scenario: dirty leftover kept
- GIVEN that detached worktree holds an uncommitted change
- WHEN the runner would start a session for `a`
- THEN it starts none, prints `kept` with the worktree's absolute path and `uncommitted changes`, runs `clm run --finish a`, and leaves the worktree as it was

#### Scenario: unwritable leftover replaced
- GIVEN the clean leftover of "clean leftover replaced" holding an ignored directory `cache/` with a file in it, `cache/` without its owner's write permission
- WHEN the runner starts a session for `a`
- THEN the old worktree is removed and the session runs in a new one at that path, detached at `origin/main`

#### Scenario: leftover git can't remove
- GIVEN the clean leftover of "clean leftover replaced", which `git worktree remove` refuses or can't delete
- WHEN the runner would start a session for `a`
- THEN it starts none, prints a line starting `left ` with the worktree's absolute path and `git worktree prune`, and runs `clm run --finish a`

#### Serves
- capability/unattended-runner
