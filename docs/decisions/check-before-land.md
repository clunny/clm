---
title: "A build runs the land check as one plain command, and a refused check is reported, not replaced"
date: 2026-10-26
status: accepted
---

`clm run`'s prompt names the command `clm land` will run (`clm.toml`'s `[land] check`) and tells the build to run it from its worktree's top level as one plain command, spelt as `clm.toml` spells it. When Claude Code refuses or denies it anyway, the build doesn't run a part of the check and call that the check: it reports the refusal with `clm defect`, lists it under `## CLM defects`, and lands with `clm land`, which runs the whole command on the merged tree and refuses a land that fails it.

**Why.** Two builds were refused `.github/scripts/ci-check.sh` in their `.claude/worktrees/` worktrees, ran only the Rust checks, and went on as if checked; `clm land` caught what they missed (`claude-code-s-worktree-guard-refused-the-repository-s-check`). The prompt said only "pass the repo's checks", so each build spelt the command its own way. Claude Code's worktree isolation refuses a command it can't show stays inside the worktree (compound forms, a variable where an option may stand, a script fed to an interpreter that names git), and an allow rule written for `.github/scripts/ci-check.sh` doesn't match the script by an absolute path. Run plainly from the worktree, a repository script runs: on 2026-10-26 an isolated subagent ran `.github/scripts/detector-tools.sh` that way, and also by its absolute path and through `bash` with a pipe, and none was refused, so the refusals came from how the command was spelt. Naming the exact command, and its plain form, removes the guesswork.

**Why not stop.** The land check is deterministic and `clm land` always runs it (`deterministic-enforcement`), so a build whose own run was refused can't land unchecked. Stopping would throw away a build whose code may be fine; landing through `clm land` lets the program decide, and the defect report tells the human the guard misfired.

**Alternatives.**
- **Wider allow rules** (`Bash(*/.github/scripts/ci-check.sh)`). Matches any path that ends that way, inside the repository or not, and doesn't lift the isolation's refusals, which aren't permission rules.
- **Stop the build on a refusal.** Safe but wasteful, as above. The item offered it as one way out; reporting and landing through `clm land` meets the same aim, since nothing lands on a partial check.
- **Name the script in the prompt as a constant.** Ties `clm-run` to this repository; `[land] check` is already the per-repository answer.
