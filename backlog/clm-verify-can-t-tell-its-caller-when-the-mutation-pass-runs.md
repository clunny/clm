---
title: "clm-verify can't tell its caller when the mutation pass runs"
kind: deferred
components: [clm, verify]
---

`clm/verify-command` has `clm verify` catch `SIGINT` and `SIGTERM` "while the mutation pass runs" and let a signal "at any other time" end it as it would any process. `clm-verify`'s `verify_with` gives the root one moment only, `announce`, called with every outcome before the pass a green run goes on to; `Outcome::mutate`, which says whether a pass follows, is crate-private, and nothing marks the pass's end before `file::file` lands what passes found, which runs after it in `local` mode with `mutate` set whether or not a pass ran.

So the root (`crates/clm/src/main.rs`, `verify`) installs its handlers in `announce` when the outcome line starts `verify passed ` and keeps them until the call returns. A signal during the filing after the pass, or after a passed outcome that runs no pass (no `mutate`, or a `--commit` answered from the ledger), is caught and ends `clm verify` with 128 plus its number once the call returns, rather than at once. The spec auditor and the code review of the build both found it.

Settling it needs a spec change: either `verify`'s Contract offers the pass's bounds to its caller (a start and end hook on `Call`, or a public `Outcome` field saying a pass follows, with the filing counted as part of the pass, as `verify`'s Contract already describes it), or `clm/verify-command` states the window the root can see.

## The human's answer (2026-11-17)

The third choice: `clm/verify-command` is reworded to the window the root can see, from the passed outcome until `clm-verify`'s call returns, the pass and the filing of what it found together, as `verify`'s Contract already counts them. A signal during the filing is then held until the filing is done, which is safer than cutting a land in half; `clm-verify` gains no hook and no public field for it.

Due with the change that `backlog/a-mutation-pass-that-stops-starts-over-and-has-no-limit-on-its-time` becomes: it rewrites what a stop request means to a pass, and carries this rewording of `clm/verify-command`.
