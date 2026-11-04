---
title: Runner
status: active
owners: ["@clunny"]
depends_on: [orchestrator, orchestrator.run, vcs]
paths: ["crates/clm-runner/**"]
successor: ""
serves: [capability/unattended-runner, architecture/git-is-truth]
---

## Contract
`clm runner --permission-mode <mode>` turns a machine the human owns into an unattended builder for one clone (`machine-runners`). Run in the clone's checkout, it builds what `clm next` lists without anyone at the keyboard: for each component it prepares the build with `clm run <component>`, starts Claude Code headless (`claude -p`) in a worktree of its own for it, and when that session exits finishes it with `clm run --finish <component>`, then looks for more work. It pulls its work from git alone, through those commands: no service dispatches to it, and the claims, the places (`build-places`) and `clm land` coordinate it with every other builder, local or remote, as they do today. It holds no state of its own beyond the worktrees and logs of the sessions it is running, so stopping it loses nothing: a stopped build's claim and pushed work stay, and the next `clm run` of the component resumes it.

The runner never reads, stores, passes or sets a Claude credential. Each session it starts is the machine's own `claude`, unmodified, signed in however the human signed it in (an API key in their environment or their own subscription login), and inherits the runner's environment unchanged. How many sessions run at once is the clone's `max_builds` (`config`), since each build holds a place. The machine needs `claude` on `PATH`, signed in, with the `clm` plugin installed (for the guard and the `spec-auditor` subagent), and git credentials that can push `impl/*` and `main` as `clm land` needs.

`--permission-mode` is required and passed to `claude` unchanged, never checked: the human chooses how much the unattended sessions may do, as they choose their own session's mode for `build` (`builds-in-subagents`), and CLM ships no allowlist. Nobody answers a headless session's permission prompts, so a tool call that needs confirmation is denied: under any mode short of `bypassPermissions` that includes every command the machine's Claude Code settings don't allow, and in every mode a write to a path Claude Code protects and the guard's `ask` decisions. So a build gets past its first step only under `bypassPermissions` or with allow rules, in the machine's or the repository's Claude Code settings, for at least `git`, `clm` and the repository's check command; choosing which is the human's setup, not the runner's. The session is told to treat a denial as a step only the human can take. A runner can never approve or review a spec change (`approval-policy`).

It takes `--root` as every command does, `--interval <minutes>` (a positive decimal number, default 5), how long it waits between looks for work when nothing finishes, `--once`, to exit when nothing it can start is left and its sessions have all exited, and `--expiry <hours>` (default 4), passed to `clm next` and `clm run`. It exits 0 when stopped with an interrupt or terminate signal or when `--once` is done, 1 when it can't start (no `--permission-mode`, no `claude` on `PATH`, or its first `clm next` refused, with the reason), and 2 when its first `clm next` fails in git. It runs on Linux and macOS, the platforms CLM is released for (`no-windows-support`); a `clm` built from source elsewhere has `clm runner` exit 1 saying it runs on Linux and macOS, and start nothing.

## Design
Crate `clm-runner`. It drives the other commands as subprocesses, through their Contracts: `clm next` for the work (one component ID per line, already capped at the free places), `clm run <component>` and `clm run --finish <component>` for preparing and finishing a build, by their exit codes and output. So the claim, the resume logic, the prompt, the places and the cleanup stay `orchestrator.run`'s and `orchestrator`'s, and a runner build is exactly the build `build` launches. The crate's entry takes the program it runs as `clm`, which `crates/clm` passes as its own executable, and the program it runs as `claude` (`claude` on `PATH`), so its tests run the library with fake scripts for both and need no real repository for the loop; the worktree requirements use a real git repository and a fake `clm`. `crates/clm`'s dispatch of `clm runner` is the root's file, edited by this component's build as for every new command. Its git calls (the worktrees, `git status --porcelain` for cleanliness) go through `clm-git`'s `git(root, args)` and `worktrees(root)`; it runs no `git` itself, which `vcs`'s scan test checks for every crate that depends on `clm-git` (`git-plumbing-layer`). Every `clm` subprocess runs with `--root <root>`, where `<root>` is the CLM repository the runner resolved.

The process-group and signal code sits under `#[cfg(unix)]`; elsewhere the entry only prints the refusal and exits 1. CLM is released only for Linux and macOS (`no-windows-support`).

**Starting.** It checks, in order, that `--permission-mode` was given (clap takes it as optional, so a missing one is the runner's exit 1, not clap's usage error) and that `claude` resolves on `PATH`, exiting 1 naming the first missing one before running anything. The mode's value is not checked.

