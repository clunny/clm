---
title: "Specs restate their own requirements in Design and still narrate past changes in requirement text"
kind: idea
components: [check.coverage, ci, capability, plugin, vcs, gate.fingerprint, orchestrator.checkpoint, plugin.hooks.guard, plugin.panel, sync.regenerate, view, view.repo, web.client-live, web.worker.platform]
---

`clm health`'s spec-slimmer pass on 2026-11-09 found most flagged components' Design and Contract repeating their requirements nearly word for word, many requirements carrying several "a skill that doesn't mention X" scenarios that prove one thing, and some requirements that are source greps or one-time migration instructions. Change `20261109-strip-history-wording` fixed only the Contract and Design wording that `health/history-words` flags; requirement text that narrates was left because rewording it changes its version:

- `check.coverage/owner`: "as it does now".
- `ci/nextest-runner`: its second sentence, how the build that moved the script to `nextest` was to proceed, which has happened.

The larger cuts each need judging per component, for example `orchestrator.checkpoint/outcome-name` (records a removed feature), `gate.fingerprint`'s `adds_change` with no non-test caller, `web.client-live`'s unrequired subscription-renewal code, `sync.regenerate`'s scenarios worded as `clm rebase` runs, the source of its workaround. Re-run `/clm:health` to regenerate the proposals before designing.
