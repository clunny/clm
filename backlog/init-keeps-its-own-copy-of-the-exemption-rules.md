---
title: "init keeps its own copy of the exemption rules, and nothing proves it agrees with the model's"
kind: bug
components: [init, model]
origin: 20261113-phase-2-delete
---

`clm survey --components` refuses an exemption entry the model would refuse (`init/survey-components`). `crates/clm-init/src/nodes.rs` checks the entries with rules of its own, written while `model/exemptions` was unbuilt (`workaround-init-survey-components-1`, deleted in `20261113-phase-2-delete`). The model's check is built now, but it is private to `clm-model`, and init's test feeds a hand-written list of bad entries and asserts only that the message names `exemptions`, so the two copies can drift.

Closing it means init judges an entry by the model's own check, which the model would have to offer in its Contract. That is a spec change to `model`.
