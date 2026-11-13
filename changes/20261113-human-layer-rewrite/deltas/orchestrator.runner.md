---
component: orchestrator.runner
base: { frontmatter: "933bfcadd20f06e0" }
---

## MODIFIED Frontmatter

```yaml
serves: [capability/build-from-spec, architecture/git-is-truth]
```

## MODIFIED Requirements

### Requirement: runner-launch <!-- base: 8acf1778b73d4baa -->
`clm runner --permission-mode <mode>` SHALL run `clm next` and, for each component it prints that the runner isn't already running, run `clm run <component>`; when that exits 0, it SHALL add a worktree at `<main>/.claude/worktrees/agent-runner-<component>`, detached at `origin/main`, and start `claude -p --model sonnet` in it with a prompt made of the runner's fixed unattended paragraph, a blank line and the launcher `clm run` printed, with `--permission-mode <mode>` as given and `--add-dir <root>/.clm/prompts`, writing the session's stdout and stderr to `<root>/.clm/logs/<component>.log`. It SHALL run `clm run <component>` without `--review-model`, and pass no other flag and set no environment variable for the session. It SHALL never run `clm run` for a component one of its sessions is running.

#### Scenario: ready node launched
- GIVEN a clone, a fake `clm` whose `next` prints `a` and whose `run a` prints a launcher and exits 0, and a fake `claude` on `PATH` that records its arguments, current directory and environment
- WHEN `clm runner --permission-mode acceptEdits --once` runs
- THEN the fake `claude` runs once in `<main>/.claude/worktrees/agent-runner-a` with `-p`, `--model sonnet`, a prompt that starts with the unattended paragraph, which names `human-task`, `blocks` and `clm land`, and ends with the launcher, `--permission-mode acceptEdits` and `--add-dir <root>/.clm/prompts`, and with the runner's environment unchanged
- AND its output is in `<root>/.clm/logs/a.log`

#### Scenario: running node not prepared again
- GIVEN the runner running a session for `a`, and a fake `clm` whose `next` now prints `a` and `b`
- WHEN the runner looks for work again
- THEN it runs `clm run b` and never runs `clm run a` a second time

#### Serves
- capability/build-from-spec
- capability/right-model-per-task

#### Needs
- model/parse-backlog

### Requirement: runner-refusals <!-- base: 76e90c8403491e24 -->
The runner SHALL go by `clm run`'s exit code: on 3 it SHALL start nothing more until one of its sessions exits or `--interval` passes; on 1 it SHALL print `skipped <component>: <reason>` and not run `clm run <component>` again before its next look; on 2 it SHALL print `git failed: <reason>` and start nothing more until its next look, where `<reason>` is `clm run`'s stderr with each line's leading `clm run: ` removed and its lines joined by `; `. None of them SHALL stop the runner. A `clm next` that exits 1 or 2 on the runner's first look SHALL make it exit with that code, printing `clm next`'s stderr; on a later look the runner SHALL print `clm next failed: <stderr>` and go on.

#### Scenario: no free place
- GIVEN a fake `clm` whose `next` prints `a` and `b`, and whose `run a` exits 3
- WHEN the runner looks for work
- THEN it doesn't run `clm run b` and starts no session until a session exits or the interval passes

#### Scenario: refused node skipped
- GIVEN a fake `clm` whose `next` prints `a` and `b`, whose `run a` exits 1 with `clm run: unpushed local branch impl/a` on stderr, and whose `run b` succeeds
- WHEN the runner looks for work
- THEN it prints `skipped a: unpushed local branch impl/a`, runs `clm run b`, and launches a session for `b`

#### Scenario: first look fails in git
- GIVEN a fake `clm` whose `next` exits 2
- WHEN `clm runner --permission-mode acceptEdits` runs
- THEN it exits 2 and runs no `clm run`

#### Serves
- capability/build-from-spec
- capability/work-concurrently

### Requirement: runner-worktree-leftover <!-- base: dd9862dba34ea5bf -->
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
- capability/build-from-spec

### Requirement: runner-prerequisites <!-- base: fd6e760872a9ae38 -->
`clm runner` SHALL exit 1, before it runs any other command, naming `--permission-mode` when the option is absent, and otherwise naming `claude` when no `claude` resolves on `PATH`. It SHALL pass the mode's value on unchecked. It SHALL never read, write or pass a Claude credential: it sets no variable for the sessions and reads none of Claude Code's files.

#### Scenario: no claude
- GIVEN no `claude` on `PATH`
- WHEN `clm runner --permission-mode acceptEdits` runs
- THEN it exits 1 naming `claude`, and runs neither `clm next` nor `clm run`

#### Scenario: no permission mode
- WHEN `clm runner` runs without `--permission-mode`
- THEN it exits 1 naming `--permission-mode` and runs nothing

#### Serves
- capability/build-from-spec

### Requirement: runner-once-and-stop <!-- base: c9603f1ea81771b8 -->
With `--once`, the runner SHALL exit 0 once a look starts no session and none of its sessions is running. On an interrupt or terminate signal it SHALL end any wait at once, start nothing more, send each running session's process group a terminate signal, wait up to 30 seconds for it and then kill the group, run `clm run --finish <component>` for each, and exit 0.

#### Scenario: once drains the queue
- GIVEN a fake `clm` whose `next` prints `a` on the first look and nothing after `a`'s session exits
- WHEN `clm runner --permission-mode acceptEdits --once` runs
- THEN it runs one session for `a`, finishes it, and exits 0

#### Scenario: stopped by a signal
- GIVEN a session for `a` still running, whose fake `claude` started a child process
- WHEN the runner receives a terminate signal
- THEN the session and its child receive a terminate signal, the runner runs `clm run --finish a` after they exit, and the runner exits 0 without running `clm run` again

#### Serves
- capability/build-from-spec

### Requirement: runner-needs-guard <!-- base: df4cfbe1d0748f49 -->
`clm runner` SHALL, after its `runner-prerequisites` checks and before it runs any other command, run `claude plugin list --json` in the clone's main worktree and exit 1, naming the `clm` plugin and saying that its sessions would run without the guard, unless the output holds a plugin whose `id`'s name part, before `@`, is `clm` and that is enabled for that directory (`enabled`, and `projectEnabled` when its `projectPath` is the main worktree), so that every unattended session it starts loads the guard (`guard-blocks-push-to-main`). When `claude plugin list` itself fails, it SHALL exit 1 printing its stderr. It reads none of Claude Code's files to decide.

#### Scenario: plugin missing
- GIVEN a fake `claude` whose `plugin list` lists no `clm` plugin
- WHEN `clm runner --permission-mode acceptEdits` runs
- THEN it exits 1 naming the `clm` plugin and the guard, and runs neither `clm next` nor `clm run`

#### Scenario: plugin disabled
- GIVEN a fake `claude` whose `plugin list` lists `clm` as disabled
- WHEN `clm runner --permission-mode acceptEdits` runs
- THEN it exits 1 naming the `clm` plugin

#### Scenario: plugin enabled
- GIVEN a fake `claude` whose `plugin list` lists `clm` as enabled, and a fake `clm` whose `next` prints nothing
- WHEN `clm runner --permission-mode acceptEdits --once` runs
- THEN it runs `clm next` and exits 0

#### Serves
- capability/build-from-spec
- architecture/land-locally
