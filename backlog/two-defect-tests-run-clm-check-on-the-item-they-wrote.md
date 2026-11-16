---
title: "Two defect tests run clm check on the item they wrote"
kind: deferred
components: [plugin.hooks, clm, check]
blocks: [clm/routes-only]
---

`plugin.hooks/commands-in-crate` moved 22 of the 24 tests `changes/20261115-commands-in-their-crates/moved-tests.txt` lists for `plugin.hooks` into `crates/clm-hooks/tests/`: `defect.rs` 15 of 17, `hook_commands.rs` 7 of 7. Each moved test now calls `clm-hooks`' call of its command. Two stay in `crates/clm/tests/defect.rs`. Their scenario runs another component's command on what `clm defect` wrote, and `clm-hooks` can neither call that command (`check` is not in its `depends_on`) nor run the `clm` binary:

- `backlog_on_a_spec_branch` (`plugin.hooks/defect-backlog`, scenario "on a spec branch": "`clm check` accepts it");
- `backlog_node_a_live_change_creates` (`plugin.hooks/defect-backlog`, scenario "node a live change creates": "`clm check` accepts the item").

Such a scenario is the root's to prove (`commands-live-in-their-crates`). This item is due with the spec change that follows the build run, which moves these scenarios into requirements of the root.
