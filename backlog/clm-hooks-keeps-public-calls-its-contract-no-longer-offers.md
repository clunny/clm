---
title: "clm-hooks keeps public calls its Contract no longer offers, and a comment names a requirement that doesn't exist"
kind: deferred
components: [plugin.hooks]
origin: 20261123-wave-5-hooks-by-the-rule
---

Wave 5 (`20261123-wave-5-hooks-by-the-rule`) makes `plugin.hooks`' Contract offer only the three command calls, `DefectArgs`, `HookArgs` and `clm_root`. "It offers no other public call." Five public functions of `crates/clm-hooks/src/lib.rs` stay outside it:
- `autosync(event)` and `settings(cwd)`, which nothing calls, tests included;
- `session_start(event)`, `session_start_hook(agent_harness, event)` and `autosync_hook(agent_harness, event)`, which only the crate's tests call. No requirement asks for them as calls, and the command calls do the same through their private helpers.

The `toml` dev-dependency of `crates/clm-hooks/Cargo.toml` is used only by `tests/git_layer.rs`, which the same change deletes.

`clm unreferenced` won't flag any of them, since their names occur in other crates. Separately, the comment at `crates/clm-hooks/src/defect.rs` line 42 names `model/backlog-nodes`, a requirement that doesn't exist; the requirement is `model/backlog-components`.

In plugin.hooks' build of that change, do three things (`no-dead-code`):
- Remove `autosync`, `settings` and `session_start`, and make `session_start_hook` and `autosync_hook` private (or fold them into the private helpers).
- Remove the `toml` dev-dependency.
- Correct the comment.

Move every test that calls `session_start`, `session_start_hook` or `autosync_hook` onto `session_start_command` or `autosync_command`, passing `--agent-harness codex` where it tests Codex. This covers `tests/hooks.rs`, `tests/agent_harness.rs` and `the_paragraph_says_component`; re-cite them, never delete them. The root's `hook-agent-harness-flag` names `session_start_hook` and `autosync_hook` until wave 7 rewords it, but no code outside the crate calls them. No requirement states any of the three calls, so no yes is needed. Resolve this item in that commit with the trailer `Resolves: backlog/clm-hooks-keeps-public-calls-its-contract-no-longer-offers`.
