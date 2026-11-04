---
title: "Nodes can own assets"
date: 2026-10-22
status: accepted
---

A node's spec can include files that aren't Markdown, such as UI mockups, in `specs/<node path>/assets/`. They are part of the node:
- the model hashes them like other elements;
- `clm context` lists them with their hashes, so a changed mockup makes the node's plans stale (`hash-based-staleness`) and the implementer knows to read them;
- `clm check` fails a spec that refers to a missing asset;
- changes add or modify them through `changes/<id>/assets/<node-id>/`, with base hashes, and remove them with a delta's `REMOVED Assets`, so they reach `specs/` only through `clm apply`.

Asset changes are `SOFT`: they shape what gets built, so a human sees them, but they don't change what other nodes rely on. Requirements win when an asset disagrees with them.

This came up because the `web` mockups made during design had no place in git, so a session building `web` in another context would never see them.

Alternatives considered:
- Mockups in the change directory, referenced from plan notes. They would move when the change is archived, and nothing would hash them.
- Files beside `SPEC.md` with no tool support. That works today, but nothing hashes or checks them, and the gate would treat every edit as `HARD`.
- Including asset contents in the context bundle. Bundles would bloat, and binary assets can't be included.