**The loop.** Repeatedly: it runs `clm next --expiry <h>`. On the first look, a `clm next` exit 1 makes the runner exit 1 and an exit 2 makes it exit 2, each printing `clm next`'s stderr; on later looks it prints `clm next failed: <stderr>` and waits for the next look. For each component `clm next` prints, in order, that this runner isn't already running, it runs `clm run <component> --expiry <h>`:
- exit 0: it launches the session (below) and goes on to the next component;
- exit 3 (no free place): it launches nothing more until a session exits or the interval passes;
- exit 1: it prints `skipped <component>: <reason>` and skips the component until the next look;
- exit 2: it prints `git failed: <reason>` and launches nothing more until the next look.
`<reason>` is `clm run`'s stderr with each line's leading `clm run: ` removed and the lines joined by `; `. It then waits until one of its sessions exits, `--interval` passes or a signal arrives, and looks again. A component it is already running is never prepared twice, even when its own claim has lapsed mid-session and `clm next` lists it again. With `--once`, it exits 0 when a look started nothing and no session of its own is running. `clm next` and `clm run` fetch, so each look sees `origin` as it is.

**The worktree.** Each session runs in its own worktree at `<main>/.claude/worktrees/agent-runner-<component>`, where `<main>` is the main worktree (the first `git worktree list --porcelain` lists). There `clm run --finish` and `clm run`'s leftovers step find it by the branch it has checked out, as they find a subagent's, and the `agent-` prefix is what `clm wrap-up` matches (`orchestrator.checkpoint/checkpoint`), so every tidy-up that serves subagents serves it with no change to those components. The runner creates `.claude/worktrees/` when missing and adds the worktree detached at `origin/main` (`git worktree add --detach <path> origin/main`), as Claude Code starts a subagent's worktree from `origin/main`; the prompt's first step switches it to `impl/<component>`. When that path already holds a worktree (one `clm run` didn't tidy: detached or on another branch, such as a session that died before switching), the runner removes it first when it has no uncommitted changes or untracked files that aren't ignored, and otherwise doesn't launch: it prints `kept <absolute path>: uncommitted changes` and runs `clm run --finish <component>`, which frees the place and keeps the claim; the component comes back when the claim lapses, and is skipped again until the human looks. When adding the worktree fails, it prints `git failed: <message>`, runs `clm run --finish <component>`, and goes on. In a CLM repository that is a subdirectory of its git repository, the session's current directory is the same subdirectory of the worktree.

**The session.** It starts `claude -p <prompt> --model sonnet --permission-mode <mode> --add-dir <root>/.clm/prompts` with the worktree as its current directory, so the session may read the prompt file `clm run` wrote there (`build-prompt-file`) though it lies outside the worktree. `<prompt>` is a fixed paragraph, a constant in this crate, then a blank line, then the launcher `clm run` printed, unchanged. The paragraph says: nobody watches this session, so Claude Code denies, rather than asks about, a tool call that needs confirmation (a command the settings don't allow, a write to a protected path such as `.claude/settings.json` or `.npmrc`, the guard's `ask` decisions); this overrides the prompt file's note that Claude Code may ask the human to confirm a call. The session treats such a denial as a step only the human can take: it never writes the file or runs the command another way, leaves the requirement that needs it unbuilt with a `human-task` backlog item whose `blocks` names it (`requirement-blocks`), builds the rest, and lands its branch with `clm land` even when the item is all it holds, so the item reaches `main` and the component settles; a denied `clm defect` is listed under `## CLM defects` in the pull request body instead. A component whose only pending requirements need such a step so ends settled, with the item waiting on the human, and the runner doesn't start it again. `--model sonnet` names the build's model, the cheaper one builds run on (`model-per-launch`); the runner has no main session, so it runs `clm run <component>` without `--review-model` and the build's final review runs on the build's own model. The runner adds no other flag and sets no variable: no settings file, no credential. Each session starts in a process group of its own, so a signal reaches the commands it started. Its stdout and stderr go, interleaved, to `<root>/.clm/logs/<component>.log`, replacing an earlier log of the component, `.clm/logs/` created when missing; `.clm/` is outside git (`orchestrator.run`'s exclude).

**When a session exits.** The runner runs `clm run --finish <component>`, passing its output through to its own stdout (a `--finish` exit 2 is printed as `git failed: <reason>` and the runner goes on), and prints `finished <component>: <last line>`, the last non-empty line of the session's log (`landed <sha>` or `stopped: <reason>` when the session followed its prompt), or `finished <component>: exit <code>` when the log has none. Then it looks for work at once. A session that ended without landing keeps its claim and pushed work, so the component comes back to `clm next` when the claim lapses and the runner, or any builder, resumes it. The runner sets no time limit on a session: the claim's expiry already hands a stuck build's component to another builder once it stops pushing.

