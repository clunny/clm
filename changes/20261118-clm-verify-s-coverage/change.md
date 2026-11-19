---
status: proposed
resolves: [clm-verify-s-coverage-step-fails-because-clm-verify-build-doesn]
---

## Intent

`clm verify` creates its build place, the main worktree's `.clm/verify-build`, before every run of `[verify] check` and of `[verify] mutate` it hands that place as `CLM_LAND_DIR`, so a command can write there without making it first (`backlog/clm-verify-s-coverage-step-fails-because-clm-verify-build-doesn`).
