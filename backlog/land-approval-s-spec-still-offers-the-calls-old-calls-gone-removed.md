---
title: "land.approval's spec still offers the calls old-calls-gone removed"
kind: bug
components: [land.approval]
---

`land.approval/old-calls-gone` is built: `clm-approval` no longer has `Repo::head(branch, refuse_unpushed)` or the free `held(fold, branch, fingerprint)`. But `specs/land/approval/SPEC.md` still describes both as kept: the Contract lists `head(...)` on `Repo` ("kept until `clm-land` calls `branch_head`") and a bullet for the free `held` ("kept beside `Repo::held` until `clm-land` calls the method"), and the Design's **The branch** and **Held** paragraphs say `Repo::head` "stays beside `branch_head`" and the free `held` "stays until `clm-land` calls the method". Read as the current state (`spec-is-current`), they name an API that no longer exists. Expected: a spec branch drops both from the Contract and the Design, and decides whether `old-calls-gone` then stays as the rule that keeps them out. The wave's change (`20261121-wave-3-land-by-the-rule`) wrote the contract step's text in the future tense, so it went stale the moment the build met its condition; a change that plans an expand-contract may want to state the contract step's Contract edit in the same change, applied when the removal is built.
