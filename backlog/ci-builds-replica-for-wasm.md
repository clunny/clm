---
title: "Nothing builds clm-web-replica for wasm32 on its own"
kind: bug
components: [web.worker.replica]
---

`web.worker.replica/object-class`'s scenario "the class builds" runs `cargo build -p clm-web-replica --target wasm32-unknown-unknown`, but no test runs that build: `crates/clm-web-replica/tests/class.rs` reads the sources and leaves the build to `.github/scripts/ci-check.sh`, which builds only `clm-web` for wasm32. `clm-web` links `clm-web-replica` on wasm32 (`web.worker/replica-class-export`), so the class compiles there, but with the features `clm-web` turns on (such as `clm-web-live`'s `durable`), which can hide a feature the crate needs and doesn't declare. Every other crate with a wasm scenario proves it with a `tests/wasm.rs` that runs the standalone build; `clm-web-replica` should too.
