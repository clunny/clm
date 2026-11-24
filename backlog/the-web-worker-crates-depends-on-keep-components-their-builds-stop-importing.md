---
title: "The Worker's components keep in depends_on components their builds stop importing"
kind: deferred
components: [web.worker, web.worker.platform]
---

Wave 6's branch A (`20261124-wave-6-worker-by-the-rule`) left two `depends_on` entries that the components' builds make unneeded, because `check/undeclared-imports` judges today's imports against the graph a change leaves, and both imports were still there when the spec branch landed:
- `web.worker` keeps `context`: `crates/clm-web/Cargo.toml` lists `clm-context` as a dev-dependency nothing uses. `web.worker`'s build removes it and deletes `backlog/clm-web-lists-clm-context-as-a-dev-dependency-nothing-uses` with a `Resolves:` trailer.
- `web.worker.platform` keeps `orchestrator`: `crates/clm-web-platform/Cargo.toml` lists `clm-orchestrator` and `src/live.rs` imports `clm_orchestrator::ClaimRef` for `live`'s dead copies (`tips`, `claims`, `claim_refs`, `events`, `Claim`, `Tip`, `Activity`). `web.worker.platform`'s build removes them and the dependency, through `backlog/platform-fake-dead-helpers`.

Once both builds have landed, and no `clm_context` import is left in `crates/clm-web` and no `clm_orchestrator` import in `crates/clm-web-platform`, a small spec change does two things:
- drops `context` from `specs/web/worker/SPEC.md`'s `depends_on` and `orchestrator` from `specs/web/worker/platform/SPEC.md`'s (leaving `depends_on: [model, ledger, gate, replica]`), each with a `## MODIFIED Frontmatter` delta and no requirement changed;
- lists this item in its `change.md` `resolves`, so the item goes with that change. It isn't deleted by hand, which would make the branch mixed.

If only one build has landed, the change may drop that one entry and leave this item, narrowed to the other.
