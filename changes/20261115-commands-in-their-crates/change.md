---
status: proposed
resolves: [backlog/clm-s-cli-tests-prove-other-components-through-the-binary]
---

## Intent

Phase 4 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`: fix the repository's shape. Each crate owns its commands, with their arguments, what they print and their exit status, and proves its requirements through that call in its own files; `crates/clm` only parses, finds the root, loads settings and routes.
