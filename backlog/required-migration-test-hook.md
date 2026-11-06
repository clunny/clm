---
title: "A way to leave a required migration pending at the binary's boundary"
kind: question
components: [capability, upgrade]
---

`capability/upgrade-clm`'s second scenario needs a project with a pending `required` migration, but the shipped `clm` holds only advised migrations, so the capability test reaches the refusal through a too-new `upgraded` stamp instead, and does not check that the message points at `clm upgrade`. Either a required migration ships and the test uses it, or the spec names a way to supply a migration list at the boundary. Which should it be?
