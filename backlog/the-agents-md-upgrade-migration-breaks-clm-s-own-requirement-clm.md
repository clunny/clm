---
title: "The agents-md upgrade migration breaks CLM's own requirement clm/claude-md-pushes-with-clm"
kind: bug
components: [clm, upgrade]
---

Step: `/clm:upgrade` in CLM's own repository; `clm upgrade --apply` took the advised migration `20261112-agents-md`, which moves CLAUDE.md's instructions to AGENTS.md and leaves CLAUDE.md as the single line `@AGENTS.md`.

What happened: the migration applied without error and `clm check` passed, but the repository's requirement `clm/claude-md-pushes-with-clm` (specs/SPEC.md) still says this repository's `CLAUDE.md` SHALL name `clm branch push`, and its scenario reads `CLAUDE.md`. After the migration that text lives in AGENTS.md, so `crates/clm/tests/claude_md.rs` fails ("CLAUDE.md lacks clm branch push") and the upgrade branch cannot land: its land check selects that test through its `clm:reads CLAUDE.md` marker.

Expected: the change that added the migration also updates CLM's own rules that name CLAUDE.md as the instruction file, so taking the migration here keeps every requirement true. The requirement should name AGENTS.md (or "CLAUDE.md or the AGENTS.md it imports"), and its test should read that file and carry `clm:reads AGENTS.md`; the other tests marked `clm:reads CLAUDE.md` (plugin.rs, plugin_build.rs) should be checked for the same assumption.

Rules involved: `upgrades-carry-migrations`, `dogfood-from-main`, `code-fix-needs-a-written-rule` (the fix changes a requirement's text, so it needs a spec branch before the upgrade can land).
