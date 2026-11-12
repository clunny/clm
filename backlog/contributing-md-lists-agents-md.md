---
title: "CONTRIBUTING.md's list of spec paths leaves out AGENTS.md"
kind: deferred
components: [clm]
---

`20261112-any-harness` makes `AGENTS.md` a spec path (`gate/agents-md-spec-path`), and this repository's `CONTRIBUTING.md` lists the spec paths a contribution may change: `specs/`, `changes/`, `backlog/`, `docs/`, `ARCHITECTURE.md`, `CAPABILITIES.md` and `CLAUDE.md`. Add `AGENTS.md` once `gate/agents-md-spec-path` is current; the file is the root component's code, so a spec branch can't carry the edit.
