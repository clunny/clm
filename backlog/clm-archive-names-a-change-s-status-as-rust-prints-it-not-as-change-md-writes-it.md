---
title: "`clm archive` names a change's status as Rust prints it, not as `change.md` writes it"
kind: bug
components: [apply]
origin: 20261120-wave-2-rest-by-the-rule
---

`clm archive <id>` refuses a change that is neither `done` nor `abandoned` with ``change `<id>` is Applied; only done or abandoned changes can be archived``: the status is printed with Rust's debug form (`crates/clm-apply/src/lib.rs:249`, `{:?}`), so a person reads `Applied`, `Proposed` or `Implementing` where `change.md` holds `applied`, `proposed` or `implementing`.

`check/archive-done` had the same fault in its own message, and wave 2 of phase 5 mends it there: the requirement gives the line with the status as `change.md` writes it. `apply` had its pass in wave 1, so this is filed and not folded. Whether `apply/archive-command` states the refusal's words exactly was not read; if it gives the status in lower case, this is a code-only fix under `clm claim apply --fix`, and if it doesn't say, the requirement gains the line first.
