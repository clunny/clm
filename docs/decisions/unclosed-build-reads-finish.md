---
title: "A build that ends without a closing line is judged by `clm run --finish`"
date: 2026-11-04
status: accepted
---

`builds-in-subagents` has `build` read a build's closing line, `landed <sha>` or `stopped: <reason>`, and count a subagent that ends without either as a stop, built again. A build broke that: it handed back a result ending in `waiting: <command>` (`background-work-stopped`), its `clm land` then ran and merged its branch into `origin/main`, and its subagent ended with no further closing line. Read literally, the skill would have told the human it stopped and built it again; the session had to work out from `origin` that it had landed.

A missing closing line is the build's mistake, but the prompt already says what it should have done (`orchestrator.run/background-work-stopped`), and no wording of a prompt makes a subagent that runs out of context, or ends its turn badly, write a line. What `build` can always do is ask git. `clm run --finish <component>`, which `build` runs for every return before reading the closing line, already decides whether `impl/<component>` has landed, by the rule the clone's places use (each of `origin/impl/<component>` and the local branch that exists is on `origin/main`), to choose whether to remove the worktree. It now prints that decision as `landed impl/<component>`, and `build` reads it for a build with no closing line: landed means the build is treated as landed and not rebuilt; anything else is a stop, as before.

Rejected:
- **A stricter prompt only** (a fresh closing line after any `waiting:` hand-back). The prompt already requires it; repeating it doesn't cover a build that ends without writing one, and that is the case to handle.
- **Have the skill read `origin` itself** (`origin/impl/<component>` gone and a merge on `origin/main`). It repeats in prose a rule `clm run --finish` already computes in code, and a branch deleted from `origin` after a land looks like one never pushed; two readings of "landed" would drift (`reuse-first`).

Accepted edge case: a build that landed and then ended without a closing line never says which commit landed; the human is told it landed without its closing line, and `clm wrap-up` and `origin/main` show the rest.
