---
title: "Two of effort's scenarios still name the clm binary its tests may no longer run"
kind: question
components: [effort]
origin: 20261115-commands-in-their-crates
choices:
  - "Reword both to name clm-effort's call"
  - "Leave them: commands-in-crate overrides"
---

`effort/commands-in-crate` moved effort's 20 tests out of `crates/clm/tests/effort.rs` into `crates/clm-effort/tests/command.rs`, where they call `clm-effort`'s calls and never run the binary. Two scenarios still say the binary runs:

- `record-command`, scenario "the command": "WHEN the `clm` binary runs `effort record --transcripts <dir> --branch impl/a`".
- `architecture-list`'s help scenario names `clm architecture --help`; its test now reads the call's help, which `crates/clm`'s parser builds from the same `ArchitectureArgs`.

The root's Design (`commands-live-in-their-crates`) and `commands-in-crate` say these tests call the crate, so the tests follow it; the scenarios' wording lags. Also found by the same audit: `capabilities_refused_outside_a_clm_repository` cites `capability-page`, whose text states exit 1 for an unknown slug but not the refusal outside a CLM repository, which only the Contract states. What is needed: a spec change rewording those scenarios to the call, and a requirement (or `capabilities-list` wording) that states the refusal.

## The human's answer (2026-11-15)

The second choice: the scenarios stay as they are. They state what a person sees; `clm-effort`'s call proves them and `clm/routes-only` proves the binary adds nothing. The root's Design now says so for every component. The gap this item found, that `clm capabilities` refuses outside a CLM repository by the Contract alone, gets its requirement in the batched spec change that follows the build run, which then deletes this item.
