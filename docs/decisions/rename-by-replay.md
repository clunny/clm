---
title: "A rename is a recorded list of pairs that the gate replays, and only a term swap is reviewed"
date: 2026-11-01
status: accepted
---

Renaming what CLM owns (a component, a requirement, a word in spec text) is mechanical, but the edits reach test markers, which are code paths, and the human layers, which need review. Done as an ordinary spec branch, a component rename leaves its markers `obsolete` until a build re-points them (`moved-markers-wait`), and a reworded requirement leaves its tests `behind` until a build re-cites them (`hash-based-staleness`): a build session per component to edit marker strings.

So a change records its renames as `old → new` pairs in `change.md`'s `renames`, and `clm rename` produces everything else from them and the base: the deltas (ordinary removals, additions and modifications, so `clm apply` and the delta format don't change), the applied `specs/`, the reference edits and the marker rewrites. The gate replays the branch: when its tree is exactly what `clm rename` produces from the merge base, it is a `spec` branch though it touches code files (`rename/replay`, `gate/classify`), and `clm sync` regenerates it on `origin/main` instead of rebasing it.

**Review.** An ID rename changes no intent, so it lands on checks even where it edits `ARCHITECTURE.md` or `CAPABILITIES.md`: the replay proves those edits are the recorded renames. A term swap needs review: re-citing a reworded requirement skips the build that would confirm its tests still prove the new text, and the replay proves only that the edit was the swap, not that the meaning held (`SHALL` → `SHALL NOT` replays cleanly). The human's yes to the pair is that judgement (`humans-review-intent`).

**Scope.** A component rename changes only its ID's last segment, and a requirement rename only its slug: anything else is a move, which goes through deltas and a build. Code identifiers, commands and Contract items stay out (`expand-contract`); `clm rename` does their spec side. Archived changes and decisions keep the old names, as history. Mentions outside spec text (skills, guides, `CLAUDE.md`) are printed for the author, not rewritten, since editing them is a code change or prose.

Rejected:
- **A `## RENAMED` delta section that `clm apply` carries out.** New grammar for `model` and `apply`, both oversized, and the marker rewrites would still need a record the gate replays.
- **Editing `specs/` outside a change.** `specs/` reaches `main` only through `clm apply`, and the record would be a second source of truth.
- **Leaving markers to the builds.** A build session per component for string edits, and pending work in the queue that isn't real work.
- **Every rename landing on checks.** A term swap can change a rule's meaning while its tests stay cited as current; only a human can judge that.
- **A marker form that names the rename** (`req:a/y@v~a/x`). It keeps the old ID in the code forever and moves the judgement into every coverage read.

(2026-11-13: `hash-based-staleness` stated until now that the one re-cite that isn't a build is a rename's, which the gate replays: an ID rename changes no text a test proves, and a term swap is reviewed. It still holds; the entry was cut to its rule, `human-layer-entries-stay-short`.)
