---
title: "Code comments, a test and three skills still cite decisions that are deleted"
kind: deferred
components: [plugin, land.approval, plugin.hooks.guard, ledger, gate, land, sync, clm]
origin: 20261113-phase-2-delete
---

`20261113-phase-2-delete` deleted the superseded decisions and repointed their citations in the specs and the other decisions. A spec branch can't edit code, so these remain, each to cite the decision that holds the rule now:

- `approve-in-session` → `approval-policy`: `plugin/skills/build/SKILL.md`, `plugin/skills/propose/SKILL.md`, `plugin/skills/triage/SKILL.md`, `crates/clm-approval/src/approve.rs`, `crates/clm-guard/src/lib.rs`.
- `pinned-approval` → `approve-pins-the-review`: `crates/clm-ledger/src/record.rs`.
- `light-ci-for-doc-prs` → `land-locally`: `crates/clm-ledger/src/doc_only.rs`.
- `merge-tested-tree` → `land-locally`: `crates/clm-gate/src/lib.rs`.
- `forced-fixes-cover-additions` → `coupled-changes-sequence`: the paragraph of `plugin/skills/propose/SKILL.md` that still teaches the withdrawn rule; the tests in `crates/clm/tests/plugin.rs` that require the name go with `plugin/builder-check`'s obsolete markers.
- `regenerate-applied-live-changes`, which never named a decision → `regenerate-applied-specs`: `crates/clm-land/src/derived.rs`, `crates/clm-sync/src/lib.rs`.
- The prompt `clm run` writes names `capability-tests-follow-rule` and `template-migration-fix`, and the prompt `clm draft` writes names `draft-backlog-only-change` and `draft-code-fix-written-rule`, as requirement names. Those requirements are deleted; the sentences stay, without the names.

Each component's next build can take its lines; none changes what a requirement states.
