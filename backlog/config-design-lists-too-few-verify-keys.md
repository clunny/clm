---
title: "Config's Design lists too few of this repository's verify keys"
kind: bug
components: [config]
---

`specs/config/SPEC.md`'s Design paragraph on what this repository's `clm.toml` sets gives `[verify]` only `check`, `mode`, `triggers` and `every`, while `config/verify-here` also requires `mutate`, `mutate_jobs = 6` and `mutate_minutes = 60`. The Design and the requirement disagree; the Design text needs a spec branch to bring it in line (found by the code review of `impl/config`'s `verify-here` build).

Also stale: `backlog/a-passing-check-proves-less-than-the-spec-claims.md` still says the mutation-pass change waits on `config/verify-here`, which `impl/config` built and whose change it archived.
