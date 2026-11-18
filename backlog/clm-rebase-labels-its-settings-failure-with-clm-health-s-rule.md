---
title: "clm rebase labels its settings failure with clm health's rule"
kind: idea
components: [clm, sync]
---

`20261118-settings-failure-names-its-rule` settled that a violation line carries the ID of the requirement that states the exit it reports (`component-words`), and moved `clm affected`'s settings-failure label from `check.affected/affected-all` to `check.affected/affected-base`.

`clm rebase`'s settings failure still reads `<source>: check.health/health-report: <error>` (`crates/clm/src/main.rs`, `sync_settings`, and the `health`/`workaround` dispatch beside it). `check.health/health-report` states what `clm health` does when its settings can't be loaded, not what `clm rebase` does: the rule that states `clm rebase`'s line is the root's `clm/rebase-settings`, which itself names `check.health/health-report` as the label. So the label is written into the spec, and the code follows it.

The question for a spec change: should `clm/rebase-settings` (and `sync/judged-by-check`, which reads the line) label the failure with the rule that states the rebase's exit, as `clm affected` and `clm size` now do, or is `check.health/health-report` the right label because the failing settings are the health settings? Found by the code review of the `clm/routes-only` build.
