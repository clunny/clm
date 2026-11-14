---
title: "The web Worker fits Cloudflare's Free plan"
date: 2026-10-22
status: accepted
---

The `web` Worker runs on the Free plan: 10 ms of CPU and 50 subrequests per request, which Cloudflare enforces by killing requests with a 503 once overruns stop being tolerated. To fit:
- A Worker request only checks access and answers stored views. The work of computing them runs in a `Replica` Durable Object per repo, which has 30 s of CPU per request or alarm (`web-repo-replica`); computed in the Worker, the views blew the 10 ms budget.
- It never downloads tarballs. It fetches only the spec files (`ARCHITECTURE.md`, `specs/`, `changes/`, `backlog/`, `docs/decisions/`) through batched GraphQL queries, since time spent waiting on the network doesn't count as CPU.
- Coverage comes from an incremental marker index. A push rescans only the files it changed. The object builds a repo's first index in slices of at most 50 subrequests, over its own alarms, and coverage shows as "indexing" until the index is complete.
- KV holds nothing the views read, and no Cron Trigger builds an index.

Coverage needs every tracked non-Markdown file, not only `specs/`, because `proven-by-tests` counts a requirement as implemented only when a test cites it.

What binds is the account's daily Durable Object allowance on the Free plan: 100,000 requests, 5 million rows read, 100,000 rows written, 13,000 GB-s and 5 GB stored. The Worker's 30-second cache of the views keeps polls from spending it. When usage outgrows the allowance, move to Paid.

Alternatives considered:
- Workers Paid ($5 a month), which would allow 30 s of CPU in the Worker itself and whole-tarball snapshots. Declined for now. Moving to it later only loosens limits.
- GitHub code search for markers. It lags, is rate-limited, and can't promise to match `clm coverage`.
- CI publishing coverage. Merges by the CI bot don't run CI on `main`.
- Showing status without coverage. That loses the proven counts, which are much of the point.
