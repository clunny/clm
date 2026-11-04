---
title: "The Worker's proposal and decide routes don't go through the object's /write"
kind: bug
components: [web.worker]
---

`web.worker`'s Design says a proposal's request "goes to the repo's `Replica` object (`POST /write`, `writes-through-object`), which checks the slug against its state brought up to the tip and appends", and its Contract says the Worker passes a user's token to the object for a decision's, a proposal's or a withdrawal's append. `web.worker.replica/writes-through-object` is built and tested on the object's side, but nothing in `crates/clm-web` posts to `/write`.

So `proposals::propose` judges a slug from `ledger::fold` and the snapshot view, `proposals::withdraw` reads the ledger itself, and `capabilities.rs`'s decide reads the spec files, the ledgers and the signers from GitHub and loads the graph on the Worker's request, against the Design's "a Worker request computes no view" and the 10 ms budget (`web-on-free-plan`).

No requirement of `web.worker` states the route, so the fix needs one, or the Design and the object's `/write` should go (`no-dead-code`).
