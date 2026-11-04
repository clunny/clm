---
status: done
resolves:
  - spec-paths-vs-clm-toml-approval
---

# A clm.toml approval change counts as a spec path

## Intent

A branch that changes whose approval counts, `clm.toml`'s `approval`, needs review (`approval-setting`, `humans-review-intent`), so it should land as a reviewed `spec/*` branch. Today `clm land` refuses it there: `gate/files-outside-spec-paths` lists `clm.toml`, which isn't a spec path, and `land/spec-branch-holds-spec-paths` refuses a `spec/*` branch with any such file. This makes `files-outside-spec-paths` leave `clm.toml` out exactly when `approval-setting` counts it as a spec change, so the `land.approval/approval-from-base` scenarios hold as written for `spec/x`, and their tests go back from `propose/x` to `spec/x`.

## The human's answers

> Fix the gate (Recommended): `files-outside-spec-paths` leaves `clm.toml` off its list when `approval-setting` counts it as spec, so an approval change can land on a reviewed `spec/*` branch. The `land.approval` scenarios keep `spec/x`.

## Problem

`approval-setting` makes an `approval` change to `clm.toml` a spec change that needs review, but says the file doesn't become a spec path, and `files-outside-spec-paths` reads only the spec-path list. So the one kind of `clm.toml` edit that must be reviewed can't take the reviewed branch: `clm land spec/x` refuses it before checking the approval. The build of `approval-from-base` renamed its fixtures to `propose/x` to keep its tests passing, so they no longer prove the scenarios as written.

## Approaches

- **Leave the approval change off in `files-outside-spec-paths` (chosen, the human's answer).** The call already reads both sides through git; `approval_changed` is the one decision the gate and `web.worker` already share, so reusing it adds no new rule (`reuse-first`). Every other `clm.toml` edit still counts as a file outside the spec paths, so code can't ride on it.
- **Make `clm.toml` a spec path.** Rejected: every edit to it, including `[land] check`, which names a command run on the lander's machine, would land on checks as spec, against `spec-branches-hold-spec-paths`.
- **Reword the `land.approval` scenarios to a non-`spec/` name.** Rejected by the human: it leaves the reviewed change with no reviewed branch to land on.

## Outcome

- `gate/files-outside-spec-paths` leaves out `clm.toml` when `approval_changed` of its two sides is true, with scenarios for an approval change and for another setting.
- `gate/approval-setting` says that `files-outside-spec-paths` is the one reader that leaves it off, where it said every reader stays as it is.
- `land.approval/approval-from-base` needs `gate/files-outside-spec-paths`; its text is unchanged, and its tests' fixtures go back to `spec/x` (`crates/clm-land/tests/approval.rs`).

Edge cases the change knows: an unparseable `clm.toml` counts as changed under `approval-setting`, so it is left out too, and the branch needs review, which fails closed as the gate does. A `spec/*` branch changing both the `approval` key and another key is left out as well; the review covers the whole file.

## Open questions

None.
