---
title: "The guard's spec still describes it as it was before `clm hook guard` became a call of `clm-guard`"
kind: deferred
components: [plugin.hooks.guard, plugin.hooks]
---

The build of `plugin.hooks.guard/commands-in-crate` left three places where `specs/plugin/hooks/guard/SPEC.md` no longer matches the code, each needing a spec edit:

1. **No repository root.** The requirement and the Contract say the call takes "the repository root `crates/clm` found from the current directory, which the command's `--root` overrides". `clm hook guard` has no `--root`, and the guard places each command by its event's `cwd`; finding the root as `crates/clm` does (`repo_root`) changes the process's directory, which would move the fallback for an event with no `cwd`. So `clm_guard::guard_command` takes no root. The requirement's and the Contract's wording should drop it for this command.
2. **Settings.** The Design says the guard loads the `defects` setting through `clm-hooks`' `settings(cwd)`. The call loads them under its own environment lookup, so the guard now calls `clm_config::load(cwd.and_then(clm_hooks::clm_root), env)`, the same root search with the call's lookup. Once `plugin.hooks` builds its own `commands-in-crate`, `clm-hooks` could offer `settings` with a lookup that both use, so the hooks and `clm defect` load settings one way again.
3. **Where the binary scenarios are tested.** The Design says the scenarios that run the built binary (a fake `gh` on the `PATH`, the defect ask) are tested in `crates/clm/tests/guard.rs`. They are now in `crates/clm-guard/tests/guard.rs`, calling the crate, and set `PATH` on their own process as the root's Design says.

Due with the spec change that follows the `commands-in-crate` build run.
