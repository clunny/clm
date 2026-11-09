---
title: "Timing-bound tests fail when several builds' land checks run at once"
kind: bug
components: [land]
---

Step: `clm land --check-only` and `clm land`, run by unattended `clm run` builds launched in parallel by `/clm:build` (up to `max_builds` at once on one machine).
What happened: with several land checks running at the same time (host load average about 81), Rust tests with time budgets failed in crates none of the builds touched: `clm-git` a_fetch_that_loses_a_ref_race_goes_on, `clm-guard` queries_and_lookups_that_outlast_their_budget_block, `clm-runner` scenario_stopped_by_a_signal and `clm-land` step_times two_steps. Each passed when its check was rerun alone. Builds lost whole check runs to them, and one build saw different failures on each of three runs.
Expected: a land's check fails only when the merged tree is broken. Either the timing-bound tests tolerate a loaded host, or concurrent land checks on one clone are serialised (or limited separately from `max_builds`), so parallel builds don't make each other's checks fail.
Rule involved: land-locally (every land runs the full check on this machine) together with builds-in-subagents and build-places (several builds land concurrently); a flaky check makes landing depend on retries.
