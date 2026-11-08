---
status: done
resolves: []
---

## Intent

Files `backlog/land-specs-still-name-a-local-main-base`: `land/derived-specs` and the decision `land-policy-from-base` still describe a local-main base no land uses.

The item this draft was given, `approval-still-reads-local-main-with-host-none` (`kind: deferred`), was code-only work: `land.approval/approval-from-base` already reads the base as `origin/main` on every host. It was done on `impl/land.approval` (`14b1f5b`, `deferred-work-resolves-in-code`): `clm-approval`'s `Repo` lost its `remote` flag, so its base is always `origin/main`, it always fetches, and it always reads `origin/<branch>`. So this change resolves nothing.

## The human's answers

- backlog/approval-still-reads-local-main-with-host-none: "Draft now (Recommended)" — remove the remote field and the local-main branch from clm-approval
