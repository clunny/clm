---
title: "A live change and an archived one with the same ID load as the archived one alone"
kind: bug
components: [model]
---

Step: drafting `sync/archive-complete`'s scenario "archive's place taken" in wave 3 of `backlog/a-passing-check-proves-less-than-the-spec-claims` (`20261121-wave-3-sync-by-the-rule`), which needed a live change whose archive place is already taken.

What happens: when `changes/x/change.md` and `changes/archive/x/change.md` both exist, `clm_model::load_files` (`crates/clm-model/src/lib.rs`) inserts the changes it loads into one map by ID, so the archived copy replaces the live one and the graph holds one change `x`, the archived one. No violation names the pair. `check.coverage`'s progress skips archived changes, so the live `x` is never counted, `clm rebase` never archives it, and nothing reports that it exists.

Expected: a live and an archived change that share an ID are a violation `clm check` reports, naming both paths, whichever the graph keeps; or the model states which it keeps and some command says the other is shadowed. Which component states the rule (`model` loading, or a `check` rule over duplicate IDs) is for the fix to settle.
