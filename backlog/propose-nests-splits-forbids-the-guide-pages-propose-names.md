---
title: "The plugin's propose-nests-splits says propose names no docs/guide/ path, while two other rules have it name two"
kind: question
components: [plugin]
---

`plugin/propose-nests-splits` ends "It SHALL NOT name `docs/guide/`", and its scenario says the skill doesn't name it. `plugin/method-pointers` allows `propose` to name `docs/guide/glossary.md` and `docs/guide/settings.md`, and `plugin/propose-adds-glossary-terms` and `plugin/propose-adds-settings-sections` require it to. The build followed the latter three, so the test for `propose-nests-splits` strips both paths before checking.

Needed: a spec change that words `propose-nests-splits` as "names no guide page by path except the two `method-pointers` allows", or drops the sentence.
