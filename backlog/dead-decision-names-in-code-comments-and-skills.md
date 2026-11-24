---
title: "Code comments, tests, skills and prompts still cite decisions and requirements that are deleted"
kind: deferred
components: [plugin.hooks.guard, ledger, orchestrator.triage, clm]
origin: 20261113-phase-2-delete
---

`20261113-phase-2-delete` deleted the superseded decisions and repointed their citations in the specs and the other decisions. A spec branch can't edit code, so these remain, each to cite the decision that holds the rule now:

- `approve-in-session` → `approval-policy`: `crates/clm-guard/src/lib.rs`.
- `pinned-approval` → `approve-pins-the-review`: `crates/clm-ledger/src/record.rs`.
- `light-ci-for-doc-prs` → `land-locally`: `crates/clm-ledger/src/doc_only.rs`.
- The prompt `clm run` writes names `capability-tests-follow-rule` and `template-migration-fix`, and the prompt `clm draft` writes names `draft-backlog-only-change` and `draft-code-fix-written-rule` (`crates/clm-triage/src/draft.rs`, `orchestrator.triage`'s), as requirement names; `crates/clm-run/src/lib.rs` also names `background-work-stopped`, `prompt-doc-readers` and `prompt-recorded-check` in comments. Those requirements are deleted; the sentences stay, without the names.

The skills' lines are `plugin`'s, split into `backlog/dead-decision-names-in-the-plugin-s-skills` so its build can resolve them with a trailer. Each component's next build can take its lines; none changes what a requirement states. The lines of `gate` (`crates/clm-gate/src/lib.rs`), `land` (`crates/clm-land/src/derived.rs`) and `land.approval` (`crates/clm-approval/src/approve.rs`) are mended by their builds of `20261121-wave-3-land-by-the-rule`, and `sync`'s (`regenerate-applied-live-changes` → `regenerate-applied-specs` in `crates/clm-sync/src/lib.rs`) by its build of `20261121-wave-3-sync-by-the-rule`.
