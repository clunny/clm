---
title: "Stamp this repository's clm.toml with upgraded once the upgrade build is installed"
kind: deferred
components: [config]
origin: 20261105-clm-upgrade
---

Once `upgrade` and `config/upgrade-settings` are built, landed and installed in this clone (`scripts/dev-install.sh`, a human step), run `clm upgrade` here. Then, on a branch of its own, run `clm upgrade --apply` and `clm upgrade --decline` for each advised note that doesn't fit this repository. `CLAUDE.md` here isn't `clm init`'s template, so the `/clm:help` migration will be pending until it is taken or declined. Land the result. Until then this repository has no `upgraded`, and every migration is judged by its `done` check alone, which is correct but runs them all.

Do this on its own branch and never in an `impl/*` branch: under `settings-before-use`, a clone running a `clm` older than the build rejects the `upgraded` key.
