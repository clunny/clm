---
title: "clm apply and clm archive fall short of two requirements"
kind: bug
components: [apply]
origin: 20261119-wave-1-by-the-rule
---

Found by the second builder check of wave 1 of phase 5 (2026-11-19), read from the code.

- `crates/clm-apply/src/lib.rs:852` and `:895`: `validate` stops at a delta's first bad frontmatter line and skips that delta's sections, where the Contract says it returns every reason the change can't be applied.
- `crates/clm-apply/src/lib.rs:140`: with both `changes/x/` and an archived `x`, `clm archive x` says there is no unarchived change, where `apply/archive-command` says it refuses because the archive entry exists. Not confirmed by a run.
