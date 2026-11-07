---
title: "This repository's land check marks no steps, so a failing land here names none"
kind: idea
components: [ci]
origin: 20261107-a-failed-land-check
---

## Intent

`20261107-a-failed-land-check` has a failing `clm land` name the step open when `[land] check` ended, when the check marks its steps with `::group::<name>` and `::endgroup::` lines (`check-step-markers`). `.github/scripts/ci-check.sh` runs fmt, clippy, the detector tools, the wasm builds, the tests and the `web`, panel and capability suites, and prints no markers, so this repository's own refusals still show only the 40-line tail. Have the script mark each of those steps.
