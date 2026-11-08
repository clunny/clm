---
title: "Projects upgrading past the lifted pre-land check lock aren't told their tests must now run side by side"
kind: bug
priority: P2
components: [upgrade]
---

## Problem

`20261107-builds-pre-land-checks` has a build's `clm land --check-only` (run in a build place, `CLM_SLOT` not 0) stop taking the clone's check lock (`land/check-lock`). Until now that lock was what kept a project's checks from overlapping when parallel builds ran them: it was added after one project's fixed-port browser tests failed whenever two checks overlapped (`check-runs-one-at-a-time`). Once the lock is lifted, a project whose tests still bind fixed ports or paths starts failing intermittently between parallel builds, for reasons unrelated to any branch.

`tests-run-concurrently` and `CLM_SLOT` (`20261107-parallel-builds-test-runs`) state the rule and give builds the means, but an existing project learns of the rule only from `clm guide constraints` and its builds' context bundles. That change left an upgrade migration out "until a project trips on it" (`yagni`). Lifting the lock is that moment: a project upgrading `clm` past it changes behaviour with nothing telling it so.

## Wanted

`clm upgrade` (`/clm:upgrade`) offers projects set up before the lift an advised note migration (`upgrades-carry-migrations`, `upgrade-note-and-decline`). It says that builds' pre-land checks now run at the same time, that the project's tests must run side by side on one host (`tests-run-concurrently`), keying the ports and paths they must know in advance on `CLM_SLOT`, and how to check its own tests for fixed ports, paths and shared servers. It should also say what to do meanwhile: replace the constraint if the project keeps runs apart another way. Whether the migration should also offer anything stronger (for example, filing the fixed ports it finds as bugs, as `/clm:init` does for existing code under `plugin/init-skill-concurrent-tests`) is for the design.

Wanted before the next release, which will ship the lift.
