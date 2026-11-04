---
title: "The check script builds clm-web-replica for wasm32"
kind: idea
components: [ci]
---

`web.worker.replica/object-class` says `cargo build -p clm-web-replica --target wasm32-unknown-unknown` succeeds, but `.github/scripts/ci-check.sh` builds only `clm-web` for wasm32, and `clm-web` does not yet link `clm-web-replica`. Until `web.worker/replica-class-export` makes it do so, nothing in the land checks builds the `Replica` class. Add the line beside the others in the script's wasm section.
