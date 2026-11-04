---
title: "code-file replay unobservable until rename/marker-rewrite"
kind: workaround
components: [gate]
requirement: gate/classify@2825a73f
---

The scenario 'rename branch' rewrites a marker in a test, but rename/marker-rewrite is still specd, so clm rename never changes a code file. The path where replays accepts a diff holding code files is built but cannot be exercised end to end; the tests prove the replayed spec-only branch, the human-layer and term review rules, and that an extra code edit stays mixed.
