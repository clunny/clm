---
title: "Draft prompt has no route for a bug only an architecture entry states"
kind: bug
components: [orchestrator.triage]
origin: 20261108-web-suites-take-a
---

`clm draft`'s prompt (`crates/clm-triage/src/draft.rs`, `orchestrator.triage`) calls a bug code-only "only when a requirement or scenario as written already states the behaviour its fix restores, and the fix's test cites it, as `req:<component>/<slug>@<version>`". Some bugs restore behaviour that only an `ARCHITECTURE.md` entry states, and `human-layer-proof` says a test never cites an entry. Drafting `20261108-web-suites-take-a` hit this: `backlog/web-suites-take-a-preview-port-two-runs-can-share` asked `web/playwright.config.ts` to stop reusing a preview server another run started. Only `tests-run-concurrently` and its "a preview server" scenario state that behaviour, and the change touches only `web`'s own test-suite config, which no requirement of `web` covers (`web/test/preview-port.test.ts` and `web/test/playwright-config.test.ts` cite nothing).

Read literally, the prompt makes such a fix a spec change: a `web` requirement about how its own test runner treats ports. That is test plumbing a requirement shouldn't state. The prompt's harness-only route covers only the capability suite's plumbing. The draft treated the fix as code-only under `clm claim web --fix`, with an uncited test, and landed it as `8029e80`.

The prompt should say how to route a bug whose behaviour only a carried or project architecture entry states, or whose fix is plumbing in a component's own test suite: code-only by `--fix` with an uncited test, or something else.
