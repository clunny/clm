---
title: "The review audit judges earlier lands by today's rule for review"
kind: bug
components: [ledger]
origin: 20261118-the-pass-s-tooling
---

`clm ledger verify --review` recomputes, for each signed land, whether it needed review, with the gate as it is now (`ledger/verify-recomputes-review`), and reports `ledger/understated-land` when the recomputed answer ranks above the recorded one. `20261118-the-pass-s-tooling` has a spec branch that cuts a requirement, a scenario or a sentence need review (`gate/cuts`). So in a signed repository every land made before that rule that removed one, and was rightly recorded `spec`, is reported as understated and unapproved once the rule is built: 26 of this repository's last 150 spec lands would be. The same held when widening an exemption came to need review.

This repository is unsigned, so nothing reports here. An adopter with `docs/allowed_signers` meets it on the release that carries the rule. It needs a spec change and should be fixed before that release: a land has to be audited by the rule in force when it landed, which the `landed` record doesn't name today.
