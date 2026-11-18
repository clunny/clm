---
title: "clm-coverage's crate doc still says clm-check re-exports it"
kind: bug
components: [check.coverage]
---

Step: building `check/coverage-lists` at `ee87592c` (`20261115-check-passes-on-no-coverage`). `check`'s build removed the `pub use clm_coverage::` re-exports from `clm-check`, and `/code-review` found this.

What happens: the crate doc at the top of `crates/clm-coverage/src/lib.rs` still ends with "`clm-check` re-exports this library unchanged." That is no longer true. It also contradicts both Contracts, which say `clm-check` "re-exports none of it". A build that reads the crate doc would expect `clm_check::Marker` and similar to compile, and they don't.

The fix: drop that sentence, or replace it with "Callers import it directly; `clm-check` re-exports none of it (`check-coverage-split`)." It is a doc comment in `check.coverage`'s own files, so `check`'s build couldn't change it.

The review also noted that `docs/decisions/land-approval-split.md` cites "as `clm-check` re-exports `clm-coverage`" as its precedent, in the present tense. That was true when the decision was made. A docs-only edit could put it in the past tense if wanted.
