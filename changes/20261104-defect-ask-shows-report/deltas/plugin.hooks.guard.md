---
component: plugin.hooks.guard
base: { design: "be76f27b4aa5f2ff" }
---

## MODIFIED Design
Crate `clm-guard`, at `crates/clm-guard/` (`guard-split`): `lib.rs` (`guard`, `Decision`, `ask_json`, and the `clm` call a segment makes), `push.rs`, `release.rs` and `gh.rs`. Its module `shell` is `plugin.hooks.guard.shell`: `guard` walks the command once and judges each piece the walk yields, with that piece's checkout. It loads the `defects` setting through `clm-hooks`' `settings(cwd)`, the one way the hooks and `clm defect` load settings, so they agree on the repository. It runs `git` and `gh` only through `clm-git` (`git-through-layer`). The scenarios that run the built binary (a fake `gh` or `git` on the `PATH`, the time budgets, the defect ask) are tested in `crates/clm/tests/guard.rs`, since Cargo gives only the root package's tests the binary (`tests-count-where-cited`). A test file cites this component's requirements or the shell's, never both, so each counts toward one component's size.

A `cargo run` is how this repository's `CLAUDE.md` tells sessions to run CLM, so it is the documented way, not obfuscation, but the guard doesn't work out which package it runs or which tokens reach the binary (`guard-asks-any-cargo-run`, `approve-guard`). Asking too often is the safe side, so another binary's `approve` and a value that happens to be the word, as in `--title approve`, are asked about too. A segment's `git` call is found the same way as a plain `clm` call, so prose that quotes a command, such as a pull request body, isn't mistaken for one.

**Pushes** (`push-guard`). The command hasn't been allowed yet, so the guard passes git nothing from it beyond the options the requirement lists. A query git can't answer (no upstream, no tracking ref for the destination, or a `simple` push it would refuse anyway) decides nothing. `@{push}` follows `push.default=upstream` on a branch cut from `origin/main`. The 20-second budget of the push queries and the 30-second budget of the release lookups (`release-guard`) together stay under Claude Code's 60-second hook timeout, which would let the command through; the clock for the push queries starts at the command's first push query. A query that runs out of time blocks the push: git stalling, as on a `--git-dir` on a hung file system, is no answer. A `remote.<name>.push` refspec counts whichever remote it names, since asking which remote a push uses would need more of git's rules than a backstop is worth. The branch is always `main`, the only base CLM's flow uses (`no-stacked-prs`). GraphQL's `updateRef` and `updateRefs` are blocked for any ref, since their ref IDs can't be read. CLM never merges a pull request (`land-locally`), so none of the host routes to `main` is part of its flow. `clm land` pushes `main` from inside the binary, not through a guarded `git push`, so it passes; what keeps a session from landing an unapproved branch that needs review is `clm land`'s gate, which is deterministic (`land-locally`). The guard is a backstop for accidents, not a shell parser: `eval`, a command held in a variable, an `export`ed `GIT_DIR`, a script file, a refspec `xargs` reads from stdin, a `cd` into a directory on a hung file system (resolving it isn't a git query, so it has no budget), a command's output piped into a shell (`echo '...' | bash`), a refspec held in a variable (`git push origin $B`, or a `for` loop's variable), which a session's own scripts write for their branches so that treating it as possibly `main` would block them, a GraphQL query read from a file, and the contents API writing a file to `main` get through.

**Re-run lookups** (`release-run-lookup`). With no `-R`/`--repo`, the guard asks `gh repo view --json url` which repository a `gh run rerun` targets, since `gh api` takes `{owner}/{repo}` from the checkout but its host from `GH_HOST` or the default host, not from that repository; the lookup then asks the host the re-run asks.

**Approval** (`approve-in-session`). The `ask` permission decision is `{"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "ask", "permissionDecisionReason": ...}}` on stdout and exit 0. Claude Code then shows the human the reason before the command runs, in any permission mode; it doesn't promise to show the command beside it, so a reason that asks the human to judge text quotes that text; unattended sessions run under `dontAsk`, which denies `ask`, so they can never approve. A `clm defect` ask (`defect-approval`) is made the same way; its reason quotes each `clm defect` line and the report it sends (`defect-ask-shows-report`), says the report leaves the project as a public issue, and that when no one can approve (an unattended session), the defect goes under `## CLM defects` in the pull request body instead.


## MODIFIED Requirements

