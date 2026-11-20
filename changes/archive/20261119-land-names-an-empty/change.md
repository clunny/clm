---
status: done
resolves: [land-names-an-empty-owner-for-a-file-outside-the-clm-root]
---

## Intent

`clm land` refuses a build branch that changes a file outside the CLM root with a line that says the file is outside the root, which no build of the root's components may change, instead of one naming an empty owner and telling the session to build the file in that empty component.

## Problem

Since `gate/paths-under-the-root`, `clm_gate::foreign_files` returns a file a build changes outside the CLM root, named `../<path>`, with an empty owner. `land/build-holds-own-files` states one message for every foreign file, built around its owner, and `crates/clm-land/src/own_files.rs` formats it so: ``../b: land/build-holds-own-files: owned by ``; impl/a changes its own files, ... Build this in ``, or leave ...``. The refusal is right; the message sends the session to a component that doesn't exist. No requirement states another message, so the fix needs one (`code-fix-needs-a-written-rule`).

## Approaches

- **Taken: a second line for an empty owner.** `build-holds-own-files` states the line for a file outside the root: it names no owner and offers only the route that fits, leaving the requirement unbuilt behind a question item, since no component of the root can build a file outside it. One branch on the owner where the line is formatted; `foreign_files`' contract stays as it is.
- **Not taken: give `foreign_files` a distinct marker (an `Option` owner or an enum) for an outside file.** It changes `gate`'s Contract and the shape `land` matches for one message; the empty owner already says it, as `gate/paths-under-the-root` states.
- **Not taken: accept the message.** It names an empty component and a route (`Build this in ```) a session can't take.

## Outcome

- `land/build-holds-own-files` (modified): for a file `foreign_files` lists with an empty owner, `clm land` and `clm land --check-only` print `<file>: land/build-holds-own-files: outside the CLM root, which no build of the root's components may change. Leave the requirement unbuilt and add a question item that blocks it` in place of the `owned by` line. New scenario *a file outside the root*, with the root at `tool/`. Every sentence holding `SHALL` stays; the change adds one.

## Constraints

- Edge cases: none known beyond the subdirectory root, the only layout in which a file can be outside the root.
- Workarounds: none.

## Open questions

None.
