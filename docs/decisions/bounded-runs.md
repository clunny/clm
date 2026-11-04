---
title: "clm-git bounds a command's time with run_within, killing it when the limit runs out"
date: 2026-10-27
status: accepted
---

Refines `git-plumbing-layer` and `release-guard-runs-gh`. The release guard must answer within its 30-second lookup budget, before Claude Code's own hook timeout lets the command through (`plugin.hooks/release-guard`). `clm_git::run` waits for a command with no limit, so since `plugin.hooks/git-through-layer` the guard runs each `gh` lookup on a thread and gives up on it with `recv_timeout`. A stalled `gh` is then left running, orphaned, until it ends by itself; the guard's own wrapper, before the layer, killed it. Owning processes is the layer's job, so the limit belongs there.

`clm-git` gains `run_within(root, program, args, options, limit)`: `run` with a time limit. It drains the command's output on detached threads and polls `try_wait`; when the limit passes it kills and reaps the command and returns a new error variant, `TimedOut { command, limit }`, printed `<command>: timed out after <limit>`. It doesn't wait for the output pipes to close, since a process the command started may hold them. It kills only the child, not a process group: `git` and `gh` are run directly, and `std` kills one process portably. The guard passes what remains of its budget as each call's limit and drops its thread and channel.

`host/git-through-layer`'s scan counts `run_within(` calls as it counts `run(` calls, so the guard's one `gh` call stays the one allowed in `clm-hooks` whichever function it uses.

Alternatives considered:
- **A `timeout: Option<Duration>` field in `Options`**, as the backlog item proposed. `Options` is built by struct literal in other crates, and `clm-ledger` writes `Options { stdin, env }` without `..Default::default()`, so a new field breaks its build. `git`'s build may not edit another node's code to absorb its own change (`contract-removal-needs`), so the field would need a ledger requirement just to add `..Default::default()`. A separate function adds without breaking anyone (`expand-contract`), and keeps `run`'s callers, which need no limit, unchanged.
- **Killing the process group** (`process_group(0)` and `kill(-pgid)`). It would also end what `gh` starts, but it is Unix-only and needs `libc` or an unsafe call; the readers being detached already keeps a surviving grandchild from blocking the call.
- **Keeping the thread in the guard and killing the child from there.** The guard would need the child's ID from the layer, and every caller wanting a limit would repeat it.
