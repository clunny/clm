---
title: "`resolves` entries stay bare slugs; `clm check` rejects any other shape"
date: 2026-10-26
status: accepted
---

A session that claimed items with `clm claim backlog/<slug>` wrote its change's `resolves` in the same form. `clm check`, `clm apply` and `clm sync` all passed it; only `clm gate` rejected it, late, and with a hint (`backlog/backlog/<slug>.md`) that compounded the mistake.

So `clm check` tests the shape of every live change's `resolves` and `from` entries (`check/backlog-entry-shape`): a `resolves` entry is a bare slug, and `from` is `backlog/<slug>`, the forms `docs/spec-format.md` already gives and every existing change uses. The message says what to write. The check runs wherever `clm check` runs, including `clm sync`, so the mistake surfaces at the first check. The gate gives no hint of its own: `check/backlog-entry-shape` has already said what to write.

The check tests shape, not existence. An item may be gone for good reasons: a `from` item is deleted in the spec PR that promotes it, and a `resolves` item is deleted by the build that fixes it (`task-deletes-fixed-items`), or by the same spec PR. Whether an entry ever named an item needs the merge base, which only the gate has, so `gate/resolves-named-items` keeps that.

Alternatives considered:
- Accept `backlog/<slug>` in `resolves` and strip it when parsing. Friendlier, but leaves two spellings in committed files, which every reader of `resolves` (the gate, `clm sync`'s archive, `priority-caps`, `web`) must then agree on, and archived changes keep whatever was written.
- Check existence in `clm check` for `proposed` and `approved` changes. Catches typos earlier too, but fails on a spec PR that deletes the item it resolves, which the gate allows.
- Only fix the gate's hint, and say in `propose` that entries are bare slugs. Prose doesn't enforce anything (`deterministic-enforcement`), and the mistake would still surface last.
