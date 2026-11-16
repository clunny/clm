---
title: "Two defect tests run clm check on the item they wrote"
kind: question
components: [plugin.hooks, clm, check]
blocks: [clm/routes-only]
choices:
  - "Root tests, as for init's eleven"
  - "Reword the scenarios"
---

`plugin.hooks/commands-in-crate` moved 22 of the 24 tests `changes/20261115-commands-in-their-crates/moved-tests.txt` lists for `plugin.hooks` into `crates/clm-hooks/tests/` (`defect.rs` 15 of 17, `hook_commands.rs` 7 of 7), each now calling `clm-hooks`' call of the command. Two stay in `crates/clm/tests/defect.rs`, because their scenario runs another component's command on what `clm defect` wrote, which `clm-hooks` (`depends_on` holds no `check`) can neither call nor run as a binary:

- `backlog_on_a_spec_branch` (`plugin.hooks/defect-backlog`, scenario "on a spec branch": "`clm check` accepts it");
- `backlog_node_a_live_change_creates` (`plugin.hooks/defect-backlog`, scenario "node a live change creates": "`clm check` accepts the item").

What is needed: a spec change saying where those two scenarios' `clm check` halves are proven once `clm/routes-only` holds, as `backlog/seventeen-init-tests-can-t-move-into-its-crate.md` was answered for `init` (a scenario that runs another component's command on what this one made is the root's), after which `plugin.hooks`' next build moves the rest of each test.
