---
title: "clm watch re-runs a cancelled check's run once, then reports it"
date: 2026-10-23
status: superseded
superseded_by: land-locally
---

Refines `watch-and-resume`. `clm watch` ignored `CANCELLED` checks, because the status rollup also lists runs that a newer run of the same workflow superseded. But when the latest run of a check is cancelled, nothing restarts it, so `clm watch` waited forever on a PR whose checks had otherwise passed, and only the human noticed.

Now `clm watch` judges each check by its latest entry. A latest entry that is cancelled, with nothing still running, gets one `gh run rerun` from `clm watch`, which keeps polling. A second cancellation prints `failed <check> (cancelled)`, so the session decides what to do. A re-run changes nothing in the repository and is what the human did by hand each time, so `clm watch` does it without asking. Re-running only once keeps a run that is cancelled on purpose from looping.

Rejected:
- **Only reporting it, as `failed <check> (cancelled)`.** Every caller would then need the same re-run step, and the skills' `failed` handling (read the failure, fix it, push) doesn't fit a run with nothing to fix.
- **Re-running without limit.** A run someone cancelled deliberately would be restarted for as long as the watch runs.
