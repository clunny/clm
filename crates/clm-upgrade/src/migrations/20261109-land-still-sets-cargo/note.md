Lands no longer set CARGO_TARGET_DIR: point cargo at CLM_LAND_DIR in your check
`clm land` no longer sets `CARGO_TARGET_DIR` or `CARGO_INCREMENTAL` for `[land] check`. It hands the check `CLM_LAND_DIR` instead, the clone's build place that lasts from land to land, while `clm land --check-only` hands none, so a build's check compiles in its own worktree.

If your check script runs cargo, export `CARGO_TARGET_DIR` from `CLM_LAND_DIR` and `CARGO_INCREMENTAL=0` before its first cargo command, when `CLM_LAND_DIR` is set and non-empty. These are the lines `clm init` writes for it:

```
if [ -n "${CLM_LAND_DIR:-}" ]; then
    export CARGO_TARGET_DIR="$CLM_LAND_DIR" CARGO_INCREMENTAL=0
fi
```

Without them a land's cargo build goes to the land tree's own `target/`, which the land cleans before every check, so every land compiles from scratch.

If your check keeps its build output elsewhere on purpose, decline this note with `clm upgrade --decline 20261109-land-still-sets-cargo`.
