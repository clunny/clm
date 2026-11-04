---
title: "The guard refuses a `clm land` piped into another command"
date: 2026-11-02
status: accepted
---

`clm hook guard` blocks a Bash command that pipes the output of `clm land`, `--check-only` included, into another command, and names the plain command to run instead (`plugin.hooks.guard/piped-land-guard`). It applies to every session, not only builds.

**Why.** `clm run`'s prompt tells a build to run `clm land --check-only` as one plain command, with no pipe or redirection, and says why (`prompt-check-command`). In one `/clm:implement` session three of seventeen builds still piped it, once `clm land` itself, into `tail`. A pipe reports `tail`'s result rather than the land's and cuts its output, so a failing check can read as passing. Nothing broke, since `clm land` re-runs the checks, but prose alone didn't hold the rule, and a rule that must hold is checked by a program (`deterministic-enforcement`). The guard already reads every Bash command a session runs, splits it as the shell would and keeps the separator that ends each segment, so the rule costs one check and no new reading (`reuse-first`).

**Why pipes only.** The observed mistake is the pipe. A redirection such as `2>&1` loses nothing, and one to a file needs a second command to read the output, which no build has done; the guard is a backstop for accidents, not a shell parser. Reading redirections would also need the shell reading (`plugin.hooks.guard.shell`) to keep them, since its tokens leave them out, for no mistake seen (`yagni`). The prompt still forbids both.

**Why every session.** The harm is the same wherever the land runs, and telling a build's worktree from any other would add a rule for no gain. A human who wants the land's output in a file runs it in their own terminal, which no hook guards.

**Alternatives.**
- *Keep the rule in the prompt only, with stronger wording.* Rejected: the prompt already gives the reason, and builds piped anyway.
- *Have `clm land` notice that its output is a pipe.* Rejected: Claude Code's Bash tool reads a command's output through a pipe too, so `clm land` can't tell the two apart.
- *Ask the human instead of blocking.* Rejected: unattended builds run under `dontAsk`, which denies an ask without a reason the build can act on, and there is nothing for a human to judge.
