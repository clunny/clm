---
title: "CLAUDE.md edit plus its test classify mixed"
kind: bug
components: [gate, clm]
---

Requirement clm/claude-md-pushes-with-clm needs a CLAUDE.md edit and a test in one build, but gate/classify counts CLAUDE.md as a spec file, so the branch classifies mixed and cannot land. A build also cannot make a second worktree. Left unbuilt; CLAUDE.md should change on its own branch (small-change route), then the test can land.
