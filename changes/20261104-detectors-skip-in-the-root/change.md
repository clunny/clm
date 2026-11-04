---
status: proposed
---

# Detectors skip in the root, and the recheck's other fixes

## Intent

A recheck of the three changes from issues #67, #64, #61 and #23 against `ARCHITECTURE.md` found one design that breaks `path-ownership` and three gaps. `20261104-detector-requires-manifest` added `requires` to `clm-check`'s `Detector`, which the root builds by literal, so `check`'s build would have had to edit the root's code for an addition, which `path-ownership` allows only for a removal. This change moves the skip into the root, which builds the detector list (`clm/detector-requires-skip`), and leaves `check` only the relative-path lookup (`check/detector-relative-command`), so no build reaches into another component. It also has a rename's adding build mark the old name `clm:remove-by` the change (`expand-contract`), documents `requires` in the guide's settings table (`method-in-the-plugin`), and traces `config/detector-requires-setting` to `capability/adopt-clm`, which the skip serves.

## Problem

- `path-ownership`: `crates/clm/src/main.rs` builds `clm_check::Detector { name, command, hint, timeout }`. A new field breaks that line on `check`'s build.
- `expand-contract`: the old path of a breaking contract change is marked `clm:remove-by <change-id>`; `rename-adds-then-removes` didn't say so.
- `method-in-the-plugin`: an adopter writing a detector learns its keys from the guide, whose settings table didn't list them.
- `traced-rules`: the requirement served `no-dead-weight`, but skipping a detector until its manifest exists is what lets a new repository's bootstrap pass.

## Approaches

- **Skip in the root (chosen).** The root already maps the settings' detectors onto `clm-check`'s; dropping the ones whose file is missing is a filter there, inside its own code. `check` keeps a plain list.
- **Add the field to `clm-check` and allow compile-only fixes for additions.** Rejected here: it needs `path-ownership` changed, which is a reviewed edit of `ARCHITECTURE.md`; the gap is reported as a backlog item.
- **Expand-contract on `clm-check`'s API** (a second function taking the new field). Rejected: two entry points for one list, to avoid a filter the caller can do.

## Outcome

- `check/detector-requires` and `clm/check-passes-requires`, neither built, are replaced by `check/detector-relative-command` and `clm/detector-requires-skip`; `init/init-without-manifest` needs both.
- `plugin/builder-check` and `orchestrator.triage/draft-contract-step-deferred` say the adding build marks the old name `clm:remove-by` the change.

## Constraints

- No requirement it removes has a citing test.
