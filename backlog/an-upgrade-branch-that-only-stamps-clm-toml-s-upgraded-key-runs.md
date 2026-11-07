---
title: "An upgrade branch that only stamps clm.toml's upgraded key runs the full land check"
kind: bug
issues: [262]
components: [ledger, upgrade]
origin: 20261107-slow-tests-after-land
---

Step: `clm land` of an upgrade branch made by `/clm:upgrade` whose migration is a note (docs only), in a project whose `[land] check` takes about 15 minutes.

What happened: the branch's only non-doc edit was the `upgraded` stamp `clm upgrade` writes into `clm.toml`. `clm.toml` is part of the code key, so the land found no `checked` record for the merged tree and ran the whole check command, though no code, test or check setting changed. Reported from another project as part of GitHub issue #262.

Expected: a land whose only code-key difference is the `upgraded` stamp (or comments in `clm.toml`) counts as already checked, as a spec-only land does.

Rule involved: `landed-record-names-its-code-key` / the ledger's code key, and `upgrade`'s stamp.
