---
title: "clm-apply's archive comment says a live and archived pair loads as the archived one"
kind: deferred
components: [apply]
origin: 20261122-a-live-change-and
---

Condition: `model/unique-change-ids` (change `20261122-a-live-change-and`) is current.

The comment in `clm_apply::archive` (`crates/clm-apply/src/lib.rs`, around line 238: "a change both live and archived loads only as the archived one, which would read as no live change") explains the on-disk check of the archive place by the model's old behaviour. Once `model/unique-change-ids` is built, the model keeps the live copy and reports the pair, so the comment is false. Reword it to the reason the check still holds: an archive directory that holds no change makes no pair, and `archive` must refuse to move into it. The check itself stays; no requirement changes.
