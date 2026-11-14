---
title: "Adoption is a skill plus a deterministic clm init"
date: 2026-10-22
status: accepted
---

A repository comes under CLM through `/clm:init`. The skill holds the judgment: a short onboarding conversation (what the project is about, the invariants and constraints to lock in, the stack), and drafting the first spec graph. `clm init` holds everything that's the same for every project, written from templates compiled into the binary: `clm.toml`, `scripts/clm-check.sh`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `specs/SPEC.md`, `AGENTS.md` and `CLAUDE.md`, each only where missing, and the `.gitignore` lines CLM needs (`init/init-files`, `init-ignores-session-files`). It writes nothing under `.github/`: no hosted CI runs checks (`land-locally`). A given `clm` version always writes the same bytes, so tests pin the output (`deterministic-enforcement`).

- **Who owns what.** Everything `clm init` writes is the project's from then on, including `scripts/clm-check.sh`, which builds and tests the project. Running `clm init` again changes no existing file; it only adds a `.gitignore` line that is missing (`init/init-rerun`). The rules that must hold live in `clm` and the plugin, so an old `CLAUDE.md` doesn't weaken them.
- **Upgrades.** `clm upgrade --apply` is what may touch a project's files after adoption: it runs migrations that rewrite exactly what CLM can rewrite exactly (a settings key, a `.gitignore` line), and replaces a file the project owns only while it still equals a template an earlier `clm init` wrote byte for byte. Any other edit to a file the project owns is the human's, proposed by `/clm:upgrade` (`upgrade-migrations`).
- **Bootstrap.** No land can check the commit that brings in the checks, and the guard blocks Claude from pushing to `main` (`guard-blocks-push-to-main`). So `clm init`'s output is the one commit a human pushes to `main` directly, and `clm init` ends by saying so, with the host's setup steps (`init/init-next-steps`). The first design then goes up as a spec branch through the normal flow, so the team's first act is reviewing its own intent, with every check already on.

Rejected:
- The skill alone writing every file from templates in its text: quick to build, but the output varies between runs, nothing deterministic tests it, and upgrading means hand-editing each adopter's files.
- A CI workflow that `clm init` generates and owns, regenerated on each upgrade: it went with hosted checks (`land-locally`), and with it the one file a rerun rewrote.
