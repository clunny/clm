---
title: "clm watch waits for the pushed head and holds back failures older than itself"
date: 2026-10-23
status: superseded
superseded_by: land-locally
---

Refines `watch-and-resume` and `watch-reruns-cancelled`. `statusCheckRollup` doesn't say which commit a check ran on, and GitHub registers a new run a few seconds after the push or label that triggers it. A `clm watch` started in that window judged the superseded run and reported a failure the caller had just fixed, so sessions restarted the watch up to three times.

So `clm watch` expects a head: `--head <sha>`, or else the local `refs/remotes/origin/<headRefName>`, which the caller's push has just updated. It keeps polling while the PR's `headRefOid` differs, which closes the push window. Ten polls of difference mean someone else moved the head, so it says so on stderr and judges the PR as it is rather than hanging. For the label window, where the head doesn't change, a failure that completed before the watch started counts as running for the first three polls, enough time for the new run to register. A watch on a PR that really failed earlier pays those three polls.

The output lines and exit codes are unchanged, so no skill or prompt has to change how it reads them.

Alternatives considered:
- A `--new-run` flag the caller passes after a push or label, making the watch wait for a run that started after it, with no limit. Exact, but every skill and prompt must pass it at the right times, and forgetting it brings the bug back.
- Reading check runs per commit through the REST API (`commits/<sha>/check-runs`). It closes the push window only, and adds a second API shape.
