---
title: "Unattended sessions may run read-only helpers, linters, chmod +x and the repo's check script"
date: 2026-10-24
status: superseded
superseded_by: builds-in-subagents
---

`clm run`'s `--allowedTools` allows the read-only text helpers `echo`, `sed -n`, `sort`, `uniq`, `cut`, `tr`, `nl`, `diff` and `comm`, the linters `bash -n`, `shellcheck` and `actionlint`, and `chmod +x`, in every CLM repository. This repository's `.claude/settings.json` also allows `.github/scripts/ci-check.sh`, which installs Playwright's browser only when `CI` is `true`. That extends `unattended-commands`, which still governs npm and `rustup`.

**Why.** Under `dontAsk`, Claude Code denies a compound command unless each part is allowed. The subagents a session launches, the `spec-auditor` and the forked `/code-review`, never see `clm run`'s one-command-per-call rule, and they write pipelines such as `git diff ... | sed -n ...` or `clm context x; echo ---`. Every such call was denied, so the reviews ran on a guessed diff. The human chose to widen the rules rather than teach the subagents to run one command per call: teaching can't reach the built-in `/code-review`. The linters, `chmod +x` and `ci-check.sh` are commands plans and instructions already ask unattended sessions to run.

**What widening can't reach.** Under `dontAsk`, Claude Code denies `cd` followed by `git` whatever the rules say, and subshells are its call, not an allow rule's. So the `spec-auditor`'s instructions also say to run each command from the directory it was launched in, never starting one with `cd` or wrapping commands in `( ... )`.

**What stays out.** `awk` (its `system()` runs any command), `find` (`-exec`, `-delete`), `xargs`, `env` and `timeout` (each runs any command given to it), and `sed` without `-n` (`-i` edits files in place). `VAR=...;` prefixes are, like subshells, Claude Code's to approve.

**What it costs.** `sed -n` can still write a file with `w`, `sort -o` and `uniq`'s second argument write their output to a file, and `ci-check.sh` runs whatever the session writes into it. None of that reaches further than `Bash(git *)`, which sessions already have, or `cargo test`, which runs the session's own code. The allowlist stops accidents, not a session that means harm. The `PreToolUse` guard still blocks what must never happen unattended (`guard-blocks-push-to-main`).

**Browsers.** `npx playwright install --with-deps chromium` uses `sudo`, which `unattended-commands` keeps out of sessions. Gating it on `CI` keeps that: CI installs the browser, and on a machine the human installs it once.

**Alternatives.** Putting everything in this repository's `.claude/settings.json` would leave every other CLM repository's sessions with the same denials. Telling `spec-auditor` to run one command per call keeps the rules tight but leaves `/code-review` denied.
