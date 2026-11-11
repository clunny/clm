---
title: "Project tests that read CLAUDE.md or specs aren't told to carry a clm:reads marker"
kind: bug
components: [plugin.init, orchestrator.run]
---

A spec branch edited `CLAUDE.md`, and a project test that reads `CLAUDE.md` rejected the edit. The test had no `clm:reads CLAUDE.md` marker, so the spec land ran only `clm check` (`doc-readers-marked`), landed, and `main` then failed every code branch's land check until a build fixed the test outside its own component.

Lands work as designed here; what's missing is the instruction. The marker rule appears only in `docs/guide/ci-and-deployment.md`: neither the init skill nor the build prompt tells a session writing a test that reads `CLAUDE.md`, `ARCHITECTURE.md`, `specs/` or other doc-only paths to mark it, so a project's sessions don't.

Expected: the init skill (for tests it writes or recovers) and the build prompt state the rule where tests are written.
