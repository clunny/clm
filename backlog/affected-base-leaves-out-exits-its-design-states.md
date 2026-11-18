---
title: "`affected-base` leaves out two outcomes that `check.affected`'s Contract and Design state"
kind: question
components: [check.affected]
---

`check.affected/affected-base` (at `e2cc9038`) states the command's exits of 1, but two outcomes that the Contract and Design state, and that tests in `crates/clm-affected/tests/command.rs` prove, are stated by no requirement:

- **Exit 2 when git fails otherwise.** The Contract says "2 when git fails otherwise", and `affected-overlap` says "any other git failure SHALL exit 2, as `affected-base` says for `--base`", but `affected-base` doesn't say it. `a_git_failure_exits_2` used to cite `affected-base`; the spec-auditor found that the text doesn't state it, so its marker was taken off on `impl/check.affected` (`code-fix-needs-a-written-rule`).
- **`--readers` loads no graph.** The Design says `--readers` "doesn't read `[affected] all` or the owners at all", and `affected-base` now exempts `--readers` from the settings, but its "1 with the error when the graph can't be loaded" has no such exception, so read literally it requires `--readers` to fail on a graph cycle. `readers_alone_ignore_the_graph_and_the_settings` proves the Design's behaviour and cites nothing.

Should a spec change add both clauses to `affected-base` (exit 2 for any other git failure; no graph read with `--readers`), so the two tests can cite it?
