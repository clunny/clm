---
title: "Retired terms in spec text are caught only at land"
kind: bug
components: [clm]
origin: 20261110-clm-run-and-other
---

A spec branch whose requirement text names a retired command (here `clm sync`, now `clm rebase`) passes `clm check` and `clm gate`, and fails only at `clm land`, in `crates/clm/tests/retired_terms.rs` (`clm/old-command-names-retired`). A draft therefore learns of it only after queuing for a land check. Expected: the check that a spec text names no retired term runs where a spec branch is judged before the land (`clm check`, or `clm land --check-only`, which the draft prompt asks for only for hand edits), so a draft catches it before landing.
