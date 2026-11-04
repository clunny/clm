---
title: "Unattended sessions name superpowers' skills and pin its version"
date: 2026-10-22
status: accepted
---

Refines `superpowers-for-execution`, and resolves its open check of whether the skills can be called this way.

The `superpowers-unattended` experiment (five headless runs on one small Rust task) found:
- **Loaded but unnamed skills did nothing.** With superpowers loaded but its skills not named in the prompt, no skill ran.
- **Named skills worked unattended.** They ran, the tests failed before the code made them pass, and nothing waited for a human.
- **Plain instructions matched them on that task**, at about two-thirds of the cost.

We use superpowers anyway: a toy task can't show what the fuller skills add, and writing our own copy would cut us off from upstream improvements (`reuse-first`).

So launched sessions:
1. follow a pinned superpowers version, given to the build by path;
2. name the skills they must use;
3. start with a preamble saying the session is unattended, so the skills' "ask your human partner" steps become rulings rather than stalls.

Claude Code creates the build's worktree (`builds-in-subagents`), so the worktree skill's consent step never applies. Upgrades are deliberate: a human moves the pin by hand after reading the release's notes (`cut-evals`, which removed the experiment's cases). A build in a subagent of the human's session (`builds-in-subagents`) gets the parent's plugins, not the pin, so its prompt gives each skill as the pinned copy's path, which `clm superpowers <skill>` prints, as `propose` does for brainstorming.

Correctness never rests on the skills, because CLM's checks and gates judge the result (`deterministic-enforcement`).
