---
title: "A build runs the land check as `clm land --check-only`, not by the script's path"
date: 2026-10-27
status: accepted
---

`clm land --check-only` runs `[land] check` and then `clm check` on the current working tree, as a land runs them (`CI` unset; `CARGO_TARGET_DIR` as the caller has it, so a build checks in its own `target/`, `build-places`), and lands, fetches and records nothing. `clm run`'s prompt names it wherever a build passes the repository's checks, instead of the configured command's path.

**Why.** Claude Code's worktree isolation, which guards a build running in an isolated worktree, refused every command naming `.github/scripts/ci-check.sh`, this repository's `[land] check`. The prompt's rule of one plain command, spelt as `clm.toml` spells it, wasn't enough: the build learnt the check's result only when `clm land` ran it. A `clm` subcommand names no path for the guard to judge, works the same for every repository whatever its `[land] check` is called and wherever it lives, and keeps the build's check exactly the land's, since both run through the same code in `clm-land`.

**Why it records nothing.** A `checked` record names a committed tree's code key, and a land checks the merge with `origin/main`, which may have moved. The working tree a session checks may hold uncommitted edits and isn't that merge, so a passing `--check-only` never lets a land skip its own check.

**Alternatives.**
- *Move the script out of `.github/`* (to `scripts/ci-check.sh`, like `clm init`'s `scripts/clm-check.sh`). Rejected: the guard's rule isn't documented, so another path may be refused too, and adopters' paths vary; the move also touches `ci`'s requirements, `clm.toml`, the allowlist and their tests for no gain over a `clm` command.
- *Tell builds to rely on `clm land` for the check.* Rejected: the prompt already falls back to that when the command is refused, and it costs a land attempt per failure; a build should fail fast in its own worktree.
- *Run the land in a dry-run mode (fetch, merge with `origin/main`, check, no push).* Rejected for now: it needs the branch pushed first and checks a merge the session hasn't made; `clm sync` before landing already brings `main` in.
