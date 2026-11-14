---
title: "The panel type-checks against its own hand-written declarations of the plugin API"
date: 2026-11-03
status: accepted
---

The panel first type-checked against a vendored `index.d.ts`: Anthropic's file, copied as Claude Code wrote it and marked early access. This repository's content is being prepared for a public copy (`20261103-public-copy-prep`), and whether that file may be redistributed is unsettled, so it couldn't stay.

The panel uses a small part of the API: about ten hooks events, eleven `$` calls, `atom`, `read`, `update` and the JSX globals. So `plugin-panel/declarations/claude-code/index.d.ts` declares those, written by hand, and nothing else. A trial with every name typed `any` failed `tsc` with 54 implicit-`any` errors, so the file carries real signatures.

The cost is drift: `tsc` now checks the panel against what this repository believes the API is, not against what the engine writes. `claude plugin validate` still reads the module as the engine does and runs at every land where `claude` is on `PATH` (`panel-validate`), and when the two disagree the hand-written file is what's wrong. The check keeps its main job, catching a type used without its import or a slip in the panel's own code, on every checkout.

Alternatives considered:
- Keeping the real file out of git, copied into each checkout. A fresh worktree or a contributor has none, so the check would skip or fail where a type error is likeliest. Finding the engine's copy at check time was rejected earlier for the same reason.
- Dropping `tsc`. Type errors would land unnoticed again, the fault `tsc` was added to catch.
- Asking Anthropic for permission to redistribute the file. It would delay publishing with no certain answer. If a package ever ships the declarations, the panel can move to it by changing `tsconfig.json`'s type root.
