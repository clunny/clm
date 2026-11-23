---
status: applied
resolves: [the-worker-source-reads-times-and-picks-files-by-its-own-copies-of-the-replica-s-rules]
---

## Intent

The Worker's `Replica` object (`web.worker.replica`) reads GitHub's times and picks the files it fetches by its own copies of rules that `replica` and `web.worker.platform` should own, and one copy is wrong. `web.worker.platform`'s `install::unix_time` reads no offset, checks no digits and accepts trailing text, so a ref whose `committedDate` is `2026-10-01T12:00:00+02:00` is taken as 12:00 UTC, two hours late: the claim lapses two hours late in the replica's implementation view, and in the Worker's live state, which reads the same call. And `clm-web-replica`'s `source.rs` keeps its own `is_spec` and `wanted`, though `replica` now offers `keeps_text` and `wanted` as the rule a source fetches by (`replica/files-it-fetches`); the two agree today, but if they drift the source never fetches a file `pending_fetch` lists and the replica stays `Preparing` for good.

This change states both as rules: `web.worker.platform/unix-time`, that `install::unix_time` reads a whole RFC 3339 time, offset included, and refuses anything else; `web.worker.replica/times-as-instants`, that the times the object gives the replica are the instants GitHub's times name; and `web.worker.replica/fetches-what-the-replica-wants`, that the object fetches exactly what `clm_replica::wanted` accepts, the files `keeps_text` accepts first. `web.worker.replica`'s Design says the source holds no copy of either rule.

Neither bug's behaviour was stated by a requirement (no requirement of either component says how a time is read, or which files the object fetches), so neither is a code-only fix (`code-fix-needs-a-written-rule`).

## Approaches

- **Fix `install::unix_time` in the platform, and have the replica object use it everywhere (chosen).** `web.worker.platform`'s Contract already offers `unix_time`; fixing it there mends the replica source's ref and event times and the Worker's own live state (`live.rs`'s `committedDate` and `created_at`) at once. `clm-web-replica`'s `commit_time` (`object.rs`), which reads a push's head commit `timestamp` by cutting it at 19 characters, appending `Z` and adding the offset itself, then calls `unix_time` on the whole text and loses its own offset arithmetic.
- **Have `clm-web-replica` call `clm-replica`'s RFC 3339 reader.** It is private to `clm-replica`'s `compute.rs`; offering it would widen `replica`'s Contract with a time parser that isn't the replica's business, and would leave the Worker's live state reading times wrong. Not taken.
- **One reader for the workspace.** `clm-replica` (`compute.rs`), `clm-orchestrator` (`places.rs`) and, after this change, `clm-web-platform` each hold a whole RFC 3339 reader. All three depend on `clm-ledger`, whose records' `at` is such a time, so `ledger` could offer one reader and the three call it. That spans four components, two of them being built in wave 4 now; it is left as an open question rather than folded in here (`yagni`, keep it small).

## Outcome

- `web.worker.platform`: `install::unix_time` reads `YYYY-MM-DDTHH:MM:SS`, an optional fraction, and `Z` or `±HH:MM`, counting the offset, and gives none for anything else. Its doc comment stops saying it reads only GitHub's `Z` form.
- `web.worker.replica`: `source.rs` drops `is_spec` and `wanted` and calls `clm_replica::keeps_text` and `clm_replica::wanted` (in `object.rs`'s compare filter and its spec-only `fetch_pending` too); `object.rs`'s `commit_time` reads the push's `timestamp` with `install::unix_time` whole. `clm_web_platform::files::is_spec_path` stays: `clm-web-platform`'s own `files.rs` and `clm-web-review` still use it.

Edge cases the change knows: none needing a workaround. A fraction of a second is dropped, not rounded; a leap second (`:60`) need not be read, since GitHub never gives one.

## Constraints

- `replica/files-it-fetches` is current, so `clm_replica::keeps_text` and `wanted` are on `main` to call.
- No public call another component uses changes its signature: `install::unix_time` keeps `fn(&str) -> Option<u64>`, and only refuses more text.

## Open questions

- Three whole RFC 3339 readers (`clm-replica`, `clm-orchestrator`, `clm-web-platform`) could become one that `ledger` offers. Worth a change of its own once wave 4 has landed?