**Stopping.** An interrupt or terminate signal (caught with a signal-handling crate such as `signal-hook`, since the standard library can't) ends a wait at once. The runner launches nothing more, sends each running session's process group a terminate signal, waits up to 30 seconds for each and then kills the group, runs `clm run --finish` for each component, and exits 0. Their claims and pushed work stay.

**Concurrency and subscriptions.** The runner starts at most what `clm next` prints and `clm run` admits, so `max_builds` bounds it. A machine signed in with a Pro or Max subscription, whose limits assume ordinary individual use, should set `max_builds = 1` in its `clm.local.toml`; that is the human's setting, not a rule the runner enforces.

## Requirements

### Requirement: runner-launch
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
- capability/unattended-runner
- capability/right-model-per-task

#### Needs
- model/parse-backlog

### Requirement: runner-refusals
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
- capability/unattended-runner
- capability/work-concurrently

### Requirement: runner-finish
When a session the runner started exits, the runner SHALL run `clm run --finish <component>`, passing its output through, print `finished <component>: <line>`, where `<line>` is the last non-empty line of the session's log, or `exit <code>` when the log has none, and then look for work again without waiting for `--interval`.

#### Scenario: landed build finished
- GIVEN a fake `claude` that prints `landed 1a2b3c4` and exits 0
- WHEN its session for `a` exits
- THEN the runner runs `clm run --finish a`, prints `finished a: landed 1a2b3c4`, and runs `clm next` again at once

#### Scenario: silent failure
- GIVEN a fake `claude` that prints nothing and exits 1
- WHEN its session for `a` exits
- THEN the runner runs `clm run --finish a` and prints `finished a: exit 1`

### Requirement: runner-worktree-leftover
When `<main>/.claude/worktrees/agent-runner-<component>` already holds a worktree, the runner SHALL remove it before adding the new one when it has no uncommitted changes and no untracked files that aren't ignored; otherwise it SHALL start no session for the component, print `kept <absolute path>: uncommitted changes`, and run `clm run --finish <component>`.

#### Scenario: clean leftover replaced
- GIVEN a clean worktree at `<main>/.claude/worktrees/agent-runner-a`, detached, left by an earlier runner, and a fake `clm` whose `run a` succeeds
- WHEN the runner starts a session for `a`
- THEN the old worktree is removed and the session runs in a new one at that path, detached at `origin/main`

#### Scenario: dirty leftover kept
- GIVEN that detached worktree holds an uncommitted change
- WHEN the runner would start a session for `a`
- THEN it starts none, prints `kept` with the worktree's absolute path and `uncommitted changes`, runs `clm run --finish a`, and leaves the worktree as it was

#### Serves
- capability/unattended-runner

### Requirement: runner-prerequisites
`clm runner` SHALL exit 1, before it runs any other command, naming `--permission-mode` when the option is absent, and otherwise naming `claude` when no `claude` resolves on `PATH`. It SHALL pass the mode's value on unchecked. It SHALL never read, write or pass a Claude credential: it sets no variable for the sessions and reads none of Claude Code's files.

#### Scenario: no claude
- GIVEN no `claude` on `PATH`
- WHEN `clm runner --permission-mode acceptEdits` runs
- THEN it exits 1 naming `claude`, and runs neither `clm next` nor `clm run`

#### Scenario: no permission mode
- WHEN `clm runner` runs without `--permission-mode`
- THEN it exits 1 naming `--permission-mode` and runs nothing

#### Serves
- capability/unattended-runner

### Requirement: runner-once-and-stop
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
- capability/unattended-runner

### Requirement: runner-needs-guard
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
- capability/unattended-runner
- architecture/guard-blocks-push-to-main

### Requirement: paragraph-says-pull-request
The fixed paragraph the runner puts before the launcher in each session's prompt SHALL say that a denied `clm defect` is listed under `## CLM defects` in the pull request body, and SHALL NOT hold the word `page`.

#### Scenario: the session's prompt
- GIVEN the runner starting a session for component `a`
- WHEN the fake `claude` records the prompt it was given
- THEN the prompt's paragraph names `## CLM defects` and `pull request body`, and doesn't hold the word `page`

#### Serves
- architecture/plain-terms

### Requirement: says-component
The paragraph the runner adds to a build's session and the lines it prints SHALL say component and components, never the old words, as `component-words` defines them. A test reads the string literals in `crates/clm-runner/src/` and fails naming the file, the line and the word for each old word `component-words` doesn't keep.

#### Scenario: the session paragraph
- GIVEN the runner's session paragraph
- WHEN the test reads it
- THEN it holds no old word

#### Serves
- architecture/plain-terms
