---
title: "The guard is its own crate, its shell reading its own node"
date: 2026-10-31
status: accepted
---

`plugin.hooks` measured spec 17,538 of 12,000 and code 77,979 of 40,000 tokens. About half the code count was `crates/clm/tests/plugin.rs`, 160 KB of plugin tests that counted toward `plugin.hooks` whole because a thousand lines of hook tests in it cite the node (`tests-count-where-cited`). The rest was the guard: 68 KB of `crates/clm-hooks/src/guard/` and most of the crate's tests, beside 11 KB of session start, autosync and `clm defect`.

The human chose to move the hook tests out of `plugin.rs` and to split twice, with the guard in its own crate:

- **`plugin.hooks.guard`, crate `clm-guard`**: the rules (`push-guard`, `release-guard`, `approve-guard`, `defect-approval`) and the queries they make. The guard changes for different reasons than the session-start text or `clm defect`, and nothing of `clm-hooks` but `settings` is used by it.
- **`plugin.hooks.guard.shell`, the crate's `shell` module**: how a command is read as the shell would read it (quotes, heredocs, here-strings, `-c` scripts, quoted substitutions, the command position) and each piece's checkout. The guard's walk over a command already decides nothing: it yields each piece with its text, tokens and checkout, and the rules judge them. That is the seam, so the checkout rules move out of `push-guard` into a requirement of the shell's own (`guard-checkout`), which `push-guard` reads, and `release-guard` through it.

The shell stays a module of `clm-guard` rather than a crate: it has one user, and the rest of the guard is its only reason to exist. Its `paths` (`crates/clm-guard/src/shell/**`, `crates/clm-guard/tests/shell.rs`) are disjoint from the guard's, which list the crate's top-level files.

`clm-guard` reads settings through `clm-hooks`' `settings(cwd)`, so the child depends on its parent and not the other way: the hooks, the guard and `clm defect` keep one way to find the repository and load its settings (`reuse-first`).

Rejected:
- **Raising the budget.** The guard and the shell reading are seams by the guide's tests: different reasons to change, and one already a separate pass that decides nothing.
- **Moving only the tests.** It brings the code count near 40,000 but leaves the spec at 17,500.
- **Two crates, `clm-guard` and a shell crate.** A crate boundary buys nothing a module and disjoint `paths` don't, for a reader with one user.
