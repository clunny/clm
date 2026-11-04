---
title: "A change to clm.toml's approval setting is HARD spec work"
date: 2026-10-27
status: accepted
---

`approval` (`approval-policy`) lives in the repository's `clm.toml`, which the gate counted as code. So an `impl/*` branch could set `approval = "any"` and land on checks alone, and `ledger verify`, which judges each signed HARD land by the `clm.toml` committed at the land's base (`signed-ledger-records`), would from then on accept every self-approval: the rule that a person other than the author must approve could be switched off with nobody's approval. `docs/allowed_signers`, the other half of that protection, needs review to change too (`gate/needs-review`).

The gate now counts a change to `clm.toml`'s `approval` value as a spec change, which `path-tiers` already rates `HARD` (`gate/approval-setting`). `verify` judges the land that changes the policy by the `clm.toml` at its base, so turning `other` off needs another person's approval for the shared record to stay clean. (`clm land` itself counts approvals under the settings of the root it runs in, which may be the branch's own checkout; making it read the policy from `origin/main` is a separate backlog item, `land-reads-approval-from-main`.) Only the value counts, compared as parsed TOML, so the file's other settings and its comments stay code; a `clm.toml` that doesn't parse on either side fails closed. Changing the policy alongside code makes the branch `mixed`, which the gate already tells the author to split. The gate parses the file with the workspace's `toml` crate, which builds for `wasm32` like the rest of `clm-gate`.

A clone's `clm.local.toml` may still set `approval` for its own lands, as `config` allows: that changes only what that clone's `clm land` counts, and `verify` audits every signed land by the committed `clm.toml` at its base, so it can't weaken the shared record.

Rejected: moving the policy to a file under `docs/`, beside `docs/allowed_signers`. It gets the same protection with no new gate rule, but breaks every adopter that sets `approval` in `clm.toml` and needs a migration across two branches (the new file is `HARD` spec work, removing the key is code), while `config`, `land` and `ledger` would all change where they read it. Rating the whole of `clm.toml` `HARD` was rejected too: its other settings (`[land] check`, detectors, `[affected]`) are tooling that code changes routinely, which the deterministic checks already gate (`humans-review-intent`), and making each of their edits wait for a human would add reviews that guard nothing a code change can't already do.
