---
status: applied
resolves: []
---

## Intent

Report a CLM defect found while drafting `platform-fake-dead-helpers`: this change only adds `backlog/a-draft-s-clm-setup-is-denied-by-claude-code-s-auto-mode.md`.

`platform-fake-dead-helpers` is code-only deferred work in `web.worker.platform` (removing `write_checkout`, `version_of`, `citing` and `FakeRepo`'s `compare_status` and `ref_times` from `crates/clm-web-platform/src/fake.rs`, with the `fake` feature's `tempfile` dependency), so it goes by component with `clm claim web.worker.platform --fix` (`deferred-work-resolves-in-code`). The claim was taken, but Claude Code's auto-mode classifier denied `clm setup` on the fix branch, so no test result there could be trusted: the item is left (`setup failed`), stays open in the backlog, and its claim was released.

## The human's answers

- backlog/platform-fake-dead-helpers: "Draft now (Recommended)" — the helpers write_checkout, version_of, citing and fields compare_status, ref_times were confirmed unused outside crates/clm-web-platform/src/fake.rs on origin/main.
