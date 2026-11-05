---
status: applied
resolves: [two-init-requirements-name-entries-clm-init-no-longer-writes, workaround-init-template-says-pull-request-1, workaround-init-templates-say-new-words-1]
---

# Two init requirements stop naming entries clm init no longer writes

## Intent

Since `architecture-template-proof` dropped the `## Enforced by clm` list (`carried-constraints`), the `ARCHITECTURE.md` that `clm init` writes holds no entry of CLM's own. Two `init` requirements still name lines of that list: `template-says-pull-request` asks for a `git-is-truth` line holding `a pull request's URL`, and `templates-say-new-words` asks for a `humans-review-intent` line naming `human_burdens`. No build can make either true. This change drops those parts and keeps what still holds: no template calls a pull request a page, the root `SPEC.md` is written `status: unbuilt`, and no template holds a word `plain-terms-renames` replaced. The rewording settles the two `workaround` flags a build raised against these requirements, which this change deletes.

## Problem

- `init/template-says-pull-request` and `init/templates-say-new-words` state behaviour that `init/architecture-template-proof` now forbids, so the spec contradicts itself, and their tests assert only the remainder.

## Approaches

- **A. Remove both requirements.** The word guards they keep still catch a template edit that brings an old word back; removing them would lose that for no saving.
- **B. Reword both to what still holds (chosen).** Each keeps its template-wide guard and loses the clause about a line `clm init` no longer writes. The existing tests already assert that remainder, so the build only moves their markers to the new versions.

## Outcome

- `template-says-pull-request`: no file under `crates/clm-init/templates/` holds `page's`.
- `templates-say-new-words`: the root `SPEC.md` is `status: unbuilt` and no template holds a replaced word.

## Constraints

- `carried-constraints`: the written `ARCHITECTURE.md` holds no statement of a carried constraint, so no requirement may ask for one.
- `plain-terms`: both requirements still serve it.

## Edge cases and workarounds

None.
