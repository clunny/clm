---
title: "A renamed status, kind or key is written only by its new name, and its old name still loads"
date: 2026-11-02
status: accepted
---

`plain-terms-renames` renames words CLM writes into files: the node status `specd` (now `unbuilt`), the backlog kinds `human` and `friction` (now `human-task` and `workaround`) and the `change.md` key `burdens` (now `human_burdens`). Files written with the old words stay in git for good: archived changes keep the keys they were written with (`archive-immutable`), archived deltas keep their frontmatter, `effort`'s history parses backlog items out of old commits, and the gate loads a branch's merge base with the running model.

So the model reads each old word as its new one, on every tree, and keeps doing so; everything CLM writes or prints uses the new word only, and the model's term lists (`model/term-lists`), the glossary and the prompts name only the new words. A `change.md` that sets both the old and the new key is rejected, since the two would disagree. Requirement text names an old word only through this decision or `plain-terms-renames`, so `clm/retired-terms` can list it once the rename completes.

The requirement state `specd` is never written to a file (it is computed from the markers), so it is simply printed as `unbuilt` from then on. The command `clm friction` is a command, not a word in a file: it stays as a hidden alias until its contract change (`expand-contract`).

Rejected:
- **Rejecting the old words once CLM's own files are rewritten.** The archive and git history can't be rewritten, so the merge bases, the archived changes and `effort`'s history would stop loading.
- **Rewriting the archive.** `archive-immutable`, and history would no longer say what was written.
- **Accepting the old word only in the archive, as `gate` is.** `effort` and the gate read old trees outside the archive too, and a session holding a prompt from before the rename may still write the old word; reading it costs one alias per word.
