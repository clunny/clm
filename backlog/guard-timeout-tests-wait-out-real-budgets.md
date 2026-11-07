---
title: "The guard's timeout tests wait out its real 20- and 30-second budgets, adding about 30 seconds to every land that runs them"
kind: idea
priority: P3
components: [plugin.hooks.guard]
origin: 20261107-faster-land-check
---

## Intent

`crates/clm/tests/guard.rs` took 30 of the 137 seconds of a full `cargo test` run measured on 2026-11-07. Three tests (`guard_answers_a_hanging_lookup_within_its_budget`, `guard_blocks_a_push_git_does_not_answer`, `guard_push_queries_share_one_budget`) hang a fake `gh` or `git` and wait for the guard's fixed budgets to run out: `PUSH_BUDGET` (20 s, `crates/clm-guard/src/push.rs`) and `LOOKUP_BUDGET` (30 s, `crates/clm-guard/src/release.rs`). The scenarios of `plugin.hooks.guard/push-guard` and `release-guard` say the hook answers within those times, so the tests can only prove them through the binary in real time.

## Settled

- The budgets stay as the rules state them for the hook. They become parameters of `clm-guard`'s library calls: the timeout scenarios are proven by calling the library with a budget of about a second and a hanging fake, and a fast test checks that the hook passes 20 and 30 seconds. No variable or setting may shorten a safety hook's budget in production. Chosen by the human on 2026-11-07 while designing `20261107-faster-land-check`.
- Waits until `20261107-the-push-guard-blocks` lands, since that change modifies `push-guard`, whose scenarios this one rewrites.

## Open questions

- Does `crates/clm-land/tests/check_lock.rs` (6.4 s, scenarios with 2-second sleeps) deserve the same treatment, or are its waits the proof?
