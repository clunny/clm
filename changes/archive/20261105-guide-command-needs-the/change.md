---
status: done
resolves: [guide-command-needs-the-constraints-page]
---

# guide-command's byte-for-byte promise covers only pages with a file

## Intent

`init/guide-command` lists `constraints`, the generated page `init/guide-constraints-page` builds, as the last line of `clm guide`'s index, yet also promises that `clm guide <name>` prints "that page's Markdown exactly as its file is at build time", which the constraints page, having no file, can't meet. This change rewords `guide-command` so that promise covers only the pages read from a file, and says the `constraints` page prints as `guide-constraints-page` states. The index's full order stays in `guide-command`. It deletes the backlog question that blocked `guide-command`, which the human answered: `model/carried-constraints`, the work the page waits on, is already built, so `guide-command` needs no `#### Needs` for it.

## Problem

- `guide-command` says every page prints exactly as its file is, and also lists a page with no file.
- `backlog/guide-command-needs-the-constraints-page` blocks `guide-command`, so it stays `behind`, though `model/carried-constraints` and `init/guide-constraints-page` are both current now and a build can make its `index` scenario true.

## Approaches

- **A. Move the `constraints` line into `guide-constraints-page`.** The index's order would then be split across two requirements, one appending to the other's list.
- **B. Reword `guide-command`'s file promise and keep the order in it (chosen, the human's answer).** One requirement states the whole index; the byte-for-byte promise names the pages it covers.
- **C. Add only a `#### Needs` naming `model/carried-constraints`.** Leaves the contradiction, and the requirement it would name is already current.

## Outcome

`guide-command`: `clm guide <name>` prints a page read from a file (every page but `constraints`) exactly as its file is at build time; `clm guide constraints` prints the page `guide-constraints-page` states. Everything else, the index lines and their order, the unknown-page error and reading no file at run time, is unchanged. `init`'s Contract bullet for `clm guide` also names the generated `constraints` page beside the pages of `docs/guide/` and the spec format, which the architecture audit found missing (it predates this change). The build moves `guide-command`'s test markers to the new version and makes the index end with `constraints  CLM's constraints`.

Edge cases and workarounds known: none.

## Constraints

- `spec-is-current`: one requirement states the index's order.

## The human's answers

On `backlog/guide-command-needs-the-constraints-page`, offered option 1: "Drop the item and fix the wording. Delete the item, which unblocks both requirements, and reword `guide-command` so the 'exactly as its file is' promise covers only pages that have a file. The list's full order stays in one requirement." The human's answer, verbatim:

> go with option 1

## Open questions

None.
