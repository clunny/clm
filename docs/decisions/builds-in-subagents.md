---
title: "Nodes are built in worktree subagents of the human's session, and headless builds are gone"
date: 2026-10-24
status: accepted
---

`implement` builds each ready node in a background subagent of the human's session, launched with the Agent tool's `isolation: worktree`. `clm run <node>` keeps the rules: it removes merged leftovers, claims the node or picks up its unfinished branch, leaves a local `impl/<node>` at the claim, and prints the prompt, whose first step is `git switch impl/<node>`. It starts no `claude -p`. Builds run under the permission mode the human chose when they started the session: `clm run` sets no mode and no allowlist. `clm init` seeds a project's narrow allow rules for the `clm` commands unattended sessions run, since auto mode's classifier denied their pushes and lands (`unattended-steps-run-clm`).

**Why.** Headless builds ran under `dontAsk` with an allowlist, and anything off the list was denied without asking. The list kept growing (`unattended-commands`), prompts and agents carried rules that existed only to dodge its denials, protected-path nodes couldn't be built unattended at all, and every adopting repo would have needed its own list. Claude Code's subagents inherit the parent's mode, surface their permission prompts in the parent session in manual mode, and run the plugin's `PreToolUse` guard, so the human's own choice of mode governs the builds.

**Worktrees.** Two spikes on 2026-10-24 settled who makes the worktree. In a subagent without isolation, each Bash call starts in the parent's checkout, so a relative command or commit lands on the human's branch, and nested reviewers review the human's checkout. In an `isolation: worktree` subagent, each Bash call and each nested subagent starts in its own worktree, which starts from `origin/main`. Switching to a stand-in `impl/` branch, committing and running `clm` there all worked. Safety by construction beats a prompt rule (`deterministic-enforcement`). The cost: Claude Code keeps a worktree that has changes, locked, plus its `worktree-agent-*` branch, so `clm run` and `clm checkpoint` remove them once merged and clean, finding worktrees by the branch they have checked out, not by path.

**No headless path.** Every build runs in a Claude Code session today, so the `claude -p` launch, its log files, the superpowers `--plugin-dir` and the guard `--settings` go (`no-dead-code`), and with them the `interactive` queue state: a protected write is a prompt the human answers in their session, in any mode. A subagent dies with its parent session, but its claim and branch are in git, so the next `implement` resumes it. Unattended builds run through `/clm:build` in a session; there is no headless launch.

**Manual mode.** `implement` warns once that builds pause for the human's answers, and never refuses: pausing is what manual mode means.

**Superpowers.** A subagent uses the parent's plugins, so the prompt gives the pinned skills by path (`clm superpowers <skill>`), and builds follow the pinned version (`unattended-superpowers`).

**Shared binary.** Every build, and the human's session that launched them, runs the same installed `clm`. A build that ran `cargo install` of `clm` partway through its work, building it from a different revision than the one the others were using, replaced it under the others, and a subcommand the orchestrating session relied on disappeared mid-session. So the prompt tells a build never to install or replace that binary; it changes only outside builds, such as the main checkout's rebuild after a merge (`dogfood-from-main`). A guard hook that denies `cargo install` in builds was considered; the prompt rule comes first, and a hook can follow if builds keep doing it.

**Outcome from the last line.** Prompts read `clm rebase`'s outcome from its last line (`rebase ok`, `rebase incomplete`, `rebase failed`), not its exit code.

**Alternatives.** A subagent without isolation in a worktree `clm` makes keeps one layout but rests on the model remembering to `cd` in every call. Keeping headless as a setting (`builds = subagent | headless`, with a headless permission mode) serves terminals, cron and CI that nobody uses yet, at the cost of two launches, two worktree layouts and the interactive-node rules. Having the subagent run `clm run` headless itself brings back everything above.
