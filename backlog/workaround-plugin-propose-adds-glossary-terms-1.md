---
title: "Special case: the glossary path in a skill"
kind: workaround
components: [plugin]
requirement: plugin/propose-adds-glossary-terms@fd6a2597
---

Two earlier tests (method-pointers, propose-nests-splits) forbid any docs/guide/ text in a skill, while propose-adds-glossary-terms requires the propose skill to name docs/guide/glossary.md. Both tests now exempt that one path.
