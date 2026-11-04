---
title: "Hook logic lives in clm, not in hook scripts"
date: 2026-10-23
status: accepted
---

Each plugin hook is a one-line call to `clm hook <name>`, which reads the hook's JSON from stdin. The rules themselves are Rust, so they are unit-tested and mutation-checked like every other rule (`deterministic-enforcement`, `proven-by-tests`), and shell quirks across platforms don't matter. The plugin node owns this code as crate `clm-hooks`.

The `spec-approved` guard catches accidents, not an adversary: Claude shares the human's GitHub account, and a deliberately obfuscated command could still add the label. Its job is to make `humans-review-intent` hold against the mistakes that actually happen, such as a skill or a habit reaching for `--add-label`.
