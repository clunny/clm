---
title: "Failed checks carry their job, step and output; a resumed session re-runs a flaky failure and files it"
date: 2026-10-24
status: superseded
superseded_by: land-locally
---

Refines `watch-and-resume`. `clm watch` named only the failed checks, and a resumed `clm run` session can't read CI logs under `dontAsk`, so it couldn't tell a real failure from a flaky test: it re-ran everything locally, saw green, and pushed a commit only to restart CI. In one session that happened on three node PRs, each for the same flaky browser test.

So `clm watch` reads each failed check's `gh run view --job <id> --log-failed` output, as it already did for `gate`, and prints the check's step and the last 30 lines of that step's output under the `failed` line. `clm run`, resuming a PR, reads the same detail with the same code and puts it in the prompt, with each run ID. The session judges a failure flaky when its tests pass locally and it has nothing to do with the node's change: it files a `kind: bug` backlog item for the test unless one already names it, and when it pushes nothing (a push starts CI anyway) it re-runs the failed jobs with `gh run rerun <run-id> --failed`, the one command added to `clm run`'s `--allowedTools`. The judgement advises; CI's checks and the gate still decide what merges, and `implement` stops after two failures in a row for the same reason, so a test that always fails can't loop.

**Release runs.** `Bash(gh run rerun *)` matches any run, and re-running a `release.yml` run would publish, merge into `main` or deploy again. A permission pattern can't tell runs apart, so the guard (`humans-cut-releases`) refuses `gh run rerun` when a lookup of the run says it is the release workflow's, or can't say (`release-run-lookup` says how it looks the run up). That costs a `gh` call or two per re-run, in every session, and blocks nothing else.

Reading CI never stops anything: a log not uploaded after three polls is reported as `log unavailable`, and a resume whose checks can't be read says so in the prompt and carries on.

Alternatives considered:
- Letting sessions read logs themselves (`gh run view`, `gh pr checks`): less code, but each session digs through logs its own way at its own token cost, and the allow list widens more.
- A `clm rerun <pr>` command instead of allowing `gh run rerun`: no new `gh` permission, but a command to specify and test for one `gh` call (`reuse-first`).
- The guard allowing a re-run only of a run on the session's own branch: it would allow a release run from a session on `main`, and block an interactive session re-running another PR's CI.
- Deciding flakiness by rule (retry counts, a quarantine list): more machinery than the problem needs while flaky tests are few and each is filed as a bug.
