---
title: "clm land counts approvals under the approval policy committed at its base"
date: 2026-10-28
status: accepted
---

`clm land` loaded every setting from the root it ran in, so a branch landed from its own checkout was judged by the `approval` its own `clm.toml` sets. A branch that changes the value is `HARD` (`gate/approval-setting`, `approval-setting-is-spec`), but the approval it needed was counted under the policy it proposes: with `main` at `other`, a branch setting `any` landed on its author's approval, and `clm ledger verify`, which judges every signed HARD land by the `clm.toml` at the land's base, then reported it as `ledger/unapproved-land`. The land and the audit disagreed about the same commit.

`clm land` now takes `approval` from the top-level `clm.toml` committed at the base it lands on, the `origin/main` it fetched (or the local `main` without a remote), read the way `verify` reads it, so the two agree by construction: a branch that weakens the policy is approved under the stricter one in force, and one that tightens it lands under the one in force. Every other setting still comes from the root's own files: `host`, `[land] check`, the detectors and the rest describe the tooling the branch is built with, which `clm land` runs on the merged tree anyway, and nothing audits them after the fact.

A clone's `clm.local.toml` still sets `approval` for that clone's lands, as `config` allows (`local-settings`) and `approval-setting-is-spec` already accepts: the file belongs to the clone that is landing, so it can't be smuggled in by a branch, and `verify` reads the committed file alone, so it can't weaken the shared record. The alternative, ignoring the local file for this one key, would make `approval` the only setting `clm.local.toml` can't set, an exception `config`'s Contract would have to carry for a protection the audit already gives.

Rejected: having the gate refuse to land a branch that changes `approval` unless it is approved under the stricter of the two values. It needs the gate to read the ledger, which it doesn't (`clm gate` takes `--approved` from `clm land`), and leaves every other branch judged by the checkout. Also rejected: leaving it to `verify`. The audit runs only in signed repositories and after the land, when the record is already dirty.
