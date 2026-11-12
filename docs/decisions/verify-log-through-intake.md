---
title: "clm intake points at a red run's kept log; the ledger carries no test output"
date: 2026-11-12
status: accepted
---

Change `20261112-a-failed-local-verify`. Extends `verify-after-land` and `verify-skill-reruns-advise`.

A failed `local` run of `[verify] check` already keeps its whole output in `.clm/verify-logs/` of the clone's main worktree, and prints the failing step and last lines once, to whoever ran it. A triage session that later puts the `verify <sha> new` line to the human had no way to find that log, so the human decided whether the failure was real on the culprit alone.

**Chosen.** `clm-verify` offers `kept_log(root, commit)`, which owns where a kept log lives and matches it by prefix of the full commit ID, and `clm intake` prints `verify <sha> log <path>` after a `new` line when this clone holds one. The triage skill reads that file and shows the human its last 40 lines and the open step, as `/clm:verify` shows a run it made (`reuse-first`: the log and its markers already exist).

**Considered and rejected.**
- *The `verified` record carries the failing step and last lines.* It would reach every clone, but the ledger is shared, signed and append-only: every red record would push 40 lines of a project's test output, whatever they hold, to `origin` for good. A `local` run's reader is nearly always the clone that ran it.
- *The record carries the log's path.* A path on one machine means nothing to another clone, and the file belongs to the clone, not the ledger.
- *The triage skill builds the path itself.* It would copy verify's layout into a skill and break when the repository's abbreviations grow longer than the name the run gave its log (`path-ownership`).
- *A command of its own, such as `clm verify --log <sha>`.* `clm intake` already lists the red result; a second command would only make the session ask twice.

A red result from another clone or from the host's pipeline gets no `log` line, and the skill says so; carrying output across clones waits until a project needs it (`yagni`).
