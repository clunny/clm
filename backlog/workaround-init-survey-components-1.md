---
title: "Exemption entries validated in init because model/exemptions is unbuilt"
kind: workaround
components: [init]
requirement: init/survey-components@368a5dcd
---

The model's exemption parser (model/exemptions) isn't built yet, so nodes.rs validates a map's exemption entries by its own copy of the rules (rule set, files, until, why, unknown keys). It should call the model's check once model builds it, and this copy then goes.
