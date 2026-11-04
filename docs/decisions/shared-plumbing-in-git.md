---
title: "The remote's name, the clock and the refused-or-git-failed error live in clm-git"
date: 2026-10-27
status: accepted
---

The crates split out of `clm-orchestrator` each copied three small pieces besides the git wrapper `git-plumbing-layer` removed: `const REMOTE: &str = "origin"` (`clm-orchestrator`, `clm-sync`), the time in Unix seconds from `SystemTime` (`clm-orchestrator`'s `repo::now()`, `clm-view`), and a two-variant `Error { Refused, Git }` with `From<clm_git::Error>` (`clm-sync`, `clm-watch`), each with its own arm in `crates/clm/src/main.rs` that prints the message and exits 1 or 2. Two spellings of the remote can drift apart, so a command could read tracking refs of one remote and push to another, and every copied arm is one more place an exit code can be wrong.

`clm-git` already sits under every crate that shells out and depends on nothing in the workspace, so it owns them: `REMOTE`, `now()` and `Failure { Refused, Git }` with `exit_code()`. A crate whose errors are exactly those two, built only from `clm-git`'s own error, uses `Failure` as its `Error` (re-exported under that name, so callers don't change), and `crates/clm` has one helper, `failed`, that prints and exits for all of them.

A crate keeps its own type when it needs a third kind of error (`orchestrator`'s `Spec`, `host`'s `Nowhere`), or converts other crates' errors with `?`: `clm-land`'s two-variant `Error` also implements `From` for `clm_ledger::Error`, `clm_config::Error` and `std::io::Error`, and Rust's orphan rule forbids those impls on a type `clm-land` doesn't own, so adopting `Failure` there would turn every such `?` into a `map_err`. Forcing either case into the shared type would cost its users more than the copy it removes.

Rejected:
- **A new shared crate for the non-git pieces.** A crate holding a constant, a clock and an enum is more structure than the pieces justify, and every crate that would use it already depends on `clm-git`.
- **A generic error with an open set of kinds.** It would let `orchestrator` and `host` share it too, but their extra variants carry data (`Vec<Violation>`, an unfiled report) that `main.rs` handles specially, so the mapping wouldn't become one anyway.
- **Conversions from `io::Error` and other crates' errors in `clm-git`.** `clm-git` depends on nothing in the workspace and must stay that way, and an `io::Error` isn't a git failure.
- **Leaving the copies.** Cheap today, but each new crate split off copies them again, as `clm-watch` did in its build.

`orchestrator` re-exports `REMOTE` from `clm-git`, so `orchestrator.run`, `orchestrator.checkpoint` and `orchestrator.triage`, which import it from there, don't change. The crates that still spell `"origin"` in their git calls (`land`, `ledger`, `host`, `init`, `view`) and `view`'s copy of the clock move over in their own nodes' changes (`backlog/remote-name-from-clm-git-everywhere.md`).