### Requirement: defect-approval <!-- base: 1989a7b39aa6059f -->
`clm hook guard` SHALL judge a `Bash` command with a segment that runs `clm defect`, or a `cargo run` that `approve-guard` treats as running it, whose arguments hold neither `--help` nor `-h`, when the `defects` setting for the event's `cwd` is `ask` or can't be loaded and no other guard rule blocks the command (`defect-ask-shows-report`). A segment's report *can be shown* when its standard input (`guard-standard-input`) is a heredoc the shell sends as written. When every such segment's report can be shown, the guard SHALL print `hookSpecificOutput` with `permissionDecision: "ask"` and a reason that names `## CLM defects` and quotes, for each such segment in order, its own text as written, without its body, and its report, the body whole, and exit 0. When a segment's report can't be shown, it SHALL exit 2, writing to stderr a reason that names `defect-ask-shows-report`, says the guard can't show the report to the human, and tells the session to send the report as the body of a `<<'EOF'` heredoc on the `clm defect` line. When another guard rule also asks, it SHALL print one `ask` decision whose reason gives both rules' reasons. No guard rule SHALL match the heredoc body of such a segment. With `backlog` or `off` it SHALL treat the segment like any other.

#### Needs
- plugin.hooks.guard.shell/guard-standard-input

#### Scenario: asking
- GIVEN an event whose `cwd` has no settings, and a command whose first line is `clm defect --title "x" --component gate <<'EOF'`, followed by a line `The gate misfired.` and `EOF`
- WHEN `clm hook guard` runs on it
- THEN it prints the `ask` decision with a reason naming `## CLM defects` and holding `clm defect --title "x" --component gate` and `The gate misfired.`, and exits 0

#### Scenario: asking through cargo run
- GIVEN an event whose `cwd` has no settings, and a command whose first line is `cargo run -q -p clm -- defect --title "x" <<'EOF'`, followed by a line `The gate misfired.` and `EOF`
- WHEN `clm hook guard` runs on it
- THEN it prints the `ask` decision with a reason naming `## CLM defects` and holding `The gate misfired.`, and exits 0

#### Scenario: an unquoted heredoc with nothing to expand
- GIVEN an event whose `cwd` has no settings, and the command `clm defect --title x <<EOF` followed by a line `plain words` and `EOF`
- WHEN `clm hook guard` runs on it
- THEN it prints the `ask` decision with a reason holding `plain words`, and exits 0

#### Scenario: a report it can't show
- GIVEN an event whose `cwd` has no settings, and each of the commands `clm defect --title x < report.md`, `cat report.md | clm defect --title x`, `clm defect --title x <<< "$R"`, `clm defect --title x`, and `clm defect --title x <<EOF` followed by a line `cost $HOME` and `EOF`
- WHEN `clm hook guard` runs on each
- THEN each exits 2 with a reason naming `defect-ask-shows-report` and `<<'EOF'`

#### Scenario: help
- GIVEN an event whose `cwd` has no settings, and the command `clm defect --help`
- WHEN `clm hook guard` runs on it
- THEN it prints nothing and exits 0

#### Scenario: backlog
- GIVEN an event whose `cwd` is a repository with `defects = "backlog"`, and the commands `clm defect --title "x" <<'EOF'` followed by a report, and `clm defect --title x < report.md`
- WHEN `clm hook guard` runs on each
- THEN each prints nothing and exits 0

#### Scenario: quoting a blocked command
- GIVEN an event whose `cwd` has no settings, and a `clm defect --title "x" <<'EOF'` command whose report has a line `clm approve spec/x` and a line `git push origin main`
- WHEN `clm hook guard` runs on it
- THEN it prints the `ask` decision and exits 0

#### Scenario: block wins
- GIVEN an event whose `cwd` has no settings, and a command with a segment running `clm defect` with a `<<'EOF'` report and a segment `git push origin main`
- WHEN `clm hook guard` runs on it
- THEN it exits 2 naming `guard-blocks-push-to-main`

#### Scenario: two asks
- GIVEN an event whose `cwd` has no settings, and a command with a segment running `clm defect` with a `<<'EOF'` report and a segment `clm approve spec/x`
- WHEN `clm hook guard` runs on it
- THEN it prints one `ask` decision with a reason naming both `## CLM defects` and `humans-review-intent`, and exits 0

#### Serves
- capability/report-defects
- capability/guard-sessions
- architecture/approval-policy

### Requirement: defect-ask-says-pull-request <!-- base: 796adc3fd4f7aa6e -->
The reason of the `clm defect` ask (`defect-approval`) SHALL say that, when no one can approve, the defect goes under `## CLM defects` in the pull request body, and no reason the guard gives SHALL call a branch's pull request a page.

#### Scenario: the defect ask
- GIVEN `defects = "ask"`
- WHEN `clm hook guard` reads a `Bash` event running `clm defect --title x` with its report in a `<<'EOF'` heredoc
- THEN the `ask` decision's reason names `## CLM defects` and `pull request body`, and doesn't hold the word `page`

#### Serves
- architecture/plain-terms
