---
title: "The web Worker fits Cloudflare's Free plan"
date: 2026-10-22
status: accepted
---

The `web` Worker runs on the Free plan: 10 ms of CPU and 50 subrequests per request, and 1,000 KV writes a day. To fit:
- It never downloads tarballs. It fetches only the spec files (`ARCHITECTURE.md`, `specs/`, `changes/`, `backlog/`, `docs/decisions/`) through batched GraphQL queries, since time spent waiting on the network doesn't count as CPU.
- Coverage comes from an incremental marker index. A push rescans only the files it changed. A repo's first index is built in slices by a Cron Trigger, and coverage shows as "indexing" until the index is complete.
- KV holds only snapshots, the index and head pointers: about two writes per push to a default branch. PR, claim and event state lives in the Workers Cache API for 60 seconds, which costs no KV writes.

(Amended 2026-11-02, `20261102-web-repo-replica`: the views blew the 10 ms budget, which Cloudflare enforces by killing requests with a 503 once overruns stop being tolerated. The work moved to a `Replica` Durable Object per repo, which has 30 s of CPU per request or alarm (`web-repo-replica`). A Worker request now only checks access and answers stored views. KV holds nothing the views read, and the Cron index is gone: the object builds a repo's index in slices of at most 50 subrequests, over its own alarms. What binds now is the account's daily Durable Object allowance on the Free plan: 100,000 requests, 5 million rows read, 100,000 rows written, 13,000 GB-s and 5 GB stored. The Worker's 30-second cache of the views keeps polls from spending it. When usage outgrows the allowance, move to Paid.)

Coverage needs every tracked non-Markdown file, not only `specs/`, because `proven-by-tests` counts a requirement as implemented only when a test cites it.

Alternatives considered:
- Workers Paid ($5 a month), which would allow 30 s of CPU and whole-tarball snapshots. Declined for now. Moving to it later only loosens limits.
- GitHub code search for markers. It lags, is rate-limited, and can't promise to match `clm coverage`.
- CI publishing coverage. Merges by the CI bot don't run CI on `main`.
- Showing status without coverage. That loses the proven counts, which are much of the point.

At about two KV writes per push, the Free plan supports a few hundred pushes a day across all repos. Past that, move to Paid.
