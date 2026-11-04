---
title: "Each repo's web views come from a replica kept up to date by its pushes"
date: 2026-11-02
status: accepted
---

The web app's views are served from a per-repo **replica**: the repo's derived state, held by the repo's Durable Object and updated from the webhook's change events. Every read is answered from the views the replica last computed, never computed while someone waits. Views may be up to a minute behind the latest push.

**Why.** The Worker computed the views from the whole spec tree read through GitHub: the snapshot in the push webhook and Cron, and the capabilities, architecture and implementation views on request, keeping them for 60 seconds. On the live Worker, a successful `/capabilities` request used about 350 ms of CPU and `/implementation` up to 900 ms, against the Free plan's 10 ms (`web-on-free-plan`). Cloudflare tolerates overruns for a while, then kills requests at 10 ms with a 503. In the 24 hours before this decision, 290 requests failed that way, cheap ones such as `/snapshot` and `/repos` among them, and the app showed each as "Not available: this repo isn't readable with your GitHub account". The expected use makes this worse. Many small teams open their repos now and then, so caches are usually cold, and busy periods such as hackathons bring frequent pushes and many readers at once. That's exactly when recomputing from scratch on request costs the most.

**How.**
- **Cost follows the change, not the repo.** On a push, the replica asks GitHub which files changed between the old and new tips (`compare`), fetches only those, and keeps the rest it already holds. It reads the whole tree only to build the replica the first time or to recover one.
- **Recomputation is batched.** A change marks the replica out of date and sets a timer (a Durable Object alarm). When the timer fires, at most about a minute later, the replica recomputes every view once, however many changes arrived meanwhile. A write made through the web app (a decision, an approval, a proposal) recomputes at once, so the person who made it sees it.
- **Reads are instant.** The Worker answers each read from the replica's last views after its access check, and keeps them briefly in the Cache API, so polls rarely reach the Durable Object.
- **Live state joins the replica.** Claims, spec branches, pull requests and events come from the webhooks the App already receives (`push`, which also reports a branch created or deleted, and `pull_request`) instead of per-request GitHub calls. A resync about every 10 minutes while a repo has open pages catches any webhook GitHub failed to deliver.
- **Computation is unchanged in kind.** The replica computes with the same `clm` crates the Worker uses today (`web-reuses-rust-in-wasm`), whole-tree, a few hundred milliseconds per recompute, within a Durable Object's 30 s CPU budget. Incremental computation per component waits until a repo needs it (`yagni`).
- **Its own Durable Object.** The replica runs in a `Replica` object per repo, apart from `web.live`'s `Live`, which keeps storing nothing (`web-live-channel`).
- **Host-neutral core.** The replica's logic (state, applying changes, recomputing, answering reads) is plain Rust that knows nothing of GitHub, Cloudflare or a browser. A *source* says what changed and fetches files, and a *store* persists state. This change builds the GitHub source and the Durable Object store. A local `clm serve` over a clone and an Azure DevOps source fit the same core later (`host-adapters`).

**Git stays the source of truth** (`git-is-truth`, `web-derived-cache`). The replica holds only what it derived from git and the host. Deleted, it rebuilds from git with nothing lost, and no check, gate or approval reads it.

**Budget.** The replica stays on the Free plan at launch. A Durable Object request gets 30 s of CPU, so the CPU limit no longer binds. What binds is the account's daily Durable Object allowance: 100,000 requests, 5 million rows read, 100,000 rows written, 13,000 GB-s of duration and 5 GB stored. The replica keeps its state in a few large chunks loaded once per wake, so recomputes and reads cost no rows read. The Worker's short cache of the replica's answers keeps an open tab's polls from each reaching the Durable Object. When usage outgrows the allowance, the account moves to Workers Paid ($5 a month plus usage), as `web-on-free-plan` already provides.

Alternatives considered:
- **Cache the computed reports in the Worker per commit.** A cache miss still costs 35 to 90 times the CPU budget, and with many repos opened now and then, most reads miss.
- **`clm land` publishes the finished views.** A file on a `clm/view` branch, copied into R2 by the webhook. Rejected: the views would be computed by whatever `clm` version each team landed with, so a change to a view's shape breaks repos that haven't landed since. Ledger changes made off `main` would need a second path.
- **`clm land` publishes the markers, and the browser computes.** No version skew, but every first visit downloads the spec tree and the WebAssembly build, and every view is recomputed on every device.
- **A replica owned by the host, pushed to by `clm`.** Fastest reads, but the host's copy would be the only home of the derived data, which `git-is-truth` rules out.
- **Views on GitHub itself** (generated Markdown, Mermaid, Projects). No buttons that run CLM logic, no live updates, no cross-repo home, and GitHub Pages is public for most private repos.
- **The browser reading GitHub directly.** Turned down in `web-polls-worker` and `web-derived-cache`: slow, and the page's script would hold the user's token.
