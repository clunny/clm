---
title: "Adoption is a skill plus a deterministic clm init"
date: 2026-10-22
status: accepted
---

A repository comes under CLM through `/clm:init`. The skill holds the judgment: a short onboarding conversation (what the project is about, the invariants and constraints to lock in, the stack), and drafting the first spec graph. `clm init` holds everything that's the same for every project, written from templates compiled into the binary: `clm.toml`, the CLM-owned `.github/workflows/clm.yml`, starter files where missing, and the labels. A given `clm` version always writes the same bytes, so tests pin the output (`deterministic-enforcement`).

- **Who owns what.** `clm.yml` is CLM's: re-running `clm init` after an upgrade regenerates it (`regenerate-dont-merge`). Everything else `init` writes is the project's from then on and is never touched again, including `.github/scripts/clm-build.sh`, which builds and tests the project. An upgrade therefore changes one code file and is a plain code PR, never `mixed`. The rules that must hold live in `clm` and the plugin, so an old `CLAUDE.md` doesn't weaken them.
- **Bootstrap.** CI can't gate the commit that installs CI, and the guard blocks Claude from pushing to `main` (`guard-blocks-push-to-main`). So `clm init`'s output is the one commit a human pushes to `main` directly. The first design then goes up as a HARD spec PR through the normal flow, so the team's first act is reviewing its own intent, with every check already on.
- **Private releases.** `clm` binaries are release assets of a private repository (`binary-from-releases`), which an adopter's workflow token can't read. The generated workflow downloads them with a `CLM_RELEASES_TOKEN` secret the human adds, and `clm init` says so.

Rejected:
- The skill alone writing every file from templates in its text: quick to build, but the output varies between runs, nothing deterministic tests it, and upgrading means hand-editing each adopter's workflows.
- A reusable workflow in this repository (`uses: clunny/clm/.github/workflows/clm.yml@vX`): one definition, but the repository is private, and only repositories with access to it may call its workflows.
