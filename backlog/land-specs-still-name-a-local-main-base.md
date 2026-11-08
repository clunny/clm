---
title: "land's specs and a decision still name a local-main base that no land uses"
kind: bug
components: [land]
---

`land/offline-land` and `land.approval/approval-from-base` have every land made on `origin/main`, `host = "none"` included (`none-keeps-its-own-remote`), and `clm-approval` no longer keeps a local-main base (`backlog/approval-still-reads-local-main-with-host-none`, landed in `14b1f5b`). Two texts still describe the old base:

- `land/derived-specs` regenerates `specs/` "onto the base (`origin/main`'s, or the local `main`'s when landing offline)": no land is made offline or on the local `main` any more.
- `docs/decisions/land-policy-from-base.md` reads the policy from "the `origin/main` it fetched (or the local `main` without a remote)", with no `Refined by` line naming `none-keeps-its-own-remote`.

Expected: the requirement names `origin/main` alone (a modified requirement, so its tests are re-stamped), and the decision gets a `Refined by none-keeps-its-own-remote` line as `land-locally` and `host-adapters` have.
