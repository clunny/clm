---
title: "The scripts/clm-check.sh that clm init writes marks no steps"
kind: idea
components: [init]
origin: 20261107-a-failed-land-check
---

## Intent

`20261107-a-failed-land-check` has a failing `clm land` name the step open when `[land] check` ended, when the check marks its steps with `::group::<name>` and `::endgroup::` lines (`check-step-markers`). The starter `scripts/clm-check.sh` that `clm init` writes runs one line per `--lang` and prints no markers, so a new project learns of them only from `docs/guide/ci-and-deployment.md`. Have the starter mark each language's step. A project's existing script is its own (`init` writes the file only when missing), so whether `clm upgrade` should offer anything for it is part of the design (`upgrades-carry-migrations`).
