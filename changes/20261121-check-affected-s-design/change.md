---
status: proposed
resolves: [check-affected-s-design-names-select-without-the-files-it-could-not-read]
---

## Intent

`check.affected`'s Design gives `select` the signature it has: `select(graph, changed, markers, unread, all) -> Selection`, naming among its inputs the listed files the scan couldn't read as UTF-8, which the next sentences already say `select` takes for the exemption rule alone.

## Problem

The Design writes `select(graph, changed, markers, all) -> Selection`, while its own later sentence says the scan keeps the paths of the listed files it can't read as UTF-8 apart and `select` takes them for the `undeclared-imports` exemption rule alone. The build of `affected-files` at `72f3baff` gave `select` that list as a parameter, `unread`, so the signature the Design writes no longer exists and contradicts the Design's own text.

## Approaches

- **Taken: rewrite the signature and the list of inputs in the Design.** The code already matches the rest of the Design; only the stated signature is stale. No requirement changes, so nothing is made pending and no build is needed.
- **Not taken: drop the signature from the Design.** It is how a builder finds the pure core of the selection; naming it keeps the Design useful.

## Outcome

- `check.affected` Design: `select(graph, changed, markers, unread, all) -> Selection`, its inputs listing the paths of the listed files the scan couldn't read as UTF-8, and the exemption sentence naming them as `unread`. Nothing else changes.
- No code work: `crates/clm-affected/src/select.rs` already has this signature.

## Constraints

- No edge cases or workarounds known.
