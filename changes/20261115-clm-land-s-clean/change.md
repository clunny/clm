---
status: proposed
resolves: [clm-land-s-clean-up-runs-git-from-a-directory-it-already-removed]
---

## Intent

`clm land`'s clean-up after a land (deleting the landed branch on origin, clearing the build's `target/`) must not fail because the directory it runs git from is gone.
