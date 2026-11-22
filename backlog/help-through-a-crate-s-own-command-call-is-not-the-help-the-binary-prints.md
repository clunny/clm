---
title: "Help through a crate's own command call is not the help the binary prints"
kind: deferred
components: [config, orchestrator.checkpoint, land.approval]
origin: 20261120-wave-2-rest-by-the-rule
---

**The problem.** A crate that holds its own command (`commands-live-in-their-crates`) parses the command's arguments with a clap struct of its own. Given `--help`, several of these calls print as their first line the struct's code comment, such as "The command line `command` parses: the command's own arguments, without its name", where `clm <command> --help` prints the command's description. A subcommand's `Usage:` line can name only the subcommand (`Usage: record …`) where the binary prints `Usage: clm effort record …`. No user sees it today, since the binary answers `--help` before the call is reached; a test that calls the crate, and any caller that is not the binary, does.

**What was measured** (2026-11-21, `main` at `a98783b5`, each call run from a scratch crate and compared with the binary): of the nine crates of phase 5's wave 2, `clm-unreferenced`, `clm-health`, `clm-effort`, `clm-ledger`, `clm-init` and `clm-affected` differed, and `clm-check` matched. Wave 2 states the rule in each of those components' `commands-in-crate` (the call writes the help `clm` prints for the command, line for line) and their builds mend them.

**What is left.** The crates outside that wave were not measured. `clm-config`, `clm-checkpoint` and `clm-approval` were read as holding the same private struct with a doc comment; the others that hold a command (`apply`, `rename`, `gate`, `land`, `sync`, `verify`, `host`, `orchestrator` and its children, `view`, `view.repo`, `machine`, `agent-harness`) are unread. Each component's pass in waves 3, 4 and 7 measures its own calls and states the same clause; wave 1's components (`config`, `apply`, `rename`, `machine`, `agent-harness`, `gate.fingerprint`) have had their pass and need this item.

**Deferred** (2026-11-21, the human): each of waves 3 to 7 measures and states the help of its own crates' calls in its pass. The six components of wave 1 that have had their pass (`config`, `apply`, `rename`, `machine`, `agent-harness`, `gate.fingerprint`) are taken in wave 7, on one small branch beside `backlog/clm-archive-names-a-change-s-status-as-rust-prints-it-not-as-change-md-writes-it` (`backlog/a-passing-check-proves-less-than-the-spec-claims.md`, wave 2's block). This item is deleted when the last of them is built. Until then, `clm triage` lists it as `deferred` and the build loop drafts nothing for it.
